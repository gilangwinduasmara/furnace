use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use dirs;
use std::io::{self, Cursor, Write, Read};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub php: std::collections::HashMap<String, PlatformUrls>,
}

#[derive(Debug, Deserialize)]
pub struct PlatformUrls {
    pub linux: Option<PhpSource>,
    pub windows: Option<PhpSource>,
    pub macos: Option<PhpSource>,
}

#[derive(Debug, Deserialize)]
pub struct PhpSource {
    pub url: Option<String>,
    pub command: Option<String>,
    #[serde(rename = "type")]
    pub archive_type: Option<String>,
}

pub fn load_repository() -> Result<Repository, Box<dyn std::error::Error>> {
    let repo_path = dirs::home_dir().unwrap().join(".furnace/repository.yml");
    let content = fs::read_to_string(repo_path)?;
    Ok(serde_yaml::from_str(&content)?)
}

pub fn detect_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

pub fn php_install(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Preparing to install PHP version {}...", version);
    let repo = load_repository()?;
    let platform = detect_platform();
    let source = repo.php.get(version)
        .and_then(|p| match platform {
            "windows" => p.windows.as_ref(),
            "macos" => p.macos.as_ref(),
            _ => p.linux.as_ref(),
        })
        .ok_or("Version/platform not found in repository")?;

    if let Some(url) = &source.url {
        println!("Downloading PHP from {}", url);
        let response = reqwest::blocking::get(url)?;
        let total_size = response.content_length().unwrap_or(0);
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})").unwrap()
            .progress_chars("#>-")
        );
        let mut content = Vec::with_capacity(total_size as usize);
        let mut source_resp = response;
        let mut buffer = [0u8; 8192];
        let mut downloaded = 0u64;
        loop {
            let n = source_resp.read(&mut buffer)?;
            if n == 0 { break; }
            content.extend_from_slice(&buffer[..n]);
            downloaded += n as u64;
            pb.set_position(downloaded);
        }
        pb.finish_with_message("Download complete.");
        let php_dir = dirs::home_dir().unwrap().join(format!(".furnace/php/{}", version));
        fs::create_dir_all(&php_dir)?;
        println!("Extracting PHP archive...");
        match source.archive_type.as_deref() {
            Some("zip") => {
                let reader = Cursor::new(&content);
                let mut zip = zip::ZipArchive::new(reader)?;
                zip.extract(&php_dir)?;
            }
            Some("tar.gz") => {
                let tar = flate2::read::GzDecoder::new(Cursor::new(&content));
                let mut archive = tar::Archive::new(tar);
                archive.unpack(&php_dir)?;
            }
            Some(other) => return Err(format!("Unknown archive type: {}", other).into()),
            None => return Err("Missing archive type for url-based PHP source".into()),
        }
        println!("Extraction complete.");
        println!("Verifying PHP binaries...");
        let php_bin = if platform == "windows" {
            php_dir.join("php.exe")
        } else {
            php_dir.join("bin/php")
        };
        if !php_bin.exists() {
            return Err("php binary not found after extraction".into());
        }
        if platform != "windows" && !php_dir.join("sbin/php-fpm").exists() {
            return Err("php-fpm binary not found after extraction".into());
        }
        println!("PHP {} installed at {}", version, php_dir.display());
        Ok(())
    } else if let Some(cmd) = &source.command {
        println!("Running install command: {}", cmd);
        let mut parts = cmd.split_whitespace();
        let program = parts.next().ok_or("Invalid command")?;
        let args: Vec<&str> = parts.collect();
        let status = std::process::Command::new(program)
            .args(&args)
            .spawn()?
            .wait()?;
        if status.success() {
            println!("PHP {} installed via command.", version);
            if detect_platform() == "macos" && cmd.contains("brew install") {
                let home = dirs::home_dir().unwrap();
                let furnace_php_dir = home.join(format!(".furnace/php/{}", version));
                if let Some(parent) = furnace_php_dir.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("Failed to create parent directory for symlink: {e}");
                    }
                }
                let brew_prefix_output = std::process::Command::new("brew")
                    .arg("--prefix")
                    .arg(format!("php@{}", version.split('.').take(2).collect::<Vec<_>>().join(".")))
                    .output();
                if let Ok(output) = brew_prefix_output {
                    if output.status.success() {
                        let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let php_path = std::path::Path::new(&prefix);
                        if php_path.exists() {
                            let _ = std::fs::remove_file(&furnace_php_dir);
                            let _ = std::fs::remove_dir_all(&furnace_php_dir);
                            if let Err(e) = std::os::unix::fs::symlink(php_path, &furnace_php_dir) {
                                eprintln!("Failed to create symlink: {e}");
                            } else {
                                println!("Symlinked {} to {}", php_path.display(), furnace_php_dir.display());
                            }
                        } else {
                            eprintln!("brew prefix path does not exist: {}", php_path.display());
                        }
                    } else {
                        eprintln!("brew --prefix failed: {}", String::from_utf8_lossy(&output.stderr));
                    }
                } else {
                    eprintln!("Failed to run brew --prefix");
                }
            }
            Ok(())
        } else {
            Err(format!("Command failed with status: {}", status).into())
        }
    } else {
        Err("No url or command found for this platform/version".into())
    }
}

