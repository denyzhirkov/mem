# mem

Minimal local-first knowledge keeper. Markdown files, SQLite index, tiny desktop UI, CLI.

## Install

### Desktop app

Download the latest release for your platform:

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [.dmg](https://github.com/denyzhirkov/mem/releases/latest) |
| macOS (Intel) | [.dmg](https://github.com/denyzhirkov/mem/releases/latest) |
| Linux | [.AppImage](https://github.com/denyzhirkov/mem/releases/latest) / [.deb](https://github.com/denyzhirkov/mem/releases/latest) |
| Windows | [.msi](https://github.com/denyzhirkov/mem/releases/latest) |

### CLI

```bash
curl -fsSL https://raw.githubusercontent.com/denyzhirkov/mem/master/scripts/install.sh | sh
```

Or build from source:

```bash
cargo install --path crates/mem-cli
```

## Usage

### Desktop

Open the app and start writing. That's it.

- **Cmd+S** — save note
- **Cmd+N** — new blank note
- **Cmd+P** — search / switch notes
- **#tag** — type tags inline, they auto-extract on save

Notes are Markdown files in `~/.mem-vault/notes/`. Tags create connections — related notes appear at the bottom.

Auto-save kicks in after you stop typing or every ~80 characters.

### CLI

```bash
mem init                        # create a vault in current directory
mem note new "My idea"          # create a note
mem note list                   # list all notes
mem note show <id-or-slug>      # print note content
mem search "query"              # search notes
mem sync status                 # git status
mem sync commit "update notes"  # git commit
mem sync pull                   # git pull
mem sync push                   # git push
```

## How it works

- **Files are truth** — notes live as `.md` files on disk. You can edit them with any editor.
- **SQLite is an index** — fast search, tags, backlinks. Delete it and it rebuilds from files.
- **Git sync** — your vault is just a folder. Push to a repo to sync between machines.
- **Tags** — write `#tag` anywhere in a note. Tags connect notes automatically.
- **Local-first** — no accounts, no cloud, no telemetry. Everything stays on your machine.

## Vault structure

```
~/.mem-vault/
  mem.json              # vault config
  notes/
    2026/
      2026-03-11-my-note.md
    inbox/
    journal/
  .mem/
    index.sqlite        # rebuildable index
```

## Development

```bash
# CLI
cargo run -p mem-cli -- note list

# Desktop
cd apps/desktop
npm install
npx tauri dev

# Build all (CLI + Desktop)
./scripts/build.sh 0.1.0
```

### Stack

- **Core**: Rust workspace (domain, storage, index, parser, sync)
- **Desktop**: Tauri + SolidJS + Tiptap
- **Search**: SQLite FTS5
- **Sync**: Git CLI wrapper

## Auto-updates

Desktop app checks for updates on GitHub Releases. To enable:

1. Generate signing keys: `npx tauri signer generate -w ~/.tauri/mem.key`
2. Set `TAURI_SIGNING_PRIVATE_KEY` in GitHub repo secrets
3. Add the public key to `tauri.conf.json` → `plugins.updater.pubkey`
4. Update the `endpoints` URL with your GitHub username

## Release

Tag and push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions builds for macOS (ARM + Intel), Linux, and Windows, then creates a release.

## License

MIT
