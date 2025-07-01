use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::Local;
use tracing::{info, error};

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,
    pub path: String,
    pub php_version: String,
    pub serve_with: String,
    pub site: String,
}

pub fn is_laravel_project<P: AsRef<Path>>(dir: P) -> bool {
    let dir = dir.as_ref();
    dir.join("artisan").exists() && dir.join("composer.json").exists()
}

pub fn parse_php_version<P: AsRef<Path>>(composer_path: P) -> Option<String> {
    let content = fs::read_to_string(composer_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let req = json.get("require")?.get("php")?.as_str()?;
    Some(req.to_string())
}

// --- PHP version sanitization for cook_here ---
fn extract_major_minor(version: &str) -> String {
    // Extracts e.g. "^8.2" or ">=8.2.1" to "8.2"
    let digits: String = version.chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = digits.split('.');
    let major = parts.next().unwrap_or("");
    let minor = parts.next().unwrap_or("");
    if !major.is_empty() && !minor.is_empty() {
        format!("{}.{}", major, minor)
    } else {
        digits
    }
}

pub fn cook_here(name: Option<String>) {
    let cwd = std::env::current_dir().expect("Failed to get current dir");
    if !is_laravel_project(&cwd) {
        error!("Not a Laravel project");
        return;
    }
    let project_name = name.unwrap_or_else(|| cwd.file_name().unwrap().to_string_lossy().to_string());
    println!("cooking {}", project_name);
    
    let composer_path = cwd.join("composer.json");

    // Use the active PHP version from ~/.furnace.yml if available, else fallback to composer
    let home = dirs::home_dir().expect("Cannot find home directory");
    let furnace_yml = home.join(".furnace.yml");
    let active_php_version = if let Ok(furnace_yml_content) = fs::read_to_string(&furnace_yml) {
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&furnace_yml_content) {
            yaml.get("php_version").and_then(|v| v.as_str()).map(|s| extract_major_minor(s))
        } else { None }
    } else { None };
    let php_version = active_php_version.unwrap_or_else(|| parse_php_version(&composer_path).map(|s| extract_major_minor(&s)).unwrap_or_else(|| "unknown".to_string()));
    let site = format!("{}.test", project_name);


    // Check for existing recipe for this path
    let recipes_dir = home.join(".furnace/recipes");
    fs::create_dir_all(&recipes_dir).expect("Failed to create recipes dir");
    let cwd_str = cwd.to_string_lossy();
    let already_registered = std::fs::read_dir(&recipes_dir)
        .ok()
        .and_then(|entries| {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(existing) = serde_yaml::from_str::<Recipe>(&content) {
                        if existing.path == cwd_str {
                            error!("A recipe for this directory is already registered as '{}'.", existing.name);
                            return Some(());
                        }
                    }
                }
            }
            None
        })
        .is_some();
    let recipe_filename = format!("{}.yml", project_name);
    let recipe_path = recipes_dir.join(&recipe_filename);
    // Always (re)write the recipe file for this project (by path)
    let recipe = Recipe {
        name: project_name.clone(),
        path: cwd_str.to_string(),
        php_version: php_version.clone(),
        serve_with: "apache".to_string(),
        site: site.clone(),
    };
    let yml = serde_yaml::to_string(&recipe).expect("Failed to serialize recipe");
    // Always write the recipe file
    fs::write(&recipe_path, yml).expect("Failed to write recipe file");
    info!("Recipe created/updated at {}", recipe_path.display());
    println!("cooking {}", project_name);

    // Read values from the just-written recipe YAML (only if it was just written or updated)
    let recipe: Option<Recipe> = fs::read_to_string(&recipe_path)
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok());
    let (project_name, cwd_str, php_version, site) = if let Some(recipe) = recipe {
        (recipe.name, recipe.path, recipe.php_version, recipe.site)
    } else {
        // Fallback to values we just constructed
        (project_name.clone(), cwd_str.to_string(), php_version.clone(), site.clone())
    };

    // Generate Apache config for this project
    let apache_dir = home.join(".furnace/apache");
    fs::create_dir_all(&apache_dir).expect("Failed to create apache dir");
    let apache_conf_path = apache_dir.join(format!("{}.conf", project_name));
    let logs_dir = apache_dir.join("logs");
    fs::create_dir_all(&logs_dir).expect("Failed to create apache logs dir");
    let php_fpm_socket = format!("{}/.furnace/php/{}/php-fpm.sock", home.to_string_lossy(), php_version);
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
    fs::write(&apache_conf_path, apache_conf).expect("Failed to write apache conf");
    info!("Apache config created/updated at {}", apache_conf_path.display());

    // Generate Nginx config for this project
    let nginx_dir = home.join(".furnace/nginx");
    fs::create_dir_all(&nginx_dir).expect("Failed to create nginx dir");
    let nginx_logs_dir = nginx_dir.join("logs");
    fs::create_dir_all(&nginx_logs_dir).expect("Failed to create nginx logs dir");
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

    // Write nginx config directly to servers dir for activation
    let servers_dir = nginx_dir.join("servers");
    if let Err(e) = fs::create_dir_all(&servers_dir) {
        error!("Failed to create nginx servers dir: {e}");
    }
    let server_conf_path = servers_dir.join(format!("{}.conf", project_name));
    if let Err(e) = fs::write(&server_conf_path, &nginx_conf) {
        error!("Failed to write nginx conf to servers dir: {e}");
    } else {
        info!("Nginx config created/updated at {}", server_conf_path.display());
    }

    // Always create or update symlink in project dir
    let project_symlink = cwd.join(".furnace.recipe.yml");
    if project_symlink.exists() || project_symlink.is_symlink() {
        let _ = fs::remove_file(&project_symlink);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if let Err(e) = symlink(&recipe_path, &project_symlink) {
            error!("Failed to create symlink in project dir: {e}");
        } else {
            info!("Symlinked recipe to {}", project_symlink.display());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        if let Err(e) = symlink_file(&recipe_path, &project_symlink) {
            error!("Failed to create symlink in project dir: {e}");
        } else {
            info!("Symlinked recipe to {}", project_symlink.display());
        }
    }

    println!("{} is cooked at http://{}", project_name, site);
}

