use std::sync::Mutex;
use std::path::PathBuf;
use mem_domain::{Note, NoteId, SearchResult, VaultConfig};
use mem_storage::vault::init_vault;
use mem_storage::config::load_config;
use mem_storage::storage::{write_note, read_note_content, delete_note};
use mem_index::db::IndexDb;
use mem_parser::{extract_tags, extract_links};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<IndexDb>,
    pub vault_path: PathBuf,
    pub config: VaultConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteWithTags {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub updated_at: String,
}

fn note_row_to_note(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let created_at_str: String = row.get(4)?;
    let updated_at_str: String = row.get(5)?;
    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Ok(Note {
        id: NoteId(row.get(0)?),
        title: row.get(1)?,
        slug: row.get(2)?,
        path: row.get(3)?,
        created_at,
        updated_at,
        tags: vec![],
        outgoing_links: vec![],
        content_hash: row.get(6)?,
        archived: row.get(7)?,
    })
}

fn load_tags_for_note(db: &IndexDb, note_id: &str) -> Vec<String> {
    let mut stmt = db.conn().prepare(
        "SELECT tag_name FROM note_tags WHERE note_id = ?1 ORDER BY tag_name"
    ).unwrap();
    stmt.query_map([note_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

#[tauri::command]
fn list_notes(state: tauri::State<'_, AppState>) -> Result<Vec<NoteWithTags>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn().prepare(
        "SELECT n.id, n.title, n.slug, n.updated_at, IFNULL(GROUP_CONCAT(nt.tag_name, ','), '')
         FROM notes n
         LEFT JOIN note_tags nt ON n.id = nt.note_id
         GROUP BY n.id
         ORDER BY n.updated_at DESC"
    ).map_err(|e| e.to_string())?;

    let iter = stmt.query_map([], |row| {
        let tags_str: String = row.get(4)?;
        let tags: Vec<String> = if tags_str.is_empty() {
            vec![]
        } else {
            tags_str.split(',').map(|s| s.to_string()).collect()
        };
        Ok(NoteWithTags {
            id: row.get(0)?,
            title: row.get(1)?,
            slug: row.get(2)?,
            updated_at: row.get(3)?,
            tags,
        })
    }).map_err(|e| e.to_string())?;

    iter.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_note(id: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let path: String = db.conn().query_row(
        "SELECT path FROM notes WHERE id = ?1 LIMIT 1", [&id], |row| row.get(0)
    ).map_err(|e| e.to_string())?;
    read_note_content(&PathBuf::from(path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_note_tags(id: String, state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(load_tags_for_note(&db, &id))
}

#[tauri::command]
fn get_related_notes(id: String, state: tauri::State<'_, AppState>) -> Result<Vec<NoteWithTags>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn().prepare(
        "SELECT n.id, n.title, n.slug, n.updated_at, IFNULL(GROUP_CONCAT(DISTINCT all_tags.tag_name), '')
         FROM notes n
         JOIN note_tags nt ON n.id = nt.note_id
         LEFT JOIN note_tags all_tags ON n.id = all_tags.note_id
         WHERE nt.tag_name IN (SELECT tag_name FROM note_tags WHERE note_id = ?1)
         AND n.id != ?1
         GROUP BY n.id
         HAVING COUNT(DISTINCT nt.tag_name) > 0
         ORDER BY n.updated_at DESC
         LIMIT 20"
    ).map_err(|e| e.to_string())?;

    let iter = stmt.query_map([&id], |row| {
        let tags_str: String = row.get(4)?;
        let tags: Vec<String> = if tags_str.is_empty() {
            vec![]
        } else {
            tags_str.split(',').map(|s| s.to_string()).collect()
        };
        Ok(NoteWithTags {
            id: row.get(0)?,
            title: row.get(1)?,
            slug: row.get(2)?,
            updated_at: row.get(3)?,
            tags,
        })
    }).map_err(|e| e.to_string())?;

    iter.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn all_tags(state: tauri::State<'_, AppState>) -> Result<Vec<(String, i64)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn().prepare(
        "SELECT normalized_name, note_count FROM tags WHERE note_count > 0 ORDER BY note_count DESC"
    ).map_err(|e| e.to_string())?;
    let iter = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    }).map_err(|e| e.to_string())?;
    let mut tags = vec![];
    for r in iter {
        tags.push(r.map_err(|e| e.to_string())?);
    }
    Ok(tags)
}

#[tauri::command]
fn create_note(title: String, state: tauri::State<'_, AppState>) -> Result<NoteWithTags, String> {
    let slug = generate_slug(&title);
    let content = format!("# {}\n\n", title);
    let tags = extract_tags(&content);
    let links = extract_links(&content);

    let mut note = Note {
        id: NoteId::default(),
        title: title.clone(),
        slug: slug.clone(),
        path: String::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tags: tags.clone(),
        outgoing_links: links,
        content_hash: "empty".to_string(),
        archived: false,
    };

    write_note(&state.vault_path, &state.config, &mut note, &content).map_err(|e| e.to_string())?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.upsert_note_with_content(&note, Some(&content)).map_err(|e| e.to_string())?;

    Ok(NoteWithTags {
        id: note.id.0,
        title,
        slug,
        tags,
        updated_at: note.updated_at.to_rfc3339(),
    })
}

#[tauri::command]
fn update_note(id: String, title: String, content: String, state: tauri::State<'_, AppState>) -> Result<NoteWithTags, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.conn().prepare(
        "SELECT id, title, slug, path, created_at, updated_at, content_hash, archived FROM notes WHERE id = ?1 LIMIT 1"
    ).map_err(|e| e.to_string())?;

    let mut note: Note = stmt.query_row([&id], note_row_to_note).map_err(|e| e.to_string())?;
    drop(stmt);
    drop(db);

    note.title = title.clone();
    note.tags = extract_tags(&content);
    note.outgoing_links = extract_links(&content);

    write_note(&state.vault_path, &state.config, &mut note, &content).map_err(|e| e.to_string())?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    db.upsert_note_with_content(&note, Some(&content)).map_err(|e| e.to_string())?;

    let tags = note.tags.clone();
    Ok(NoteWithTags {
        id: note.id.0,
        title,
        slug: note.slug,
        tags,
        updated_at: note.updated_at.to_rfc3339(),
    })
}

#[tauri::command]
fn remove_note(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let path: String = db.conn().query_row(
        "SELECT path FROM notes WHERE id = ?1 LIMIT 1", [&id], |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    delete_note(&PathBuf::from(&path)).map_err(|e| e.to_string())?;
    db.delete_note_index(&id).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct TagGraphData {
    pub nodes: Vec<TagNode>,
    pub edges: Vec<TagEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagNode {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagEdge {
    pub source: String,
    pub target: String,
    pub weight: i64,
}

#[tauri::command]
fn tag_graph(state: tauri::State<'_, AppState>) -> Result<TagGraphData, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Nodes: tags with note count
    let mut stmt = db.conn().prepare(
        "SELECT normalized_name, note_count FROM tags WHERE note_count > 0 ORDER BY note_count DESC"
    ).map_err(|e| e.to_string())?;
    let nodes: Vec<TagNode> = stmt.query_map([], |row| {
        Ok(TagNode { name: row.get(0)?, count: row.get(1)? })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    // Edges: tags co-occurring in same note
    let mut stmt = db.conn().prepare(
        "SELECT a.tag_name, b.tag_name, COUNT(*) as weight
         FROM note_tags a
         JOIN note_tags b ON a.note_id = b.note_id AND a.tag_name < b.tag_name
         GROUP BY a.tag_name, b.tag_name"
    ).map_err(|e| e.to_string())?;
    let edges: Vec<TagEdge> = stmt.query_map([], |row| {
        Ok(TagEdge { source: row.get(0)?, target: row.get(1)?, weight: row.get(2)? })
    }).map_err(|e| e.to_string())?
      .filter_map(|r| r.ok())
      .collect();

    Ok(TagGraphData { nodes, edges })
}

#[tauri::command]
fn search_notes(query: String, state: tauri::State<'_, AppState>) -> Result<Vec<SearchResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search(&query).map_err(|e| e.to_string())
}

fn generate_slug(title: &str) -> String {
    title.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let vault_path = mem_storage::vault::default_vault_path();
            if !mem_storage::vault::is_valid_vault(&vault_path) {
                init_vault(&vault_path, Some("Personal".to_string())).expect("Failed to init vault");
            }
            let config = load_config(&vault_path).expect("Failed to load config");
            let db_path = vault_path.join(".mem").join("index.sqlite");
            let db = IndexDb::open(&db_path).expect("Failed to open DB");

            app.manage(AppState {
                db: Mutex::new(db),
                vault_path,
                config,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_notes,
            get_note,
            get_note_tags,
            get_related_notes,
            all_tags,
            create_note,
            update_note,
            remove_note,
            search_notes,
            tag_graph
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
