use std::fs;
use std::path::Path;

/// Validate and register (copy) nginx config for a project into the Furnace-managed servers dir.
/// Returns Ok(()) if successful, Err otherwise.
pub fn register_nginx_conf(project: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap();
    let conf_path = home.join(format!(".furnace/nginx/{}.conf", project));
    let furnace_nginx_dir = home.join(".furnace/nginx");
    let servers_dir = furnace_nginx_dir.join("servers");
    fs::create_dir_all(&servers_dir)?;
    let target_path = servers_dir.join(format!("{}.conf", project));
    // Copy (not symlink) for cross-platform simplicity
    fs::copy(&conf_path, &target_path)?;
    // Validate the main nginx.conf
    let main_conf = furnace_nginx_dir.join("nginx.conf");
    let output = std::process::Command::new("nginx")
        .arg("-t")
        .arg("-c")
        .arg(&main_conf)
        .output()?;
    if !output.status.success() {
        eprintln!("Nginx config test failed:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err("Nginx config validation failed".into());
    }
    // Reload nginx
    std::process::Command::new("nginx").arg("-s").arg("reload").status()?;
    println!("Nginx config registered and reloaded for project {}", project);
    Ok(())
}

/// Unregister (remove) a site's config from the Furnace-managed servers dir.
pub fn unregister_nginx_conf(project: &str) -> Result<(), Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap();
    let servers_dir = home.join(".furnace/nginx/servers");
    let target_path = servers_dir.join(format!("{}.conf", project));
    if target_path.exists() {
        fs::remove_file(&target_path)?;
    }
    // Validate and reload main config
    let main_conf = home.join(".furnace/nginx/nginx.conf");
    let output = std::process::Command::new("nginx")
        .arg("-t")
        .arg("-c")
        .arg(&main_conf)
        .output()?;
    if !output.status.success() {
        eprintln!("Nginx config test failed:\n{}", String::from_utf8_lossy(&output.stderr));
        return Err("Nginx config validation failed".into());
    }
    std::process::Command::new("nginx").arg("-s").arg("reload").status()?;
    println!("Nginx config unregistered and nginx reloaded for project {}", project);
    Ok(())
}
