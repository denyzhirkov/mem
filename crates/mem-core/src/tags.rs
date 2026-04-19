use crate::error::{CoreError, Result};
use crate::types::TagInfo;
use crate::vault::open_db;
use std::path::Path;

/// List all tags in the vault with note counts, ordered by frequency.
pub fn list(vault: &Path) -> Result<Vec<TagInfo>> {
    let db = open_db(vault)?;
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT normalized_name, note_count FROM tags
             WHERE note_count > 0
             ORDER BY note_count DESC, normalized_name ASC",
        )?;
    let iter = stmt.query_map([], |row| {
        Ok(TagInfo {
            name: row.get(0)?,
            note_count: row.get(1)?,
        })
    })?;
    iter.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(CoreError::from)
}
