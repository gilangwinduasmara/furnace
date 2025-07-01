use std::process::Command;
use std::fs;
use std::path::Path;

/// Validates, symlinks, and reloads Apache config for a given project name.
pub fn register_apache_conf(project_name: &str) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Could not get home directory")?;
    let apache_conf_dir = home.join(".furnace/apache");
    let conf_path = apache_conf_dir.join(format!("{}.conf", project_name));
    let sites_available = Path::new("/etc/apache2/sites-available");
    let sites_enabled = Path::new("/etc/apache2/sites-enabled");
    if !conf_path.exists() {
        return Err(format!("Apache config {} does not exist", conf_path.display()));
    }
    // Symlink to sites-available
    let target_available = sites_available.join(format!("furnace-{}.conf", project_name));
    if target_available.exists() {
        let _ = fs::remove_file(&target_available);
    }
    if let Err(e) = std::os::unix::fs::symlink(&conf_path, &target_available) {
        return Err(format!("Failed to symlink to sites-available: {e}"));
    }
    // Symlink to sites-enabled
    let target_enabled = sites_enabled.join(format!("furnace-{}.conf", project_name));
    if target_enabled.exists() {
        let _ = fs::remove_file(&target_enabled);
    }
    if let Err(e) = std::os::unix::fs::symlink(&target_available, &target_enabled) {
        return Err(format!("Failed to symlink to sites-enabled: {e}"));
    }
    // Validate config
    let output = Command::new("apachectl").arg("configtest").output()
        .map_err(|e| format!("Failed to run apachectl: {e}"))?;
    if !output.status.success() {
        return Err(format!("Apache configtest failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    // Reload Apache
    let output = Command::new("sudo").arg("apachectl").arg("graceful").output()
        .map_err(|e| format!("Failed to reload apache: {e}"))?;
    if !output.status.success() {
        return Err(format!("Apache reload failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(())
}
