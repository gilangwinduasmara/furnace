// Business logic for managing Furnace services (migrated from CLI)

use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use tracing::info;

use crate::{
    recipe,
    web_service::{NginxService, WebService},
};

pub fn stop() {
    println!("Stopping Furnace services...");
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine home directory");
            return;
        }
    };
    let nginx_dir = home.join(".furnace/nginx");
    let nginx_pid = nginx_dir.join("logs/nginx.pid");
    if nginx_pid.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&nginx_pid) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let _ = Command::new("kill")
                    .arg("-QUIT")
                    .arg(pid.to_string())
                    .status();
                println!("Sent QUIT to Nginx (PID {})", pid);
                sleep(Duration::from_secs(2));
            }
        }
    }
    let php_dir = home.join(".furnace/php");
    if let Ok(entries) = std::fs::read_dir(&php_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let fpm_pid = path.join("php-fpm.pid");
                if fpm_pid.exists() {
                    if let Ok(pid_str) = std::fs::read_to_string(&fpm_pid) {
                        if let Ok(pid) = pid_str.trim().parse::<i32>() {
                            let _ = Command::new("kill")
                                .arg("-QUIT")
                                .arg(pid.to_string())
                                .status();
                            println!("Sent QUIT to PHP-FPM (PID {})", pid);
                            sleep(Duration::from_millis(500));
                        }
                    }
                }
            }
        }
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let _ = Command::new("sudo")
            .arg("brew")
            .arg("services")
            .arg("stop")
            .arg("dnsmasq")
            .status();
        println!("Tried to stop dnsmasq via brew services");
    }
    println!("All Furnace services stopped.");
}

