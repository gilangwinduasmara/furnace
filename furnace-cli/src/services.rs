pub fn stop() {
    use std::process::Command;
    use std::thread::sleep;
    use std::time::Duration;
    println!("Stopping Furnace services...");

    // 1. Stop Nginx (if running)
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
                let _ = Command::new("kill").arg("-QUIT").arg(pid.to_string()).status();
                println!("Sent QUIT to Nginx (PID {})", pid);
                sleep(Duration::from_secs(2));
            }
        }
    }

    // 2. Stop all PHP-FPM processes managed by Furnace
    let php_dir = home.join(".furnace/php");
    if let Ok(entries) = std::fs::read_dir(&php_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let fpm_pid = path.join("php-fpm.pid");
                if fpm_pid.exists() {
                    if let Ok(pid_str) = std::fs::read_to_string(&fpm_pid) {
                        if let Ok(pid) = pid_str.trim().parse::<i32>() {
                            let _ = Command::new("kill").arg("-QUIT").arg(pid.to_string()).status();
                            println!("Sent QUIT to PHP-FPM (PID {})", pid);
                            sleep(Duration::from_millis(500));
                        }
                    }
                }
            }
        }
    }

    // 3. Optionally stop dnsmasq (if managed by Furnace)
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let _ = Command::new("sudo").arg("brew").arg("services").arg("stop").arg("dnsmasq").status();
        println!("Tried to stop dnsmasq via brew services");
    }

    println!("All Furnace services stopped.");
}
use tracing::info;

