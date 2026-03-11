use std::fs;
use std::path::Path;
use thiserror::Error;
use mem_domain::VaultConfig;
use crate::config;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config Error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Vault already initialized at {0}")]
    AlreadyInitialized(String),
}

/// Initializes a new vault in the specified directory
pub fn init_vault(path: &Path, name: Option<String>) -> Result<VaultConfig, VaultError> {
    if path.join("mem.json").exists() {
        return Err(VaultError::AlreadyInitialized(path.to_string_lossy().to_string()));
    }

    // Create the base directory if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    let mut cfg = VaultConfig::default();
    if let Some(n) = name {
        cfg.vault_name = n;
    }

    // Create required subdirectories
    let notes_dir = path.join(&cfg.notes_dir);
    fs::create_dir_all(&notes_dir)?;
    
    // .mem internal directory
    let dot_mem = path.join(".mem");
    fs::create_dir_all(&dot_mem)?;
    
    // Default subfolders as a nice touch
    fs::create_dir_all(notes_dir.join("inbox"))?;
    fs::create_dir_all(notes_dir.join("journal"))?;

    // Save config
    config::save_config(path, &cfg)?;

    Ok(cfg)
}

/// Checks if a vault layout is valid
pub fn is_valid_vault(path: &Path) -> bool {
    path.join("mem.json").exists() &&
    path.join("notes").is_dir() &&
    path.join(".mem").is_dir()
}
