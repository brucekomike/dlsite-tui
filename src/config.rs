use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub username: Option<String>,
}

pub fn config_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "dlsite", "dlsite-tui") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

pub fn load_config() -> Result<Config> {
    let path = config_dir().join("config.json");
    if path.exists() {
        let data = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    } else {
        Ok(Config::default())
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join("config.json");
    fs::write(&path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

pub fn cookies_path() -> PathBuf {
    config_dir().join("cookies.json")
}
