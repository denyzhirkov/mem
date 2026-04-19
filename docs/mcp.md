# MCP integration

`mem-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) server
that exposes your mem vault to MCP-aware AI clients (Claude Desktop, Claude
Code, Cursor, etc.). It speaks JSON-RPC over stdio and delegates every tool
call to `mem-core` — the same library the CLI and desktop app use.

## Install

`mem-mcp` ships alongside `mem` in releases. The installer picks up both:

```sh
curl -fsSL https://raw.githubusercontent.com/denyzhirkov/mem/master/scripts/install.sh | sh
```

Build from source:

```sh
cargo build --release -p mem-mcp
# binary at target/release/mem-mcp
```

## Vault resolution

Each tool accepts an optional `vault_path`. When omitted, the server resolves
the vault in this order:

1. `MEM_VAULT` environment variable
2. Default global vault (`~/.mem-vault`)

The server is non-interactive — if no vault can be found, the call fails with
an `invalid_params` error telling the agent to set `MEM_VAULT` or pass
`vault_path`.

## Client configuration

### Claude Desktop / Claude Code

Add to `~/.claude.json` (or your project-scoped equivalent):

```json
{
  "mcpServers": {
    "mem": {
      "command": "mem-mcp",
      "env": {
        "MEM_VAULT": "/absolute/path/to/vault"
      }
    }
  }
}
```

### Cursor

`~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "mem": {
      "command": "mem-mcp",
      "env": { "MEM_VAULT": "/absolute/path/to/vault" }
    }
  }
}
```

## Tools

| Tool | Purpose |
|------|---------|
| `vault_init` | Initialize a new vault (needs `path`). |
| `note_new` | Create a note from a title and optional body. |
| `note_list` | List notes; filter by `tag`; `include_archived`, `only_archived`, `limit`. |
| `note_show` | Show note by id or slug; `raw=true` for file with frontmatter. |
| `note_update` | Update title / replace body / append / archive / unarchive. Renames the file on slug change. |
| `note_delete` | Delete a note and its index entry. |
| `note_related` | Notes sharing tags with the given one, ranked by overlap. |
| `tags_list` | All tags in the vault with note counts, most-used first. |
| `search` | Full-text search. Prefix with `#` for exact tag search. |
| `sync_status` | Git status of the vault. |
| `sync_commit` | Stage and commit all changes with a message. |
| `sync_pull` | `git pull --rebase`. |
| `sync_push` | `git push`. |
| `index_rebuild` | Re-open the SQLite index (runs migrations). |

All tools return structured JSON that mirrors `mem-core` types.

## Logging

The server writes logs to stderr (JSON-RPC uses stdout). Tune verbosity with
`MEM_MCP_LOG` (accepts `tracing_subscriber::EnvFilter` syntax):

```sh
MEM_MCP_LOG=debug mem-mcp
```
