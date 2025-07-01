use crate::recipe::Recipe;
use std::process::Command;
use std::fs;
use std::path::PathBuf;

pub trait WebService {
    /// Returns true if the web service is installed on the system
    fn detect_installation() -> bool where Self: Sized;
    /// Start the web service
    fn start(&self) -> Result<(), String>;
    /// Stop the web service
    fn stop(&self) -> Result<(), String>;
    /// Write the config for a given project/recipe
    fn write_conf(&self, recipe: &Recipe) -> Result<(), String>;
    /// Reload the web service (if supported)
    fn reload(&self) -> Result<(), String> { Ok(()) }
}

pub struct NginxService {
    pub nginx_dir: PathBuf,
}

impl NginxService {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Cannot find home directory");
        let nginx_dir = home.join(".furnace/nginx");
        NginxService { nginx_dir }
    }
}

impl WebService for NginxService {
    fn detect_installation() -> bool {
        #[cfg(target_os = "windows")]
        let output = Command::new("where").arg("nginx.exe").output();
        #[cfg(not(target_os = "windows"))]
        let output = Command::new("which").arg("nginx").output();
        matches!(output, Ok(ref o) if o.status.success())
    }
    fn start(&self) -> Result<(), String> {
        let status = Command::new("nginx")
            .arg("-p").arg(&self.nginx_dir)
            .arg("-c").arg("nginx.conf")
            .status()
            .map_err(|e| format!("Failed to start nginx: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("nginx start failed with status: {}", status))
        }
    }
    fn stop(&self) -> Result<(), String> {
        let pid_path = self.nginx_dir.join("logs/nginx.pid");
        if !pid_path.exists() {
            return Ok(()); // Already stopped
        }
        let pid = fs::read_to_string(&pid_path)
            .map_err(|e| format!("Failed to read nginx pid: {e}"))?
            .trim().to_string();
        let status = Command::new("kill")
            .arg("-QUIT")
            .arg(&pid)
            .status()
            .map_err(|e| format!("Failed to stop nginx: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("nginx stop failed with status: {}", status))
        }
    }
    fn write_conf(&self, recipe: &Recipe) -> Result<(), String> {
        let servers_dir = self.nginx_dir.join("servers");
        fs::create_dir_all(&servers_dir)
            .map_err(|e| format!("Failed to create nginx servers dir: {e}"))?;
        let conf_path = servers_dir.join(format!("{}.conf", recipe.name));
        let logs_dir = self.nginx_dir.join("logs");
        fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create nginx logs dir: {e}"))?;
        let php_fpm_socket = format!("{}/.furnace/php/{}/php-fpm.sock", dirs::home_dir().unwrap().to_string_lossy(), recipe.php_version);
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
            site = recipe.site,
            project_path = recipe.path,
            logs_dir = logs_dir.to_string_lossy(),
            php_fpm_socket = php_fpm_socket,
            project = recipe.name
        );
        fs::write(&conf_path, nginx_conf)
            .map_err(|e| format!("Failed to write nginx conf: {e}"))?;
        Ok(())
    }
    fn reload(&self) -> Result<(), String> {
        let status = Command::new("nginx")
            .arg("-p").arg(&self.nginx_dir)
            .arg("-c").arg("nginx.conf")
            .arg("-s").arg("reload")
            .status()
            .map_err(|e| format!("Failed to reload nginx: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("nginx reload failed with status: {}", status))
        }
    }
}

pub struct ApacheService;

impl WebService for ApacheService {
    fn detect_installation() -> bool {
        #[cfg(target_os = "macos")]
        let output = std::process::Command::new("which").arg("httpd").output();
        #[cfg(target_os = "linux")]
        let output = std::process::Command::new("which").arg("apache2").output();
        #[cfg(target_os = "windows")]
        let output = std::process::Command::new("where").arg("httpd.exe").output();
        matches!(output, Ok(ref o) if o.status.success())
    }
    fn start(&self) -> Result<(), String> {
        // TODO: implement start logic
        Ok(())
    }
    fn stop(&self) -> Result<(), String> {
        // TODO: implement stop logic
        Ok(())
    }
    fn write_conf(&self, _recipe: &Recipe) -> Result<(), String> {
        // TODO: implement config writing logic
        Ok(())
    }
    fn reload(&self) -> Result<(), String> {
        // TODO: implement reload logic
        Ok(())
    }
}
