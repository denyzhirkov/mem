use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteId(pub String);

impl Default for NoteId {
    fn default() -> Self {
        NoteId(ulid::Ulid::new().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub title: String,
    pub slug: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub outgoing_links: Vec<String>,
    pub content_hash: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub normalized_name: String,
    pub display_name: String,
    pub note_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub source_id: NoteId,
    pub target_title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub note_id: NoteId,
    pub title: String,
    pub excerpt: String,
    pub match_kind: String,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    pub enabled: bool,
    pub auto_commit: bool,
    pub default_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultConfig {
    pub version: u32,
    pub vault_name: String,
    pub notes_dir: String,
    pub index_db_path: String,
    pub default_editor_mode: String,
    pub git: Option<GitConfig>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            version: 1,
            vault_name: "personal".to_string(),
            notes_dir: "notes".to_string(),
            index_db_path: ".mem/index.sqlite".to_string(),
            default_editor_mode: "rich".to_string(),
            git: Some(GitConfig {
                enabled: true,
                auto_commit: false,
                default_branch: "main".to_string(),
            }),
        }
    }
}
