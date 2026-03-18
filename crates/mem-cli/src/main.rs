use clap::{Args, Parser, Subcommand};
use std::io::Read as _;
use std::path::PathBuf;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use dialoguer::Select;

use mem_domain::{Note, NoteId};
use mem_storage::vault::{init_vault, is_valid_vault, default_vault_path};
use mem_storage::config::load_config;
use mem_storage::storage::{write_note, read_note_raw, read_note_content};
use mem_index::db::IndexDb;
use mem_parser::{extract_tags, extract_links};

#[derive(Parser, Debug)]
#[command(name = "mem")]
#[command(about = "Minimal local-first personal knowledge management")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new mem vault
    Init {
        /// Optional path to initialize (defaults to current directory)
        path: Option<PathBuf>,
        
        /// Optional vault name
        #[arg(short, long)]
        name: Option<String>,
    },
    
    /// Note operations
    Note(NoteArgs),
    
    /// Search notes
    Search {
        query: String,
    },
    
    /// Git Sync operations
    Sync {
        #[command(subcommand)]
        command: SyncCommands,
    },
    
    /// Rebuild SQLite index from markdown files
    Index {
        #[command(subcommand)]
        command: IndexCommands,
    }
}

#[derive(Subcommand, Debug)]
enum SyncCommands {
    /// Show git status
    Status,
    /// Commit all changes
    Commit {
        message: String,
    },
    /// Pull changes from remote
    Pull,
    /// Push changes to remote
    Push,
}

#[derive(Args, Debug)]
struct NoteArgs {
// ...
// Actually it's better to just write the whole file to avoid line-matching errors if I get it wrong.
    #[command(subcommand)]
    command: NoteCommands,
}

#[derive(Subcommand, Debug)]
enum NoteCommands {
    /// Create a new note
    New {
        title: String,
    },
    /// List all notes
    List,
    /// Show note raw content
    Show {
        id_or_slug: String,
    },
    /// Update an existing note
    Update {
        /// Note ID or slug
        id_or_slug: String,
        /// Set a new title
        #[arg(short, long)]
        title: Option<String>,
        /// Replace body content
        #[arg(short, long)]
        body: Option<String>,
        /// Append text to the end
        #[arg(short, long)]
        append: Option<String>,
        /// Read new body from stdin
        #[arg(long)]
        stdin: bool,
        /// Archive the note
        #[arg(long)]
        archive: bool,
        /// Unarchive the note
        #[arg(long)]
        unarchive: bool,
    },
}

#[derive(Subcommand, Debug)]
enum IndexCommands {
    /// Rebuild index from scratch
    Rebuild,
}

