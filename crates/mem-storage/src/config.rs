use mem_domain::VaultConfig;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse Error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),
}

pub fn load_config(vault_path: &Path) -> Result<VaultConfig, ConfigError> {
    let config_path = vault_path.join("mem.json");
    if !config_path.exists() {
        return Err(ConfigError::NotFound(config_path));
    }
    
    let content = fs::read_to_string(&config_path)?;
    let config: VaultConfig = serde_json::from_str(&content)?;
    Ok(config)
}

pub fn save_config(vault_path: &Path, config: &VaultConfig) -> Result<(), ConfigError> {
    let config_path = vault_path.join("mem.json");
    let content = serde_json::to_string_pretty(config)?;
    fs::write(config_path, content)?;
    Ok(())
}
