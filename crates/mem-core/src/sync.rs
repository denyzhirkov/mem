use crate::error::Result;
use crate::types::SyncStatusInfo;
use std::path::Path;

pub fn status(vault: &Path) -> Result<SyncStatusInfo> {
    let status = mem_sync::sync_status(vault)?;
    let clean = status.is_empty();
    let conflicts = if clean {
        false
    } else {
        mem_sync::check_conflicts(vault)?
    };
    Ok(SyncStatusInfo {
        clean,
        conflicts,
        status,
    })
}

pub fn commit(vault: &Path, message: &str) -> Result<()> {
    Ok(mem_sync::commit_all(vault, message)?)
}

pub fn pull(vault: &Path) -> Result<String> {
    Ok(mem_sync::pull(vault)?)
}

pub fn push(vault: &Path) -> Result<String> {
    Ok(mem_sync::push(vault)?)
}