pub fn serve() {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let lsof = Command::new("lsof").arg("-i:80").output();
        if let Ok(output) = lsof {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if let Some(pid) = cols.get(1) {
                    if let Ok(pid_num) = pid.parse::<i32>() {
                        let _ = Command::new("kill").arg("-9").arg(pid).status();
                        eprintln!("Killed process {} using port 80", pid_num);
                    }
                }
            }
        }
    }
    info!("Starting services...");
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine home directory");
            return;
        }
    };

    let web_server = NginxService::new();
    let recipes = recipe::get_recipes();
    for recipe in recipes {
        if let Err(e) = web_server.write_conf(&recipe) {
            eprintln!(
                "Failed to write Nginx config for recipe {}: {}",
                recipe.name, e
            );
        } else {
            info!("Nginx config written for recipe {}", recipe.name);
        }
    }
    let php_dir = home.join(".furnace/php");
    if let Ok(entries) = std::fs::read_dir(&php_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let version = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let fpm_conf = path.join("furnace-php-fpm.conf");
                if fpm_conf.exists() {
                    let socket_path = path.join("php-fpm.sock");
                    let fpm_running = socket_path.exists();
                    if fpm_running {
                        info!(
                            "PHP-FPM for version {} already running (socket exists)",
                            version
                        );
                        continue;
                    }
                    let php_fpm_bin = if cfg!(target_os = "macos") {
                        format!("/opt/homebrew/opt/php@{}/sbin/php-fpm", version)
                    } else if cfg!(target_os = "linux") {
                        format!("/usr/sbin/php-fpm{}", version)
                    } else {
                        String::from("php-fpm")
                    };
                    let status = Command::new(&php_fpm_bin)
                        .arg("--nodaemonize")
                        .arg("--fpm-config")
                        .arg(&fpm_conf)
                        .spawn();
                    match status {
                        Ok(_) => info!("Started PHP-FPM for version {}", version),
                        Err(e) => eprintln!("Failed to start PHP-FPM for {}: {e}", version),
                    }
                }
            }
        }
    }
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine home directory");
            return;
        }
    };
    let nginx_dir = home.join(".furnace/nginx");
    let nginx_conf = nginx_dir.join("nginx.conf");
    let nginx_pid = nginx_dir.join("logs/nginx.pid");
    let status_output = Command::new("nginx")
        .arg("-p")
        .arg(&nginx_dir)
        .arg("-c")
        .arg("nginx.conf")
        .arg("-t")
        .output();
    match status_output {
        Ok(output) if output.status.success() => {
            info!("Nginx config test succeeded (using Furnace config)");
            let pid_valid = nginx_pid.exists()
                && std::fs::read_to_string(&nginx_pid)
                    .map(|s| s.trim().parse::<u32>().is_ok())
                    .unwrap_or(false);
            if pid_valid {
                let reload_output = Command::new("nginx")
                    .arg("-p")
                    .arg(&nginx_dir)
                    .arg("-c")
                    .arg("nginx.conf")
                    .arg("-s")
                    .arg("reload")
                    .output();
                match reload_output {
                    Ok(r) if r.status.success() => info!("Nginx reloaded (using Furnace config)"),
                    Ok(r) => {
                        let stderr = String::from_utf8_lossy(&r.stderr);
                        if stderr.contains("bind() to 0.0.0.0:80 failed")
                            || stderr.contains("Address already in use")
                        {
                            eprintln!(
                                "Nginx could not start or reload: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config."
                            );
                        } else {
                            eprintln!(
                                "Nginx reload failed with status: {}. Output: {}",
                                r.status, stderr
                            );
                        }
                        let start_output = Command::new("nginx")
                            .arg("-p")
                            .arg(&nginx_dir)
                            .arg("-c")
                            .arg("nginx.conf")
                            .output();
                        match start_output {
                            Ok(s) if s.status.success() => {
                                info!("Nginx started (using Furnace config)")
                            }
                            Ok(s) => {
                                let stderr = String::from_utf8_lossy(&s.stderr);
                                if stderr.contains("bind() to 0.0.0.0:80 failed")
                                    || stderr.contains("Address already in use")
                                {
                                    eprintln!(
                                        "Nginx could not start: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config."
                                    );
                                } else {
                                    eprintln!(
                                        "Nginx start failed with status: {}. Output: {}",
                                        s.status, stderr
                                    );
                                }
                            }
                            Err(e) => eprintln!("Failed to start nginx: {e}"),
                        }
                    }
                    Err(e) => eprintln!("Failed to reload nginx: {e}"),
                }
            } else {
                let start_output = Command::new("nginx")
                    .arg("-p")
                    .arg(&nginx_dir)
                    .arg("-c")
                    .arg("nginx.conf")
                    .output();
                match start_output {
                    Ok(s) if s.status.success() => info!("Nginx started (using Furnace config)"),
                    Ok(s) => {
                        let stderr = String::from_utf8_lossy(&s.stderr);
                        if stderr.contains("bind() to 0.0.0.0:80 failed")
                            || stderr.contains("Address already in use")
                        {
                            eprintln!(
                                "Nginx could not start: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config."
                            );
                        } else {
                            eprintln!(
                                "Nginx start failed with status: {}. Output: {}",
                                s.status, stderr
                            );
                        }
                    }
                    Err(e) => eprintln!("Failed to start nginx: {e}"),
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "Nginx config test failed with status: {}. Output: {}",
                output.status, stderr
            );
            if stderr.contains("bind() to 0.0.0.0:80 failed")
                || stderr.contains("Address already in use")
            {
                eprintln!(
                    "Nginx could not start: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config."
                );
            }
        }
        Err(e) => eprintln!("Failed to run nginx: {e}"),
    }
    let dnsmasq_check = Command::new("which").arg("dnsmasq").output();
    match dnsmasq_check {
        Ok(output) if output.status.success() => {
            info!(
                "dnsmasq is installed: {}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
            let dnsmasq_dir = home.join(".furnace/dnsmasq.d");
            if let Err(e) = std::fs::create_dir_all(&dnsmasq_dir) {
                eprintln!("Failed to create dnsmasq.d dir: {e}");
            }
            let conf_path = dnsmasq_dir.join("furnace-test.conf");
            let conf_content = "address=/.test/127.0.0.1\n";
            if let Err(e) = std::fs::write(&conf_path, conf_content) {
                eprintln!("Failed to write dnsmasq config: {e}");
            } else {
                info!(
                    "Wrote dnsmasq config for .test domains to {}",
                    conf_path.display()
                );
            }
            let reload = Command::new("sudo")
                .arg("brew")
                .arg("services")
                .arg("restart")
                .arg("dnsmasq")
                .status();
            match reload {
                Ok(r) if r.success() => info!("dnsmasq reloaded (for .test domains)"),
                Ok(r) => eprintln!("dnsmasq reload failed with status: {}", r),
                Err(e) => eprintln!("Failed to reload dnsmasq: {e}"),
            }
            println!(
                "If you haven't already, add 'conf-dir={}' to your /usr/local/etc/dnsmasq.conf and restart dnsmasq.",
                dnsmasq_dir.display()
            );
            println!("You may also need to run: sudo brew services restart dnsmasq");
            println!("And set your system DNS to 127.0.0.1");
            let main_conf_paths = [
                "/usr/local/etc/dnsmasq.conf",
                "/opt/homebrew/etc/dnsmasq.conf",
                "/etc/dnsmasq.conf",
            ];
            let conf_dir_line = format!("conf-dir={}", dnsmasq_dir.display());
            let mut updated = false;
            for path in &main_conf_paths {
                let conf_path = std::path::Path::new(path);
                if conf_path.exists() {
                    let content = std::fs::read_to_string(conf_path).unwrap_or_default();
                    let already_present = content
                        .lines()
                        .any(|l| l.trim_start().starts_with(&conf_dir_line));
                    if !already_present {
                        let new_content = format!("{}\n{}\n", content.trim_end(), conf_dir_line);
                        if let Err(e) = std::fs::write(conf_path, new_content) {
                            eprintln!("Failed to update {}: {e}", conf_path.display());
                        } else {
                            info!("Added '{}' to {}", conf_dir_line, conf_path.display());
                            updated = true;
                        }
                    }
                }
            }
            if !updated {
                println!(
                    "Please ensure 'conf-dir={}' is present in your dnsmasq.conf",
                    dnsmasq_dir.display()
                );
            }
        }
        _ => {
            eprintln!(
                "Warning: dnsmasq is not installed or not in PATH. .test domains will not resolve to 127.0.0.1"
            );
        }
    }
}

pub fn install() {
    info!("Installing services...");
    let nginx = NginxService::new();
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine home directory");
            return;
        }
    };
    let furnace_dir = home.join(".furnace");
    let repo_dst = furnace_dir.join("repository.yml");
    if let Err(e) = std::fs::create_dir_all(&furnace_dir) {
        eprintln!("Failed to create ~/.furnace directory: {e}");
        return;
    }
    let repo_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/repository.yml");
    match std::fs::copy(&repo_src, &repo_dst) {
        Ok(_) => info!("Copied default repository.yml to {}", repo_dst.display()),
        Err(e) => eprintln!("Failed to copy repository.yml: {e}"),
    }
    if NginxService::detect_installation() {
        info!("Nginx is installed");
    } else {
        eprintln!(
            "Warning: Nginx is not installed or not in PATH. Please install it before using 'furnace serve'."
        );
    }
}

pub fn status() {
    info!("Checking status...");
    // TODO: Check status of services
}

pub fn restart() {
    info!("Restarting Furnace services...");
    stop();
    serve();
}
