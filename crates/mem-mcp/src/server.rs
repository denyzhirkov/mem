use std::path::{Path, PathBuf};

use mem_core::types::{ListParams, RelatedParams, SearchParams, ShowParams, UpdateParams};
use mem_core::{note, search, sync, tags, vault, CoreError};

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ErrorData as McpError, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};

use crate::params::*;

#[derive(Clone)]
pub struct MemServer {
    tool_router: ToolRouter<Self>,
}

impl MemServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    fn resolve(explicit: Option<&Path>) -> Result<PathBuf, McpError> {
        vault::resolve_vault(explicit).map_err(core_to_mcp)
    }

    /// Snapshot of all registered tool definitions. Useful for tests and
    /// debugging tool schemas without spinning up a transport.
    pub fn tool_router_snapshot(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router.list_all()
    }
}

#[tool_router]
impl MemServer {
    #[tool(name = "vault_init", description = "Initialize a new mem vault at the given path.")]
    async fn vault_init(
        &self,
        Parameters(VaultInitArgs { path, name }): Parameters<VaultInitArgs>,
    ) -> Result<Json<mem_core::VaultInfo>, McpError> {
        vault::init(&path, name).map(Json).map_err(core_to_mcp)
    }

    #[tool(
        name = "note_new",
        description = "Create a new note with the given title and optional body."
    )]
    async fn note_new(
        &self,
        Parameters(args): Parameters<NoteNewArgs>,
    ) -> Result<Json<mem_core::NoteRef>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        note::new(&v, &args.title, args.body.as_deref())
            .map(Json)
            .map_err(core_to_mcp)
    }

    #[tool(
        name = "note_list",
        description = "List notes in the vault, newest first. Optional tag filter."
    )]
    async fn note_list(
        &self,
        Parameters(args): Parameters<NoteListArgs>,
    ) -> Result<Json<Vec<mem_core::NoteRef>>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        note::list(
            &v,
            ListParams {
                include_archived: args.include_archived,
                only_archived: args.only_archived,
                tag: args.tag,
                limit: args.limit,
            },
        )
        .map(Json)
        .map_err(core_to_mcp)
    }

    #[tool(
        name = "note_related",
        description = "Find notes related to the given one by shared tags, ranked by overlap."
    )]
    async fn note_related(
        &self,
        Parameters(args): Parameters<NoteRelatedArgs>,
    ) -> Result<Json<Vec<mem_core::NoteRef>>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        note::related(
            &v,
            RelatedParams {
                id_or_slug: args.id_or_slug,
                limit: args.limit,
            },
        )
        .map(Json)
        .map_err(core_to_mcp)
    }

    #[tool(
        name = "tags_list",
        description = "List all tags in the vault with note counts, most-used first."
    )]
    async fn tags_list(
        &self,
        Parameters(args): Parameters<VaultRef>,
    ) -> Result<Json<Vec<mem_core::TagInfo>>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        tags::list(&v).map(Json).map_err(core_to_mcp)
    }

    #[tool(
        name = "note_show",
        description = "Show a note by id or slug. Returns parsed body by default; set raw=true for the full file with frontmatter."
    )]
    async fn note_show(
        &self,
        Parameters(args): Parameters<NoteShowArgs>,
    ) -> Result<Json<mem_core::NoteView>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        note::show(
            &v,
            ShowParams {
                id_or_slug: args.id_or_slug,
                raw: args.raw,
            },
        )
        .map(Json)
        .map_err(core_to_mcp)
    }

    #[tool(
        name = "note_update",
        description = "Update a note. Provide at least one of: title, body, append, archive, unarchive."
    )]
    async fn note_update(
        &self,
        Parameters(args): Parameters<NoteUpdateArgs>,
    ) -> Result<Json<mem_core::NoteRef>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        note::update(
            &v,
            UpdateParams {
                id_or_slug: args.id_or_slug,
                title: args.title,
                body: args.body,
                append: args.append,
                archive: args.archive,
                unarchive: args.unarchive,
            },
        )
        .map(Json)
        .map_err(core_to_mcp)
    }

    #[tool(name = "note_delete", description = "Delete a note by id or slug.")]
    async fn note_delete(
        &self,
        Parameters(args): Parameters<NoteDeleteArgs>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        note::delete(&v, &args.id_or_slug).map_err(core_to_mcp)?;
        Ok(Json(serde_json::json!({"ok": true})))
    }

    #[tool(
        name = "search",
        description = "Search notes. Prefix the query with `#` for exact tag search."
    )]
    async fn search(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<Json<Vec<mem_core::SearchHit>>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        search::run(
            &v,
            SearchParams {
                query: args.query,
                limit: args.limit,
            },
        )
        .map(Json)
        .map_err(core_to_mcp)
    }

    #[tool(name = "sync_status", description = "Show git status of the vault.")]
    async fn sync_status(
        &self,
        Parameters(args): Parameters<VaultRef>,
    ) -> Result<Json<mem_core::SyncStatusInfo>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        sync::status(&v).map(Json).map_err(core_to_mcp)
    }

    #[tool(name = "sync_commit", description = "Commit all changes in the vault.")]
    async fn sync_commit(
        &self,
        Parameters(args): Parameters<SyncCommitArgs>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        sync::commit(&v, &args.message).map_err(core_to_mcp)?;
        Ok(Json(serde_json::json!({"ok": true})))
    }

    #[tool(name = "sync_pull", description = "Pull changes from the remote.")]
    async fn sync_pull(
        &self,
        Parameters(args): Parameters<VaultRef>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        let out = sync::pull(&v).map_err(core_to_mcp)?;
        Ok(Json(serde_json::json!({"output": out})))
    }

    #[tool(name = "sync_push", description = "Push changes to the remote.")]
    async fn sync_push(
        &self,
        Parameters(args): Parameters<VaultRef>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        let out = sync::push(&v).map_err(core_to_mcp)?;
        Ok(Json(serde_json::json!({"output": out})))
    }

    #[tool(
        name = "index_rebuild",
        description = "Rebuild the SQLite index (no-op in MVP beyond ensuring migrations)."
    )]
    async fn index_rebuild(
        &self,
        Parameters(args): Parameters<VaultRef>,
    ) -> Result<Json<serde_json::Value>, McpError> {
        let v = Self::resolve(args.vault_path.as_deref())?;
        mem_core::index::rebuild(&v).map_err(core_to_mcp)?;
        Ok(Json(serde_json::json!({"ok": true})))
    }
}

#[tool_handler]
impl ServerHandler for MemServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "mem — local-first Markdown knowledge base. Tools manage notes, search, and git sync. \
                 Tools accept an optional `vault_path`; when omitted, the server uses `MEM_VAULT` env \
                 or the default global vault at `~/.mem-vault`."
                    .to_string(),
            ),
            ..Default::default()
        }
    }
}

fn core_to_mcp(err: CoreError) -> McpError {
    match err {
        CoreError::NoteNotFound(_) | CoreError::InvalidInput(_) => {
            McpError::invalid_params(err.to_string(), None)
        }
        CoreError::NoVault | CoreError::NotAVault(_) => {
            McpError::invalid_params(err.to_string(), None)
        }
        _ => McpError::internal_error(err.to_string(), None),
    }
}
