use crate::error::Result;
use crate::vault::open_db;
use std::path::Path;

/// Rebuild index from vault markdown files.
///
/// MVP: reopens the DB and runs migrations + tag count fix. Full rescan of
/// markdown files is not yet implemented (matches current CLI behavior).
pub fn rebuild(vault: &Path) -> Result<()> {
    let _db = open_db(vault)?;
    Ok(())
}
