use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Storage error: {0}")]
    Storage(#[from] mem_storage::storage::StorageError),
    #[error("Vault error: {0}")]
    Vault(#[from] mem_storage::vault::VaultError),
    #[error("Config error: {0}")]
    Config(#[from] mem_storage::config::ConfigError),
    #[error("Index error: {0}")]
    Index(#[from] mem_index::db::IndexError),
    #[error("Sync error: {0}")]
    Sync(#[from] mem_sync::SyncError),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Note not found: {0}")]
    NoteNotFound(String),
    #[error("Not a mem vault at {0}")]
    NotAVault(PathBuf),
    #[error("No vault configured. Set MEM_VAULT env or pass vault_path.")]
    NoVault,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