pub fn serve() {
    // If you want to stop all services before starting, just call stop() here:
    // stop();
    // Force kill any process using port 80 before starting Nginx (macOS/Linux only)
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::process::Command;
        let lsof = Command::new("lsof").arg("-i:80").output();
        if let Ok(output) = lsof {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) { // skip header
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
    // Start PHP-FPM for all registered/supported versions
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine home directory");
            return;
        }
    };
    let php_dir = home.join(".furnace/php");
    if let Ok(entries) = std::fs::read_dir(&php_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let version = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let fpm_conf = path.join("furnace-php-fpm.conf");
                if fpm_conf.exists() {
                    // Check if PHP-FPM is already running for this version (by socket)
                    let socket_path = path.join("php-fpm.sock");
                    let fpm_running = socket_path.exists();
                    if fpm_running {
                        info!("PHP-FPM for version {} already running (socket exists)", version);
                        continue;
                    }
                    // Try to start PHP-FPM for this version
                    let php_fpm_bin = if cfg!(target_os = "macos") {
                        format!("/opt/homebrew/opt/php@{}/sbin/php-fpm", version)
                    } else if cfg!(target_os = "linux") {
                        format!("/usr/sbin/php-fpm{}", version)
                    } else {
                        // Windows or unknown
                        String::from("php-fpm")
                    };
                    let status = std::process::Command::new(&php_fpm_bin)
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
    // ...existing code...

    // Start Nginx with Furnace config
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
    let status_output = std::process::Command::new("nginx")
        .arg("-p")
        .arg(&nginx_dir)
        .arg("-c")
        .arg("nginx.conf")
        .arg("-t")
        .output();
    match status_output {
        Ok(output) if output.status.success() => {
            info!("Nginx config test succeeded (using Furnace config)");
            // Check if PID file exists and is valid
            let pid_valid = nginx_pid.exists() && std::fs::read_to_string(&nginx_pid).map(|s| s.trim().parse::<u32>().is_ok()).unwrap_or(false);
            if pid_valid {
                // Reload if PID is valid
                let reload_output = std::process::Command::new("nginx")
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
                        if stderr.contains("bind() to 0.0.0.0:80 failed") || stderr.contains("Address already in use") {
                            eprintln!("Nginx could not start or reload: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config.");
                        } else {
                            eprintln!("Nginx reload failed with status: {}. Output: {}", r.status, stderr);
                        }
                        // Try to start Nginx if reload fails
                        let start_output = std::process::Command::new("nginx")
                            .arg("-p")
                            .arg(&nginx_dir)
                            .arg("-c")
                            .arg("nginx.conf")
                            .output();
                        match start_output {
                            Ok(s) if s.status.success() => info!("Nginx started (using Furnace config)"),
                            Ok(s) => {
                                let stderr = String::from_utf8_lossy(&s.stderr);
                                if stderr.contains("bind() to 0.0.0.0:80 failed") || stderr.contains("Address already in use") {
                                    eprintln!("Nginx could not start: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config.");
                                } else {
                                    eprintln!("Nginx start failed with status: {}. Output: {}", s.status, stderr);
                                }
                            },
                            Err(e) => eprintln!("Failed to start nginx: {e}"),
                        }
                    },
                    Err(e) => eprintln!("Failed to reload nginx: {e}"),
                }
            } else {
                // Start Nginx if PID is missing/invalid
                let start_output = std::process::Command::new("nginx")
                    .arg("-p")
                    .arg(&nginx_dir)
                    .arg("-c")
                    .arg("nginx.conf")
                    .output();
                match start_output {
                    Ok(s) if s.status.success() => info!("Nginx started (using Furnace config)"),
                    Ok(s) => {
                        let stderr = String::from_utf8_lossy(&s.stderr);
                        if stderr.contains("bind() to 0.0.0.0:80 failed") || stderr.contains("Address already in use") {
                            eprintln!("Nginx could not start: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config.");
                        } else {
                            eprintln!("Nginx start failed with status: {}. Output: {}", s.status, stderr);
                        }
                    },
                    Err(e) => eprintln!("Failed to start nginx: {e}"),
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Nginx config test failed with status: {}. Output: {}", output.status, stderr);
            if stderr.contains("bind() to 0.0.0.0:80 failed") || stderr.contains("Address already in use") {
                eprintln!("Nginx could not start: Port 80 is already in use. Please stop any other web server (like Apache or another Nginx) or change the port in your Furnace config.");
            }
        }
        Err(e) => eprintln!("Failed to run nginx: {e}"),
    }
    // Check for dnsmasq and configure .test TLD
    let dnsmasq_check = std::process::Command::new("which").arg("dnsmasq").output();
    match dnsmasq_check {
        Ok(output) if output.status.success() => {
            info!("dnsmasq is installed: {}", String::from_utf8_lossy(&output.stdout).trim());
            // Write a config file for .test to ~/.furnace/dnsmasq.d/furnace-test.conf
            let dnsmasq_dir = home.join(".furnace/dnsmasq.d");
            if let Err(e) = std::fs::create_dir_all(&dnsmasq_dir) {
                eprintln!("Failed to create dnsmasq.d dir: {e}");
            }
            let conf_path = dnsmasq_dir.join("furnace-test.conf");
            let conf_content = "address=/.test/127.0.0.1\n";
            if let Err(e) = std::fs::write(&conf_path, conf_content) {
                eprintln!("Failed to write dnsmasq config: {e}");
            } else {
                info!("Wrote dnsmasq config for .test domains to {}", conf_path.display());
            }
            // Try to reload dnsmasq (may require sudo)
            let reload = std::process::Command::new("sudo")
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
            println!("If you haven't already, add 'conf-dir={}' to your /usr/local/etc/dnsmasq.conf and restart dnsmasq.", dnsmasq_dir.display());
            println!("You may also need to run: sudo brew services restart dnsmasq");
            println!("And set your system DNS to 127.0.0.1");
            // Try to add conf-dir to main dnsmasq.conf if not present
            let main_conf_paths = [
                "/usr/local/etc/dnsmasq.conf",
                "/opt/homebrew/etc/dnsmasq.conf",
                "/etc/dnsmasq.conf"
            ];
            let conf_dir_line = format!("conf-dir={}", dnsmasq_dir.display());
            let mut updated = false;
            for path in &main_conf_paths {
                let conf_path = std::path::Path::new(path);
                if conf_path.exists() {
                    let content = std::fs::read_to_string(conf_path).unwrap_or_default();
                    // Only add conf-dir line if not present and not commented out anywhere
                    let already_present = content.lines().any(|l| l.trim_start().starts_with(&conf_dir_line));
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
                println!("Please ensure 'conf-dir={}' is present in your dnsmasq.conf", dnsmasq_dir.display());
            }
        }
        _ => {
            eprintln!("Warning: dnsmasq is not installed or not in PATH. .test domains will not resolve to 127.0.0.1");
        }
    }
}

pub fn install() {
    info!("Installing services...");
    // Copy repository.yml to ~/.furnace/repository.yml if it doesn't exist
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
    // Validate if apache2/httpd is installed
    #[cfg(target_os = "macos")]
    let apache_check = std::process::Command::new("which").arg("httpd").output();
    #[cfg(target_os = "linux")]
    let apache_check = std::process::Command::new("which").arg("apache2").output();
    #[cfg(target_os = "windows")]
    let apache_check = std::process::Command::new("where").arg("httpd.exe").output();

    match apache_check {
        Ok(output) if output.status.success() => {
            info!("Apache is installed: {}", String::from_utf8_lossy(&output.stdout).trim());
        }
        _ => {
            eprintln!("Warning: Apache (httpd/apache2) is not installed or not in PATH. Please install it before using 'furnace serve'.");
        }
    }
    // Validate if nginx is installed
    #[cfg(target_os = "macos")]
    let nginx_check = std::process::Command::new("which").arg("nginx").output();
    #[cfg(target_os = "linux")]
    let nginx_check = std::process::Command::new("which").arg("nginx").output();
    #[cfg(target_os = "windows")]
    let nginx_check = std::process::Command::new("where").arg("nginx.exe").output();

    match nginx_check {
        Ok(output) if output.status.success() => {
            info!("Nginx is installed: {}", String::from_utf8_lossy(&output.stdout).trim());
        }
        _ => {
            eprintln!("Warning: Nginx is not installed or not in PATH. Please install it before using 'furnace serve'.");
        }
    }
    // Write main nginx.conf if not exists
    let nginx_dir = furnace_dir.join("nginx");
    let nginx_conf = nginx_dir.join("nginx.conf");
    let servers_dir = nginx_dir.join("servers");
    if let Err(e) = std::fs::create_dir_all(&servers_dir) {
        eprintln!("Failed to create nginx servers dir: {e}");
    }
    if !nginx_conf.exists() {
        let main_conf = r#"# Main Furnace-managed nginx.conf
# This file is generated and managed by Furnace.

worker_processes  1;

events {
    worker_connections  1024;
}

http {
    include /opt/homebrew/etc/nginx/mime.types;
    default_type  application/octet-stream;
    sendfile        on;
    keepalive_timeout  65;

    # Furnace-managed sites
    include servers/*.conf;
}
"#;
        if let Err(e) = std::fs::write(&nginx_conf, main_conf) {
            eprintln!("Failed to write nginx.conf: {e}");
        } else {
            info!("Wrote main nginx.conf to {}", nginx_conf.display());
        }
    }
    // TODO: Install other dependencies
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