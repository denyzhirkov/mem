pub const MIGRATIONS: &[&str] = &[
    // v1: Initial schema
    r#"
    CREATE TABLE IF NOT EXISTS notes (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        slug TEXT NOT NULL,
        path TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        archived BOOLEAN NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS tags (
        normalized_name TEXT PRIMARY KEY,
        display_name TEXT NOT NULL,
        note_count INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS note_tags (
        note_id TEXT NOT NULL,
        tag_name TEXT NOT NULL,
        PRIMARY KEY (note_id, tag_name),
        FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
        FOREIGN KEY (tag_name) REFERENCES tags(normalized_name) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS links (
        source_id TEXT NOT NULL,
        target_title TEXT NOT NULL,
        PRIMARY KEY (source_id, target_title),
        FOREIGN KEY (source_id) REFERENCES notes(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY
    );
    "#,

    // v2: FTS5 full-text search on note content
    r#"
    CREATE VIRTUAL TABLE IF NOT EXISTS fts_notes USING fts5(
        note_id UNINDEXED,
        title,
        content,
        tokenize='unicode61'
    );
    "#,
];
