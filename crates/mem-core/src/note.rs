use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use mem_domain::{Note, NoteId};
use mem_index::db::IndexDb;
use mem_parser::{extract_links, extract_tags};
use mem_storage::storage::{read_note_content, read_note_raw, write_note};

use crate::error::{CoreError, Result};
use crate::types::{ListParams, NoteRef, NoteView, RelatedParams, ShowParams, UpdateParams};
use crate::util::generate_slug;
use crate::vault::{config, open_db};

/// Create a new note with given title and optional body.
///
/// The final content is `# <title>\n\n<body>`. When `body` is `None`, the
/// note contains just the H1 heading.
pub fn new(vault: &Path, title: &str, body: Option<&str>) -> Result<NoteRef> {
    if title.trim().is_empty() {
        return Err(CoreError::InvalidInput("title must not be empty".into()));
    }

    let cfg = config(vault)?;
    let mut db = open_db(vault)?;

    let content = match body {
        Some(b) => format!("# {}\n\n{}", title, b),
        None => format!("# {}\n\n", title),
    };

    let mut note = Note {
        id: NoteId::default(),
        title: title.to_string(),
        slug: generate_slug(title),
        path: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tags: extract_tags(&content),
        outgoing_links: extract_links(&content),
        content_hash: "empty".to_string(),
        archived: false,
    };

    let path = write_note(vault, &cfg, &mut note, &content)?;
    note.path = path.to_string_lossy().to_string();
    db.upsert_note_with_content(&note, Some(&content))?;

    Ok(note_ref(&note))
}

