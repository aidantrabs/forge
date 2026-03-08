use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize, Serialize)]
pub struct SiteConfig {
    pub title: String,
    pub description: String,
    pub base_url: String,
    pub author: String,
}

impl SiteConfig {
    pub fn load(path: &Path) -> Self {
        let raw = fs::read_to_string(path).expect("failed to read forge.toml");
        toml::from_str(&raw).expect("failed to parse forge.toml")
    }
}