fn generate_slug(title: &str) -> String {
    title.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn open_db(vault_path: &PathBuf) -> Result<IndexDb> {
    let db_path = vault_path.join(".mem").join("index.sqlite");
    IndexDb::open(&db_path).context("Failed to open index database")
}

/// Resolve vault path: current dir if it's a vault, otherwise ask the user.
fn resolve_vault_path() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if is_valid_vault(&current_dir) {
        return Ok(current_dir);
    }

    let global_path = default_vault_path();
    let global_exists = is_valid_vault(&global_path);

    if global_exists {
        let items = vec![
            format!("Use global vault ({})", global_path.display()),
            format!("Create a new vault here ({})", current_dir.display()),
        ];

        println!("No vault found in the current directory.");
        let selection = Select::new()
            .with_prompt("What would you like to do?")
            .items(&items)
            .default(0)
            .interact()
            .context("Failed to read selection")?;

        match selection {
            0 => Ok(global_path),
            1 => {
                let config = init_vault(&current_dir, None)
                    .with_context(|| format!("Failed to initialize vault at {}", current_dir.display()))?;
                let _db = open_db(&current_dir)?;
                println!("Initialized vault '{}' at {}", config.vault_name, current_dir.display());
                Ok(current_dir)
            }
            _ => unreachable!(),
        }
    } else {
        let items = vec![
            format!("Create a new vault here ({})", current_dir.display()),
            format!("Create global vault ({})", global_path.display()),
        ];

        println!("No vault found anywhere.");
        let selection = Select::new()
            .with_prompt("Where would you like to create a vault?")
            .items(&items)
            .default(0)
            .interact()
            .context("Failed to read selection")?;

        let target = match selection {
            0 => &current_dir,
            1 => &global_path,
            _ => unreachable!(),
        };

        let config = init_vault(target, None)
            .with_context(|| format!("Failed to initialize vault at {}", target.display()))?;
        let _db = open_db(target)?;
        println!("Initialized vault '{}' at {}", config.vault_name, target.display());
        Ok(target.clone())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, name } => {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let p = path.unwrap_or(current_dir);
            let config = init_vault(&p, name)
                .with_context(|| format!("Failed to initialize vault at {}", p.display()))?;

            let _db = open_db(&p)?;

            println!("Initialized vault '{}' at {}", config.vault_name, p.display());
            println!("Created .mem internal directory and notes/ folder.");
        }
        Commands::Note(note_args) => {
            let vault_path = resolve_vault_path()?;
            let config = load_config(&vault_path).context("Not a mem vault (missing mem.json)")?;
            let mut db = open_db(&vault_path)?;

            match note_args.command {
                NoteCommands::New { title } => {
                    let mut note = Note {
                        id: NoteId::default(),
                        title: title.clone(),
                        slug: generate_slug(&title),
                        path: String::new(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        tags: vec![],
                        outgoing_links: vec![],
                        content_hash: "empty".to_string(),
                        archived: false,
                    };

                    let content = format!("# {}\n\n", title);
                    let path = write_note(&vault_path, &config, &mut note, &content)?;
                    db.upsert_note(&note)?;

                    println!("Created new note '{}' at {}", title, path.display());
                    println!("ID: {}", note.id.0);
                }
                NoteCommands::List => {
                    let mut stmt = db.conn().prepare("SELECT id, slug, title FROM notes ORDER BY updated_at DESC")?;
                    let iter = stmt.query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                    })?;
                    println!("{: <26} | {: <30} | {}", "ID", "SLUG", "TITLE");
                    println!("{:-<26}-+-{:-<30}-+-{:-<30}", "", "", "");
                    for r in iter {
                        let (id, slug, title) = r?;
                        println!("{: <26} | {: <30} | {}", id, slug, title);
                    }
                }
                NoteCommands::Show { id_or_slug } => {
                    let mut stmt = db.conn().prepare("SELECT path FROM notes WHERE id = ?1 OR slug = ?1 LIMIT 1")?;
                    let path: String = stmt.query_row([&id_or_slug], |row| row.get(0)).context("Note not found")?;

                    let content = read_note_raw(&PathBuf::from(&path))?;
                    println!("{}", content);
                }
                NoteCommands::Update { id_or_slug, title, body, append, stdin, archive, unarchive } => {
                    if title.is_none() && body.is_none() && append.is_none() && !stdin && !archive && !unarchive {
                        bail!("Nothing to update. Use --title, --body, --append, --stdin, --archive, or --unarchive.");
                    }

                    let mut stmt = db.conn().prepare(
                        "SELECT id, title, slug, path, created_at, updated_at, content_hash, archived FROM notes WHERE id = ?1 OR slug = ?1 LIMIT 1"
                    )?;
                    let mut note: Note = stmt.query_row([&id_or_slug], |row| {
                        let created_at_str: String = row.get(4)?;
                        let updated_at_str: String = row.get(5)?;
                        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());
                        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
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
                    }).context("Note not found")?;
                    drop(stmt);

                    // Read current content (without frontmatter)
                    let mut content = read_note_content(&PathBuf::from(&note.path))?;

                    if let Some(new_title) = &title {
                        note.title = new_title.clone();
                        note.slug = generate_slug(new_title);
                        // Replace H1 heading if present
                        if let Some(first_line_end) = content.find('\n') {
                            if content.starts_with("# ") {
                                content = format!("# {}{}", new_title, &content[first_line_end..]);
                            }
                        } else if content.starts_with("# ") {
                            content = format!("# {}\n\n", new_title);
                        }
                    }

                    if let Some(new_body) = body {
                        // Keep H1 line, replace the rest
                        if let Some(first_line_end) = content.find('\n') {
                            if content.starts_with("# ") {
                                content = format!("{}\n\n{}", &content[..first_line_end], new_body);
                            } else {
                                content = new_body;
                            }
                        } else {
                            content = new_body;
                        }
                    } else if stdin {
                        let mut stdin_content = String::new();
                        std::io::stdin().read_to_string(&mut stdin_content)?;
                        if let Some(first_line_end) = content.find('\n') {
                            if content.starts_with("# ") {
                                content = format!("{}\n\n{}", &content[..first_line_end], stdin_content);
                            } else {
                                content = stdin_content;
                            }
                        } else {
                            content = stdin_content;
                        }
                    }

                    if let Some(extra) = append {
                        if !content.ends_with('\n') {
                            content.push('\n');
                        }
                        content.push_str(&extra);
                        content.push('\n');
                    }

                    if archive {
                        note.archived = true;
                    }
                    if unarchive {
                        note.archived = false;
                    }

                    note.tags = extract_tags(&content);
                    note.outgoing_links = extract_links(&content);

                    write_note(&vault_path, &config, &mut note, &content)?;
                    db.upsert_note_with_content(&note, Some(&content))?;

                    println!("Updated note '{}'", note.title);
                }
            }
        }
        Commands::Search { query } => {
            let vault_path = resolve_vault_path()?;
            let _config = load_config(&vault_path).context("Not a mem vault (missing mem.json)")?;
            let db = open_db(&vault_path)?;
            let results = db.search(&query)?;

            if results.is_empty() {
                println!("No results found.");
            } else {
                for r in results {
                    println!("[{}] {} (match: {}, score: {})", r.note_id.0, r.title, r.match_kind, r.score);
                }
            }
        }
        Commands::Sync { command } => {
            let vault_path = resolve_vault_path()?;
            let _config = load_config(&vault_path).context("Not a mem vault (missing mem.json)")?;
            match command {
                SyncCommands::Status => {
                    let status = mem_sync::sync_status(&vault_path)?;
                    if status.is_empty() {
                        println!("Tree is clean.");
                    } else {
                        let has_conflicts = mem_sync::check_conflicts(&vault_path)?;
                        if has_conflicts {
                            println!("WARNING: Conflicts detected in current tree!");
                        }
                        println!("{}", status);
                    }
                }
                SyncCommands::Commit { message } => {
                    mem_sync::commit_all(&vault_path, &message)?;
                    println!("Committed changes.");
                }
                SyncCommands::Pull => {
                    let out = mem_sync::pull(&vault_path)?;
                    println!("{}", out);
                }
                SyncCommands::Push => {
                    let out = mem_sync::push(&vault_path)?;
                    println!("{}", out);
                }
            }
        }
        Commands::Index { command } => {
            match command {
                IndexCommands::Rebuild => {
                    let vault_path = resolve_vault_path()?;
                    let _config = load_config(&vault_path).context("Not a mem vault (missing mem.json)")?;
                    println!("Index rebuild is not fully implemented in MVP yet, but SQLite is ready.");
                }
            }
        }
    }

    Ok(())
}