pub fn php_list() {
    let base = dirs::home_dir().unwrap().join(".furnace/php");
    if let Ok(entries) = fs::read_dir(base) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                println!("{}", entry.file_name().to_string_lossy());
            }
        }
    }
}

pub fn php_use(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap();
    let php_dir = home.join(format!(".furnace/php/{}", version));
    if !php_dir.exists() {
        return Err(format!("PHP version {} is not installed (expected at {})", version, php_dir.display()).into());
    }
    let cwd = std::env::current_dir()?;
    let config_path = cwd.join(".furnace.yml");
    let mut config: serde_yaml::Value = if config_path.exists() {
        serde_yaml::from_str(&fs::read_to_string(&config_path)?)?
    } else {
        serde_yaml::Value::Mapping(Default::default())
    };
    config["php_version"] = serde_yaml::Value::String(version.to_string());
    fs::write(config_path, serde_yaml::to_string(&config)?)?;
    println!("Set PHP version {} for project", version);
    let recipe_path = cwd.join(".furnace.recipe.yml");
    if recipe_path.exists() {
        let recipe: Result<serde_yaml::Value, _> = serde_yaml::from_str(&fs::read_to_string(&recipe_path)?);
        if let Ok(mut recipe_yaml) = recipe {
            recipe_yaml["php_version"] = serde_yaml::Value::String(version.to_string());
            fs::write(&recipe_path, serde_yaml::to_string(&recipe_yaml)?)?;
            let home = dirs::home_dir().unwrap();
            let project_name = recipe_yaml["name"].as_str().unwrap_or("project");
            let cwd_str = recipe_yaml["path"].as_str().unwrap_or("");
            let php_version_raw = recipe_yaml["php_version"].as_str().unwrap_or(version);
            let php_version = sanitize_php_version(php_version_raw);
            let site = recipe_yaml["site"].as_str().unwrap_or("localhost");
            let apache_dir = home.join(".furnace/apache");
            fs::create_dir_all(&apache_dir).ok();
            let apache_conf_path = apache_dir.join(format!("{}.conf", project_name));
            let logs_dir = apache_dir.join("logs");
            fs::create_dir_all(&logs_dir).ok();
            let php_fpm_socket = php_dir.join("php-fpm.sock").to_string_lossy().to_string();
            let apache_conf = format!(r#"
<VirtualHost *:80>
    ServerName {site}
    DocumentRoot "{project_path}/public"

    <Directory "{project_path}/public">
        AllowOverride All
        Require all granted
    </Directory>

    <FilesMatch \.php$>
        SetHandler "proxy:unix:{php_fpm_socket}|fcgi://localhost/"
    </FilesMatch>

    ErrorLog "{logs_dir}/{project}.error.log"
    CustomLog "{logs_dir}/{project}.access.log" combined
</VirtualHost>
"#,
                site=site,
                project_path=cwd_str,
                php_fpm_socket=php_fpm_socket,
                logs_dir=logs_dir.to_string_lossy(),
                project=project_name
            );
            fs::write(&apache_conf_path, apache_conf).ok();
            let nginx_dir = home.join(".furnace/nginx");
            fs::create_dir_all(&nginx_dir).ok();
            let nginx_conf_path = nginx_dir.join(format!("{}.conf", project_name));
            let nginx_logs_dir = nginx_dir.join("logs");
            fs::create_dir_all(&nginx_logs_dir).ok();
            let nginx_conf = format!(r#"
server {{
    listen 80;
    server_name {site};
    root {project_path}/public;

    index index.php index.html;

    access_log {logs_dir}/{project}.access.log;
    error_log {logs_dir}/{project}.error.log;

    location / {{
        try_files $uri $uri/ /index.php?$query_string;
    }}

    location ~ \.php$ {{
        include /opt/homebrew/etc/nginx/fastcgi_params;
        fastcgi_pass unix:{php_fpm_socket};
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        fastcgi_index index.php;
    }}
}}
"#,
                site=site,
                project_path=cwd_str,
                logs_dir=nginx_logs_dir.to_string_lossy(),
                php_fpm_socket=php_fpm_socket,
                project=project_name
            );
            fs::write(&nginx_conf_path, &nginx_conf).ok();
            let servers_dir = nginx_dir.join("servers");
            if let Err(e) = fs::create_dir_all(&servers_dir) {
                eprintln!("Failed to create nginx servers dir: {e}");
            }
            let server_conf_path = servers_dir.join(format!("{}.conf", project_name));
            if let Err(e) = fs::write(&server_conf_path, &nginx_conf) {
                eprintln!("Failed to write nginx conf to servers dir: {e}");
            }
            println!("Updated Apache and Nginx config for project {}", project_name);
            if let Some(project_name) = recipe_yaml["name"].as_str() {
                if let Err(e) = crate::nginx_util::register_nginx_conf(project_name) {
                    eprintln!("Failed to register nginx config: {e}");
                }
            }
            let _ = std::process::Command::new("nginx")
                .arg("-p").arg(&nginx_dir)
                .arg("-c").arg("nginx.conf")
                .arg("-s").arg("reload")
                .status();
        }
    }
    if let Err(e) = php_fpm_conf(version) {
        eprintln!("Failed to start PHP-FPM: {e}");
    }
    Ok(())
}

