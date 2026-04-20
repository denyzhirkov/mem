use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Generic `{ "ok": true }` response for tools that only signal success.
#[derive(Debug, Serialize, JsonSchema)]
pub struct OkResponse {
    pub ok: bool,
}

/// Response wrapping captured process output (stdout/stderr merged).
#[derive(Debug, Serialize, JsonSchema)]
pub struct OutputResponse {
    pub output: String,
}

/// Wrapper for list responses. MCP `outputSchema` must have `type: object`,
/// so tools returning a list expose it under `items`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListResponse<T: JsonSchema + Serialize> {
    pub items: Vec<T>,
}

impl<T: JsonSchema + Serialize> ListResponse<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }
}

/// Tool args take an optional `vault_path`. If omitted, the server falls back
/// to `MEM_VAULT` env and then to the default global vault (`~/.mem-vault`).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteNewArgs {
    /// Title of the new note.
    pub title: String,
    /// Optional initial body. An H1 with the title is prepended automatically.
    pub body: Option<String>,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct NoteListArgs {
    /// Include archived notes.
    #[serde(default)]
    pub include_archived: bool,
    /// Only archived notes.
    #[serde(default)]
    pub only_archived: bool,
    /// Filter to notes carrying this tag (without `#`, case-insensitive).
    pub tag: Option<String>,
    /// Maximum number of results.
    pub limit: Option<u32>,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteRelatedArgs {
    /// Note id or slug.
    pub id_or_slug: String,
    /// Maximum results (default 20).
    pub limit: Option<u32>,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteShowArgs {
    /// Note id or slug.
    pub id_or_slug: String,
    /// Return raw file content (with YAML frontmatter) instead of parsed body.
    #[serde(default)]
    pub raw: bool,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteUpdateArgs {
    pub id_or_slug: String,
    pub title: Option<String>,
    /// Replace body content (keeps H1 line).
    pub body: Option<String>,
    /// Append text to the end.
    pub append: Option<String>,
    #[serde(default)]
    pub archive: bool,
    #[serde(default)]
    pub unarchive: bool,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoteDeleteArgs {
    pub id_or_slug: String,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchArgs {
    /// Free-text query. Prefix with `#` for exact tag search.
    pub query: String,
    pub limit: Option<u32>,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct VaultRef {
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VaultInitArgs {
    /// Path at which to initialize the vault.
    pub path: PathBuf,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncCommitArgs {
    pub message: String,
    #[serde(default)]
    pub vault_path: Option<PathBuf>,
}
