use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct FurnaceConfig {
    // Add fields as needed
}

pub fn load_config() -> Option<FurnaceConfig> {
    let content = fs::read_to_string(".furnace.yml").ok()?;
    serde_yaml::from_str(&content).ok()
}
