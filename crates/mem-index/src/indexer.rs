use crate::db::{IndexDb, IndexError};
use mem_domain::Note;

impl IndexDb {
    /// Upsert note metadata + FTS content
    pub fn upsert_note(&mut self, note: &Note) -> Result<(), IndexError> {
        self.upsert_note_with_content(note, None)
    }

    /// Upsert note metadata + FTS content (with optional body for FTS)
    pub fn upsert_note_with_content(&mut self, note: &Note, content: Option<&str>) -> Result<(), IndexError> {
        let tx = self.conn_mut().transaction()?;

        // 1. Insert/Update Note
        tx.execute(
            "INSERT INTO notes (id, title, slug, path, created_at, updated_at, content_hash, archived)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                title=excluded.title,
                slug=excluded.slug,
                path=excluded.path,
                updated_at=excluded.updated_at,
                content_hash=excluded.content_hash,
                archived=excluded.archived",
            (
                &note.id.0,
                &note.title,
                &note.slug,
                &note.path,
                note.created_at.to_rfc3339(),
                note.updated_at.to_rfc3339(),
                &note.content_hash,
                note.archived,
            ),
        )?;

        // 2. Tags & links
        tx.execute("DELETE FROM note_tags WHERE note_id = ?1", [&note.id.0])?;
        tx.execute("DELETE FROM links WHERE source_id = ?1", [&note.id.0])?;

        for tag in &note.tags {
            tx.execute(
                "INSERT INTO tags (normalized_name, display_name, note_count)
                 VALUES (?1, ?1, 1)
                 ON CONFLICT(normalized_name) DO UPDATE SET note_count = note_count + 1",
                [tag],
            )?;
            tx.execute(
                "INSERT INTO note_tags (note_id, tag_name) VALUES (?1, ?2)",
                (&note.id.0, tag),
            )?;
        }

        for target_title in &note.outgoing_links {
            tx.execute(
                "INSERT INTO links (source_id, target_title) VALUES (?1, ?2)",
                (&note.id.0, target_title),
            )?;
        }

        // 3. FTS5 — update full-text index
        tx.execute("DELETE FROM fts_notes WHERE note_id = ?1", [&note.id.0])?;
        let body = content.unwrap_or("");
        tx.execute(
            "INSERT INTO fts_notes (note_id, title, content) VALUES (?1, ?2, ?3)",
            (&note.id.0, &note.title, body),
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn delete_note_index(&mut self, id: &str) -> Result<(), IndexError> {
        let tx = self.conn_mut().transaction()?;
        tx.execute(
            "UPDATE tags SET note_count = note_count - 1
             WHERE normalized_name IN (SELECT tag_name FROM note_tags WHERE note_id = ?1)",
            [id],
        )?;
        tx.execute("DELETE FROM fts_notes WHERE note_id = ?1", [id])?;
        tx.execute("DELETE FROM notes WHERE id = ?1", [id])?;
        tx.commit()?;
        Ok(())
    }
}