fn sanitize_php_version(version: &str) -> String {
    version.chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect()
}

pub fn php_fpm_conf(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo = load_repository()?;
    let platform = detect_platform();
    let source = repo.php.get(version)
        .and_then(|p| match platform {
            "windows" => p.windows.as_ref(),
            "macos" => p.macos.as_ref(),
            _ => p.linux.as_ref(),
        })
        .ok_or("Version/platform not found in repository")?;
    let php_dir = dirs::home_dir().unwrap().join(format!(".furnace/php/{}", version));
    let user = whoami::username();
    let group = if cfg!(target_os = "macos") { "staff".to_string() } else { user.clone() };
    let php_fpm_conf_path = php_dir.join("furnace-php-fpm.conf");
    let sock_path = php_dir.join("php-fpm.sock");
    let tpl_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/php-fpm-template/php-fpm.conf.tpl");
    let tpl = fs::read_to_string(&tpl_path).expect("php-fpm.conf.tpl missing");
    let conf = tpl.replace("{php_dir}", &php_dir.to_string_lossy())
        .replace("{user}", &user)
        .replace("{group}", &group)
        .replace("{sock_path}", &sock_path.to_string_lossy());
    fs::write(&php_fpm_conf_path, conf)?;
    println!("Generated custom furnace-php-fpm.conf at {}", php_fpm_conf_path.display());
    Ok(())
}