/// List notes in the vault.
pub fn list(vault: &Path, params: ListParams) -> Result<Vec<NoteRef>> {
    let db = open_db(vault)?;

    let tag_filter = params.tag.as_ref().map(|t| t.trim_start_matches('#').to_lowercase());

    let mut sql = String::from(
        "SELECT n.id, n.slug, n.title, n.path, n.updated_at, n.archived,
                IFNULL(GROUP_CONCAT(nt.tag_name, ','), '')
         FROM notes n
         LEFT JOIN note_tags nt ON n.id = nt.note_id",
    );

    let mut conditions: Vec<String> = Vec::new();
    if params.only_archived {
        conditions.push("n.archived = 1".into());
    } else if !params.include_archived {
        conditions.push("n.archived = 0".into());
    }
    if tag_filter.is_some() {
        conditions.push(
            "n.id IN (SELECT note_id FROM note_tags WHERE LOWER(tag_name) = ?1)".into(),
        );
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" GROUP BY n.id ORDER BY n.updated_at DESC");
    if let Some(lim) = params.limit {
        sql.push_str(&format!(" LIMIT {}", lim));
    }

    let mut stmt = db.conn().prepare(&sql)?;
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<NoteRef> {
        let updated_at_str: String = row.get(4)?;
        let tags_str: String = row.get(6)?;
        let tags: Vec<String> = if tags_str.is_empty() {
            vec![]
        } else {
            tags_str.split(',').map(|s| s.to_string()).collect()
        };
        Ok(NoteRef {
            id: row.get(0)?,
            slug: row.get(1)?,
            title: row.get(2)?,
            path: row.get(3)?,
            updated_at: parse_ts(&updated_at_str),
            archived: row.get(5)?,
            tags,
        })
    };

    let rows: Vec<NoteRef> = if let Some(tag) = tag_filter {
        stmt.query_map([tag], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        stmt.query_map([], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };

    Ok(rows)
}

/// Find notes that share at least one tag with the given note, ranked by
/// number of overlapping tags.
pub fn related(vault: &Path, params: RelatedParams) -> Result<Vec<NoteRef>> {
    let db = open_db(vault)?;
    let note = load_note(&db, &params.id_or_slug)?;
    let limit = params.limit.unwrap_or(20) as i64;

    let mut stmt = db.conn().prepare(
        "SELECT n.id, n.slug, n.title, n.path, n.updated_at, n.archived,
                IFNULL(GROUP_CONCAT(DISTINCT all_tags.tag_name), ''),
                COUNT(DISTINCT nt.tag_name) AS shared
         FROM notes n
         JOIN note_tags nt ON n.id = nt.note_id
         LEFT JOIN note_tags all_tags ON n.id = all_tags.note_id
         WHERE nt.tag_name IN (SELECT tag_name FROM note_tags WHERE note_id = ?1)
         AND n.id != ?1
         AND n.archived = 0
         GROUP BY n.id
         ORDER BY shared DESC, n.updated_at DESC
         LIMIT ?2",
    )?;

    let iter = stmt.query_map(rusqlite::params![&note.id.0, limit], |row| {
        let updated_at_str: String = row.get(4)?;
        let tags_str: String = row.get(6)?;
        let tags: Vec<String> = if tags_str.is_empty() {
            vec![]
        } else {
            tags_str.split(',').map(|s| s.to_string()).collect()
        };
        Ok(NoteRef {
            id: row.get(0)?,
            slug: row.get(1)?,
            title: row.get(2)?,
            path: row.get(3)?,
            updated_at: parse_ts(&updated_at_str),
            archived: row.get(5)?,
            tags,
        })
    })?;

    Ok(iter.collect::<std::result::Result<Vec<_>, _>>()?)
}

/// Show a single note by id or slug.
pub fn show(vault: &Path, params: ShowParams) -> Result<NoteView> {
    let db = open_db(vault)?;
    let note = load_note(&db, &params.id_or_slug)?;
    let tags = load_tags(&db, &note.id.0);
    let path_buf = PathBuf::from(&note.path);

    let (body, raw) = if params.raw {
        (None, Some(read_note_raw(&path_buf)?))
    } else {
        (Some(read_note_content(&path_buf)?), None)
    };

    Ok(NoteView {
        id: note.id.0,
        slug: note.slug,
        title: note.title,
        path: note.path,
        created_at: note.created_at,
        updated_at: note.updated_at,
        archived: note.archived,
        tags,
        outgoing_links: note.outgoing_links,
        body,
        raw,
    })
}

/// Update an existing note.
pub fn update(vault: &Path, params: UpdateParams) -> Result<NoteRef> {
    if params.title.is_none()
        && params.body.is_none()
        && params.append.is_none()
        && !params.archive
        && !params.unarchive
    {
        return Err(CoreError::InvalidInput(
            "nothing to update: provide at least one of title, body, append, archive, unarchive"
                .into(),
        ));
    }
    if params.archive && params.unarchive {
        return Err(CoreError::InvalidInput(
            "archive and unarchive are mutually exclusive".into(),
        ));
    }

    let cfg = config(vault)?;
    let mut db = open_db(vault)?;
    let mut note = load_note(&db, &params.id_or_slug)?;

    let mut content = read_note_content(&PathBuf::from(&note.path))?;
    let old_path = PathBuf::from(&note.path);
    let mut slug_changed = false;

    if let Some(new_title) = &params.title {
        let new_slug = generate_slug(new_title);
        if new_slug != note.slug {
            slug_changed = true;
        }
        note.title = new_title.clone();
        note.slug = new_slug;
        content = replace_h1(&content, new_title);
    }

    if let Some(new_body) = params.body {
        content = replace_body_keep_h1(&content, &new_body);
    }

    if let Some(extra) = params.append {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&extra);
        content.push('\n');
    }

    if params.archive {
        note.archived = true;
    }
    if params.unarchive {
        note.archived = false;
    }

    note.tags = extract_tags(&content);
    note.outgoing_links = extract_links(&content);

    // If slug changed, let write_note pick a fresh path (based on new slug)
    // and remove the old file after successful write.
    if slug_changed {
        note.path = String::new();
    }

    write_note(vault, &cfg, &mut note, &content)?;

    if slug_changed && old_path.exists() && PathBuf::from(&note.path) != old_path {
        let _ = std::fs::remove_file(&old_path);
    }

    db.upsert_note_with_content(&note, Some(&content))?;

    Ok(note_ref(&note))
}

/// Delete a note file and its index entry.
pub fn delete(vault: &Path, id_or_slug: &str) -> Result<()> {
    let mut db = open_db(vault)?;
    let note = load_note(&db, id_or_slug)?;
    mem_storage::storage::delete_note(&PathBuf::from(&note.path))?;
    db.delete_note_index(&note.id.0)?;
    Ok(())
}

// ---------- internals ----------

fn note_ref(note: &Note) -> NoteRef {
    NoteRef {
        id: note.id.0.clone(),
        slug: note.slug.clone(),
        title: note.title.clone(),
        path: note.path.clone(),
        updated_at: note.updated_at,
        archived: note.archived,
        tags: note.tags.clone(),
    }
}

fn load_note(db: &IndexDb, id_or_slug: &str) -> Result<Note> {
    let mut stmt = db.conn().prepare(
        "SELECT id, title, slug, path, created_at, updated_at, content_hash, archived
         FROM notes WHERE id = ?1 OR slug = ?1 LIMIT 1",
    )?;
    let note = stmt
        .query_row([id_or_slug], |row| {
            let created_at_str: String = row.get(4)?;
            let updated_at_str: String = row.get(5)?;
            Ok(Note {
                id: NoteId(row.get(0)?),
                title: row.get(1)?,
                slug: row.get(2)?,
                path: row.get(3)?,
                created_at: parse_ts(&created_at_str),
                updated_at: parse_ts(&updated_at_str),
                tags: vec![],
                outgoing_links: vec![],
                content_hash: row.get(6)?,
                archived: row.get(7)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::NoteNotFound(id_or_slug.to_string()),
            other => CoreError::from(other),
        })?;
    Ok(note)
}

fn load_tags(db: &IndexDb, note_id: &str) -> Vec<String> {
    db.conn()
        .prepare("SELECT tag_name FROM note_tags WHERE note_id = ?1 ORDER BY tag_name")
        .and_then(|mut stmt| {
            let rows = stmt.query_map([note_id], |row| row.get::<_, String>(0))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default()
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn replace_h1(content: &str, new_title: &str) -> String {
    if content.starts_with("# ") {
        if let Some(first_line_end) = content.find('\n') {
            return format!("# {}{}", new_title, &content[first_line_end..]);
        }
        return format!("# {}\n\n", new_title);
    }
    content.to_string()
}

fn replace_body_keep_h1(content: &str, new_body: &str) -> String {
    if content.starts_with("# ") {
        if let Some(first_line_end) = content.find('\n') {
            return format!("{}\n\n{}", &content[..first_line_end], new_body);
        }
    }
    new_body.to_string()
}
