use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

use crate::schema::MIGRATIONS;

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Migration error. Current: {current}, Target: {target}")]
    Migration { current: usize, target: usize },
}

pub struct IndexDb {
    conn: Connection,
}

impl IndexDb {
    pub fn open(db_path: &Path) -> Result<Self, IndexError> {
        let conn = Connection::open(db_path)?;
        
        // Optimize for single-threaded CLI/Desktop SQLite access
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut db = Self { conn };
        db.migrate()?;
        db.fix_tag_counts()?;

        Ok(db)
    }

    fn migrate(&mut self) -> Result<(), IndexError> {
        let current_version_i64: i64 = self.conn
            .query_row(
                "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let current_version = current_version_i64 as usize;

        if current_version < MIGRATIONS.len() {
            let tx = self.conn.transaction()?;
            for (idx, sql) in MIGRATIONS.iter().enumerate().skip(current_version) {
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    [&((idx + 1) as i64) as &dyn rusqlite::ToSql],
                )?;
            }
            tx.commit()?;
        }

        Ok(())
    }

    /// Recalculate tag counts from note_tags and remove orphaned tags
    fn fix_tag_counts(&mut self) -> Result<(), IndexError> {
        self.conn.execute_batch(
            "UPDATE tags SET note_count = (
                SELECT COUNT(*) FROM note_tags WHERE tag_name = tags.normalized_name
             );
             DELETE FROM tags WHERE note_count <= 0;"
        )?;
        Ok(())
    }

    // Access to raw connection if needed
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
