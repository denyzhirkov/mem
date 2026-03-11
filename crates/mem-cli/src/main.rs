use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use anyhow::{Context, Result};
use chrono::Utc;

use mem_domain::{Note, NoteId};
use mem_storage::vault::init_vault;
use mem_storage::config::load_config;
use mem_storage::storage::{write_note, read_note_raw};
use mem_index::db::IndexDb;

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match cli.command {
        Commands::Init { path, name } => {
            let p = path.unwrap_or(current_dir);
            let config = init_vault(&p, name)
                .with_context(|| format!("Failed to initialize vault at {}", p.display()))?;
            
            // Also initialize the db
            let _db = open_db(&p)?;
            
            println!("Initialized vault '{}' at {}", config.vault_name, p.display());
            println!("Created .mem internal directory and notes/ folder.");
        }
        Commands::Note(note_args) => {
            let config = load_config(&current_dir).context("Not a mem vault (missing mem.json)")?;
            let mut db = open_db(&current_dir)?;
            
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
                        content_hash: "empty".to_string(), // MVP: simplified static hash or skip
                        archived: false,
                    };
                    
                    let content = format!("# {}\n\n", title);
                    let path = write_note(&current_dir, &config, &mut note, &content)?;
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
            }
        }
        Commands::Search { query } => {
            let config = load_config(&current_dir).context("Not a mem vault (missing mem.json)")?;
            let db = open_db(&current_dir)?;
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
            let _config = load_config(&current_dir).context("Not a mem vault (missing mem.json)")?;
            match command {
                SyncCommands::Status => {
                    let status = mem_sync::sync_status(&current_dir)?;
                    if status.is_empty() {
                        println!("Tree is clean.");
                    } else {
                        let has_conflicts = mem_sync::check_conflicts(&current_dir)?;
                        if has_conflicts {
                            println!("WARNING: Conflicts detected in current tree!");
                        }
                        println!("{}", status);
                    }
                }
                SyncCommands::Commit { message } => {
                    mem_sync::commit_all(&current_dir, &message)?;
                    println!("Committed changes.");
                }
                SyncCommands::Pull => {
                    let out = mem_sync::pull(&current_dir)?;
                    println!("{}", out);
                }
                SyncCommands::Push => {
                    let out = mem_sync::push(&current_dir)?;
                    println!("{}", out);
                }
            }
        }
        Commands::Index { command } => {
            match command {
                IndexCommands::Rebuild => {
                    let _config = load_config(&current_dir).context("Not a mem vault (missing mem.json)")?;
                    println!("Index rebuild is not fully implemented in MVP yet, but SQLite is ready.");
                }
            }
        }
    }
    
    Ok(())
}
