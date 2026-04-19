use chrono::{DateTime, Utc};
use mem_domain::SearchResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "schemars")]
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct VaultInfo {
    pub vault_name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct NoteRef {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub path: String,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct NoteView {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,
    pub tags: Vec<String>,
    pub outgoing_links: Vec<String>,
    /// Parsed body without frontmatter. None when `raw=true`.
    pub body: Option<String>,
    /// Raw file content including frontmatter. Only set when `raw=true`.
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ListParams {
    /// Include archived notes.
    #[serde(default)]
    pub include_archived: bool,
    /// Show only archived notes.
    #[serde(default)]
    pub only_archived: bool,
    /// Filter to notes carrying this tag (case-insensitive, without `#`).
    pub tag: Option<String>,
    /// Maximum results.
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct NewParams {
    /// Title of the new note.
    pub title: String,
    /// Optional initial body (H1 is synthesized from title automatically).
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct RelatedParams {
    pub id_or_slug: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct TagInfo {
    pub name: String,
    pub note_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct ShowParams {
    /// Note id or slug.
    pub id_or_slug: String,
    /// Return raw file content (with frontmatter) instead of parsed body.
    #[serde(default)]
    pub raw: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct UpdateParams {
    /// Note id or slug.
    pub id_or_slug: String,
    /// Set a new title.
    pub title: Option<String>,
    /// Replace body content (keeps H1 line).
    pub body: Option<String>,
    /// Append text to the end.
    pub append: Option<String>,
    /// Archive the note.
    #[serde(default)]
    pub archive: bool,
    /// Unarchive the note.
    #[serde(default)]
    pub unarchive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SearchParams {
    /// Free-text query. Prefix with `#` for exact tag search.
    pub query: String,
    /// Maximum results.
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub excerpt: String,
    pub match_kind: String,
    pub score: f32,
}

impl From<SearchResult> for SearchHit {
    fn from(r: SearchResult) -> Self {
        Self {
            id: r.note_id.0,
            title: r.title,
            excerpt: r.excerpt,
            match_kind: r.match_kind,
            score: r.score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct SyncStatusInfo {
    pub clean: bool,
    pub conflicts: bool,
    pub status: String,
}