pub fn list_recipes() {
    let home = dirs::home_dir().expect("Cannot find home directory");
    let recipes_dir = home.join(".furnace/recipes");
    let mut table = prettytable::Table::new();
    table.add_row(prettytable::row!["Name", "Directory", "Site"]);
    if let Ok(entries) = fs::read_dir(&recipes_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(recipe) = serde_yaml::from_str::<Recipe>(&content) {
                    table.add_row(prettytable::row![recipe.name, recipe.path, recipe.site]);
                }
            }
        }
    }
    table.printstd();
}

/// Target for recipe disposal
pub enum RecipeDisposalTarget {
    ByName(String),
    ByCurrentDir,
}

/// Main disposal function
pub fn dispose_recipe(target: RecipeDisposalTarget) {
    use std::path::PathBuf;
    let home = dirs::home_dir().expect("Cannot find home directory");
    let recipes_dir = home.join(".furnace/recipes");
    let (project_name, recipe_path, nginx_conf_path, symlink_path): (String, PathBuf, PathBuf, Option<PathBuf>);

    match target {
        RecipeDisposalTarget::ByName(name) => {
            project_name = name.clone();
            recipe_path = recipes_dir.join(format!("{}.yml", &project_name));
            nginx_conf_path = home.join(".furnace/nginx/servers").join(format!("{}.conf", &project_name));
            symlink_path = None;
        },
        RecipeDisposalTarget::ByCurrentDir => {
            let cwd = std::env::current_dir().expect("Failed to get current dir");
            let symlink = cwd.join(".furnace.recipe.yml");
            if !symlink.exists() || !symlink.is_symlink() {
                error!("No recipe found in this directory.");
                return;
            }
            let target = match fs::read_link(&symlink) {
                Ok(path) => path,
                Err(e) => {
                    error!("Failed to read symlink: {}", e);
                    return;
                }
            };
            project_name = target.file_stem().unwrap().to_string_lossy().to_string();
            recipe_path = target;
            nginx_conf_path = home.join(".furnace/nginx/servers").join(format!("{}.conf", &project_name));
            symlink_path = Some(symlink);
        }
    }

    // Remove nginx config
    if nginx_conf_path.exists() {
        if let Err(e) = fs::remove_file(&nginx_conf_path) {
            error!("Failed to delete nginx config: {}", e);
        } else {
            info!("Deleted nginx config at {}", nginx_conf_path.display());
        }
    }
    // Remove recipe file
    if recipe_path.exists() {
        if let Err(e) = fs::remove_file(&recipe_path) {
            error!("Failed to delete recipe file: {}", e);
        } else {
            info!("Deleted recipe file at {}", recipe_path.display());
        }
    }
    // Remove symlink if present
    if let Some(symlink) = symlink_path {
        if symlink.exists() || symlink.is_symlink() {
            if let Err(e) = fs::remove_file(&symlink) {
                error!("Failed to delete symlink: {}", e);
            } else {
                info!("Deleted symlink at {}", symlink.display());
            }
        }
    }
    println!("Recipe for '{}' has been disposed.", project_name);
}

/// CLI entrypoint for disposal
pub fn dispose_recipe_cli(name: Option<String>) {
    match name {
        Some(n) => dispose_recipe(RecipeDisposalTarget::ByName(n)),
        None => dispose_recipe(RecipeDisposalTarget::ByCurrentDir),
    }
}