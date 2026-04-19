use crate::error::{CoreError, Result};
use crate::types::VaultInfo;
use mem_index::db::IndexDb;
use mem_storage::config::load_config;
use mem_storage::vault::{default_vault_path, init_vault, is_valid_vault};
use mem_domain::VaultConfig;
use std::path::{Path, PathBuf};

const ENV_VAR: &str = "MEM_VAULT";

/// Resolve a vault path non-interactively.
///
/// Priority:
/// 1. Explicit `path` argument.
/// 2. Current working directory if it's a valid vault.
/// 3. `MEM_VAULT` env var if valid.
/// 4. Default global vault (`~/.mem-vault`) if valid.
/// 5. `Err(CoreError::NoVault)`.
pub fn resolve_vault(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        if is_valid_vault(p) {
            return Ok(p.to_path_buf());
        }
        return Err(CoreError::NotAVault(p.to_path_buf()));
    }

    if let Ok(cwd) = std::env::current_dir() {
        if is_valid_vault(&cwd) {
            return Ok(cwd);
        }
    }

    if let Ok(env_val) = std::env::var(ENV_VAR) {
        let p = PathBuf::from(env_val);
        if is_valid_vault(&p) {
            return Ok(p);
        }
        return Err(CoreError::NotAVault(p));
    }

    let global = default_vault_path();
    if is_valid_vault(&global) {
        return Ok(global);
    }

    Err(CoreError::NoVault)
}

/// Initialize a new vault at `path`.
pub fn init(path: &Path, name: Option<String>) -> Result<VaultInfo> {
    let cfg = init_vault(path, name)?;
    // Create index DB so vault is fully usable.
    let _db = open_db(path)?;
    Ok(VaultInfo {
        vault_name: cfg.vault_name,
        path: path.to_path_buf(),
    })
}

/// Load vault config. Errors if not a valid vault.
pub fn config(vault: &Path) -> Result<VaultConfig> {
    Ok(load_config(vault)?)
}

/// Open the SQLite index DB for a vault.
pub fn open_db(vault: &Path) -> Result<IndexDb> {
    let db_path = vault.join(".mem").join("index.sqlite");
    Ok(IndexDb::open(&db_path)?)
}
