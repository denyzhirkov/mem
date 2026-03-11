use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use mem_domain::{Note, VaultConfig};
use chrono::Utc;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Path: {0}")]
    InvalidPath(String),
}

/// Helper to get the canonical path for a note
pub fn get_note_path(vault_path: &Path, config: &VaultConfig, note: &Note) -> PathBuf {
    // Basic format: vault/notes/<year>/YYYY-MM-DD-slug.md
    let year = note.created_at.format("%Y").to_string();
    let date_prefix = note.created_at.format("%Y-%m-%d").to_string();
    let filename = format!("{}-{}.md", date_prefix, note.slug);
    
    // In MVP, we can just put everything in the notes directory, or under year.
    // The prompt suggested: notes/2026/2026-03-11-my-note.md
    vault_path
        .join(&config.notes_dir)
        .join(year)
        .join(filename)
}

/// Writes the markdown content to the appropriate file
pub fn write_note(vault_path: &Path, config: &VaultConfig, note: &mut Note, content: &str) -> Result<PathBuf, StorageError> {
    note.updated_at = Utc::now();
    let mut file_path = PathBuf::from(&note.path);
    
    // If path is empty, it's a new note, generate path
    if file_path.as_os_str().is_empty() {
        file_path = get_note_path(vault_path, config, note);
        note.path = file_path.to_string_lossy().to_string();
    }

    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // For the MVP we are assuming no explicit frontmatter is strictly required if we derive from DB,
    // but the prompt said "Use light frontmatter only if it materially simplifies stable IDs... Otherwise derive".
    // Let's write the content directly. The indexer will rely on the DB. 
    // Wait, the prompt said: Option B - derived metadata, but recommended:
    // "Use light frontmatter only if it materially simplifies stable IDs and timestamps."
    // Let's prepend a small HTML comment or frontmatter just for ID and timestamps.
    let frontmatter = format!(
        "---\nid: {}\ntitle: {}\ncreated_at: {}\nupdated_at: {}\ntags: [{}]\n---\n\n",
        note.id.0,
        note.title,
        note.created_at.to_rfc3339(),
        note.updated_at.to_rfc3339(),
        note.tags.join(", ")
    );

    let full_content = format!("{}{}", frontmatter, content);
    fs::write(&file_path, full_content)?;
    
    Ok(file_path)
}

/// Reads the raw file content (including frontmatter)
pub fn read_note_raw(path: &Path) -> Result<String, StorageError> {
    if !path.exists() {
        return Err(StorageError::InvalidPath(path.to_string_lossy().to_string()));
    }
    let content = fs::read_to_string(path)?;
    Ok(content)
}

/// Reads note content stripping YAML frontmatter
pub fn read_note_content(path: &Path) -> Result<String, StorageError> {
    let raw = read_note_raw(path)?;
    Ok(strip_frontmatter(&raw))
}

/// Strip YAML frontmatter (--- ... ---) from content
pub fn strip_frontmatter(content: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    // Find the closing ---
    if let Some(end) = content[3..].find("\n---") {
        let after = end + 3 + 4; // skip past \n---
        let rest = &content[after..];
        // Strip leading newlines after frontmatter
        rest.trim_start_matches('\n').to_string()
    } else {
        content.to_string()
    }
}

/// Deletes a note file
pub fn delete_note(path: &Path) -> Result<(), StorageError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
