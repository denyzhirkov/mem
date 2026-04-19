use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use dialoguer::Select;
use std::io::Read as _;
use std::path::PathBuf;

use mem_core::types::{ListParams, RelatedParams, SearchParams, ShowParams, UpdateParams};
use mem_core::{index as core_index, note, search, sync as core_sync, tags, vault};
use mem_storage::vault::{default_vault_path, is_valid_vault};

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
    Search { query: String },
    /// Git Sync operations
    Sync {
        #[command(subcommand)]
        command: SyncCommands,
    },
    /// Rebuild SQLite index from markdown files
    Index {
        #[command(subcommand)]
        command: IndexCommands,
    },
    /// Tag operations
    Tags {
        #[command(subcommand)]
        command: TagsCommands,
    },
}

#[derive(Subcommand, Debug)]
enum TagsCommands {
    /// List all tags with note counts
    List,
}

#[derive(Subcommand, Debug)]
enum SyncCommands {
    /// Show git status
    Status,
    /// Commit all changes
    Commit { message: String },
    /// Pull changes from remote
    Pull,
    /// Push changes to remote
    Push,
}

#[derive(Args, Debug)]
struct NoteArgs {
    #[command(subcommand)]
    command: NoteCommands,
}

#[derive(Subcommand, Debug)]
enum NoteCommands {
    /// Create a new note
    New {
        title: String,
        /// Initial body content
        #[arg(short, long)]
        body: Option<String>,
        /// Read body from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// List all notes
    List {
        /// Filter by tag (with or without #)
        #[arg(short, long)]
        tag: Option<String>,
        /// Include archived notes
        #[arg(long)]
        archived: bool,
    },
    /// Show note raw content
    Show { id_or_slug: String },
    /// Update an existing note
    Update {
        id_or_slug: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        body: Option<String>,
        #[arg(short, long)]
        append: Option<String>,
        /// Read new body from stdin
        #[arg(long)]
        stdin: bool,
        #[arg(long)]
        archive: bool,
        #[arg(long)]
        unarchive: bool,
    },
    /// Delete a note
    Delete { id_or_slug: String },
    /// Show notes related to the given one (by shared tags)
    Related {
        id_or_slug: String,
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
    },
}

#[derive(Subcommand, Debug)]
enum IndexCommands {
    /// Rebuild index from scratch
    Rebuild,
}

/// Resolve vault path interactively (CLI-only). For non-interactive use,
/// `mem_core::vault::resolve_vault` is preferred.
fn resolve_vault_interactive() -> Result<PathBuf> {
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
                let info = vault::init(&current_dir, None)?;
                println!(
                    "Initialized vault '{}' at {}",
                    info.vault_name,
                    info.path.display()
                );
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
            0 => current_dir,
            1 => global_path,
            _ => unreachable!(),
        };

        let info = vault::init(&target, None)?;
        println!(
            "Initialized vault '{}' at {}",
            info.vault_name,
            info.path.display()
        );
        Ok(target)
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, name } => {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let p = path.unwrap_or(current_dir);
            let info = vault::init(&p, name)?;
            println!(
                "Initialized vault '{}' at {}",
                info.vault_name,
                info.path.display()
            );
            println!("Created .mem internal directory and notes/ folder.");
        }

        Commands::Note(note_args) => {
            let v = resolve_vault_interactive()?;
            match note_args.command {
                NoteCommands::New { title, body, stdin } => {
                    let body = if stdin {
                        let mut buf = String::new();
                        std::io::stdin().read_to_string(&mut buf)?;
                        Some(buf)
                    } else {
                        body
                    };
                    let r = note::new(&v, &title, body.as_deref())?;
                    println!("Created new note '{}' at {}", r.title, r.path);
                    println!("ID: {}", r.id);
                }
                NoteCommands::List { tag, archived } => {
                    let rows = note::list(
                        &v,
                        ListParams {
                            include_archived: archived,
                            tag,
                            ..Default::default()
                        },
                    )?;
                    println!("{: <26} | {: <30} | {}", "ID", "SLUG", "TITLE");
                    println!("{:-<26}-+-{:-<30}-+-{:-<30}", "", "", "");
                    for r in rows {
                        println!("{: <26} | {: <30} | {}", r.id, r.slug, r.title);
                    }
                }
                NoteCommands::Delete { id_or_slug } => {
                    note::delete(&v, &id_or_slug)?;
                    println!("Deleted note '{}'", id_or_slug);
                }
                NoteCommands::Related { id_or_slug, limit } => {
                    let rows = note::related(
                        &v,
                        RelatedParams {
                            id_or_slug,
                            limit: Some(limit),
                        },
                    )?;
                    if rows.is_empty() {
                        println!("No related notes.");
                    } else {
                        for r in rows {
                            let tag_str = if r.tags.is_empty() {
                                String::new()
                            } else {
                                format!("  [{}]", r.tags.join(", "))
                            };
                            println!("{}  {}{}", r.slug, r.title, tag_str);
                        }
                    }
                }
                NoteCommands::Show { id_or_slug } => {
                    let view = note::show(
                        &v,
                        ShowParams {
                            id_or_slug,
                            raw: true,
                        },
                    )?;
                    println!("{}", view.raw.unwrap_or_default());
                }
                NoteCommands::Update {
                    id_or_slug,
                    title,
                    body,
                    append,
                    stdin,
                    archive,
                    unarchive,
                } => {
                    let body = if stdin {
                        let mut buf = String::new();
                        std::io::stdin().read_to_string(&mut buf)?;
                        Some(buf)
                    } else {
                        body
                    };

                    let r = note::update(
                        &v,
                        UpdateParams {
                            id_or_slug,
                            title,
                            body,
                            append,
                            archive,
                            unarchive,
                        },
                    )?;
                    println!("Updated note '{}'", r.title);
                }
            }
        }

        Commands::Search { query } => {
            let v = resolve_vault_interactive()?;
            let hits = search::run(
                &v,
                SearchParams {
                    query,
                    limit: None,
                },
            )?;
            if hits.is_empty() {
                println!("No results found.");
            } else {
                for h in hits {
                    println!(
                        "[{}] {} (match: {}, score: {})",
                        h.id, h.title, h.match_kind, h.score
                    );
                }
            }
        }

        Commands::Sync { command } => {
            let v = resolve_vault_interactive()?;
            match command {
                SyncCommands::Status => {
                    let s = core_sync::status(&v)?;
                    if s.clean {
                        println!("Tree is clean.");
                    } else {
                        if s.conflicts {
                            println!("WARNING: Conflicts detected in current tree!");
                        }
                        println!("{}", s.status);
                    }
                }
                SyncCommands::Commit { message } => {
                    core_sync::commit(&v, &message)?;
                    println!("Committed changes.");
                }
                SyncCommands::Pull => {
                    println!("{}", core_sync::pull(&v)?);
                }
                SyncCommands::Push => {
                    println!("{}", core_sync::push(&v)?);
                }
            }
        }

        Commands::Index { command } => match command {
            IndexCommands::Rebuild => {
                let v = resolve_vault_interactive()?;
                core_index::rebuild(&v)?;
                println!("Index rebuild is not fully implemented in MVP yet, but SQLite is ready.");
            }
        },

        Commands::Tags { command } => match command {
            TagsCommands::List => {
                let v = resolve_vault_interactive()?;
                let rows = tags::list(&v)?;
                if rows.is_empty() {
                    println!("No tags.");
                } else {
                    for t in rows {
                        println!("{: >4}  #{}", t.note_count, t.name);
                    }
                }
            }
        },
    }

    Ok(())
}
