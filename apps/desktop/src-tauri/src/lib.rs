use std::path::PathBuf;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use mem_core::types::{ListParams, RelatedParams, ShowParams, UpdateParams};
use mem_core::{note, tags, vault};
use mem_domain::SearchResult;
use mem_index::db::IndexDb;
use serde::Serialize;
use tauri::{Emitter, Manager};

pub struct AppState {
    pub db: Mutex<IndexDb>,
    pub vault_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteWithTags {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub updated_at: String,
}

impl From<mem_core::NoteRef> for NoteWithTags {
    fn from(r: mem_core::NoteRef) -> Self {
        Self {
            id: r.id,
            title: r.title,
            slug: r.slug,
            tags: r.tags,
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}

#[tauri::command]
fn list_notes(state: tauri::State<'_, AppState>) -> Result<Vec<NoteWithTags>, String> {
    let refs = note::list(&state.vault_path, ListParams::default()).map_err(|e| e.to_string())?;
    Ok(refs.into_iter().map(Into::into).collect())
}

#[tauri::command]
fn get_note(id: String, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let view = note::show(
        &state.vault_path,
        ShowParams {
            id_or_slug: id,
            raw: false,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(view.body.unwrap_or_default())
}

#[tauri::command]
fn get_note_tags(id: String, state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(load_tags_for_note(&db, &id))
}

#[tauri::command]
fn get_related_notes(
    id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<NoteWithTags>, String> {
    let refs = note::related(
        &state.vault_path,
        RelatedParams {
            id_or_slug: id,
            limit: Some(20),
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(refs.into_iter().map(Into::into).collect())
}

#[tauri::command]
fn all_tags(state: tauri::State<'_, AppState>) -> Result<Vec<(String, i64)>, String> {
    let rows = tags::list(&state.vault_path).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|t| (t.name, t.note_count)).collect())
}

#[tauri::command]
fn create_note(
    title: String,
    state: tauri::State<'_, AppState>,
) -> Result<NoteWithTags, String> {
    let r = note::new(&state.vault_path, &title, None).map_err(|e| e.to_string())?;
    refresh_db(&state)?;
    Ok(r.into())
}

#[tauri::command]
fn update_note(
    id: String,
    title: String,
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<NoteWithTags, String> {
    let r = note::update(
        &state.vault_path,
        UpdateParams {
            id_or_slug: id,
            title: Some(title),
            body: Some(content),
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    refresh_db(&state)?;
    Ok(r.into())
}

#[tauri::command]
fn remove_note(id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    note::delete(&state.vault_path, &id).map_err(|e| e.to_string())?;
    refresh_db(&state)?;
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

    let mut stmt = db
        .conn()
        .prepare(
            "SELECT normalized_name, note_count FROM tags WHERE note_count > 0 ORDER BY note_count DESC",
        )
        .map_err(|e| e.to_string())?;
    let nodes: Vec<TagNode> = stmt
        .query_map([], |row| {
            Ok(TagNode {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = db
        .conn()
        .prepare(
            "SELECT a.tag_name, b.tag_name, COUNT(*) as weight
             FROM note_tags a
             JOIN note_tags b ON a.note_id = b.note_id AND a.tag_name < b.tag_name
             GROUP BY a.tag_name, b.tag_name",
        )
        .map_err(|e| e.to_string())?;
    let edges: Vec<TagEdge> = stmt
        .query_map([], |row| {
            Ok(TagEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(TagGraphData { nodes, edges })
}

#[tauri::command]
fn search_notes(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    // Frontend expects domain SearchResult shape; use index directly to preserve it.
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_notes_by_tag(tag: String, state: tauri::State<'_, AppState>) -> Result<Vec<NoteWithTags>, String> {
    let refs = note::list(
        &state.vault_path,
        ListParams { tag: Some(tag), ..Default::default() },
    )
    .map_err(|e| e.to_string())?;
    Ok(refs.into_iter().map(Into::into).collect())
}

#[tauri::command]
fn related_tags(tag: String, state: tauri::State<'_, AppState>) -> Result<Vec<(String, i64)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .conn()
        .prepare(
            "SELECT b.tag_name, COUNT(*) as weight
             FROM note_tags a
             JOIN note_tags b ON a.note_id = b.note_id AND a.tag_name != b.tag_name
             WHERE a.tag_name = ?1
             GROUP BY b.tag_name
             ORDER BY weight DESC, b.tag_name ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, i64)> = stmt
        .query_map([tag], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn load_tags_for_note(db: &IndexDb, note_id: &str) -> Vec<String> {
    let mut stmt = db
        .conn()
        .prepare("SELECT tag_name FROM note_tags WHERE note_id = ?1 ORDER BY tag_name")
        .unwrap();
    stmt.query_map([note_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

/// Reopen the in-state DB connection after write ops so read-only commands see
/// fresh data (WAL visibility across connections is fine, but this keeps FTS
/// counters current without relying on connection cache).
fn refresh_db(state: &tauri::State<'_, AppState>) -> Result<(), String> {
    let new_db = vault::open_db(&state.vault_path).map_err(|e| e.to_string())?;
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    *guard = new_db;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let vault_path = mem_storage::vault::default_vault_path();
            if !mem_storage::vault::is_valid_vault(&vault_path) {
                vault::init(&vault_path, Some("Personal".to_string()))
                    .expect("Failed to init vault");
            }
            let db = vault::open_db(&vault_path).expect("Failed to open DB");

            app.manage(AppState {
                db: Mutex::new(db),
                vault_path: vault_path.clone(),
            });

            let app_handle = app.handle().clone();
            let notes_path = vault_path.join("notes");
            std::thread::spawn(move || {
                use notify::{recommended_watcher, Event, RecursiveMode, Watcher};
                let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
                let mut watcher = match recommended_watcher(tx) {
                    Ok(w) => w,
                    Err(e) => { eprintln!("watcher init: {e}"); return; }
                };
                if watcher.watch(&notes_path, RecursiveMode::NonRecursive).is_err() {
                    return;
                }
                let debounce = Duration::from_millis(300);
                let mut pending = false;
                let mut last = Instant::now();
                loop {
                    match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(Ok(_)) => { pending = true; last = Instant::now(); }
                        Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Timeout) => {
                            if pending && last.elapsed() >= debounce {
                                pending = false;
                                let _ = app_handle.emit("vault-changed", ());
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
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
            tag_graph,
            list_notes_by_tag,
            related_tags
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
