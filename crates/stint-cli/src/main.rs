//! Entry point for the Stint CLI.

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use stint_core::models::project::{Project, ProjectStatus};
use stint_core::models::types::ProjectId;
use stint_core::storage::sqlite::SqliteStorage;
use stint_core::storage::Storage;
use time::OffsetDateTime;

/// Parses a dollar amount string into integer cents using exact string math.
///
/// Accepts formats like "150", "150.00", "19.99". Rejects negative values
/// and malformed input. Returns cents as i64.
fn parse_cents(s: &str) -> Result<i64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("rate cannot be empty".to_string());
    }
    if s.starts_with('-') {
        return Err("rate cannot be negative".to_string());
    }

    let (dollars_str, cents_str) = if let Some((d, c)) = s.split_once('.') {
        (d, c)
    } else {
        (s, "")
    };

    let dollars: i64 = dollars_str
        .parse()
        .map_err(|_| format!("invalid rate: '{s}'"))?;

    let cents: i64 = match cents_str.len() {
        0 => 0,
        1 => {
            cents_str
                .parse::<i64>()
                .map_err(|_| format!("invalid rate: '{s}'"))?
                * 10
        }
        2 => cents_str
            .parse()
            .map_err(|_| format!("invalid rate: '{s}'"))?,
        _ => return Err(format!("rate has too many decimal places: '{s}'")),
    };

    dollars
        .checked_mul(100)
        .and_then(|d| d.checked_add(cents))
        .ok_or_else(|| format!("invalid rate: '{s}'"))
}

/// Terminal-native project time tracker.
#[derive(Parser)]
#[command(name = "stint", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Top-level commands.
#[derive(Subcommand)]
enum Commands {
    /// Manage projects.
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
}

/// Project subcommands.
#[derive(Subcommand)]
enum ProjectCommands {
    /// Register a new project.
    Add {
        /// Project name (must be unique).
        name: String,

        /// Directory path for this project.
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Comma-separated tags.
        #[arg(short, long)]
        tags: Option<String>,

        /// Hourly rate in dollars (e.g., 150.00).
        #[arg(long, value_parser = parse_cents)]
        rate: Option<i64>,
    },

    /// List registered projects.
    List {
        /// Show all projects including archived.
        #[arg(short, long)]
        all: bool,
    },
}

/// Opens the database, exiting on failure.
fn open_storage() -> SqliteStorage {
    let path = SqliteStorage::default_path();
    match SqliteStorage::open(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to open database at {}: {e}", path.display());
            process::exit(1);
        }
    }
}

/// Handles the `project add` command.
fn project_add(name: String, path: Option<PathBuf>, tags: Option<String>, rate: Option<i64>) {
    // Validate path before opening the database
    let paths = match path {
        Some(p) => {
            let resolved = match p.canonicalize() {
                Ok(abs) => abs,
                Err(e) => {
                    eprintln!("error: invalid path '{}': {e}", p.display());
                    process::exit(1);
                }
            };
            vec![resolved]
        }
        None => vec![],
    };

    let storage = open_storage();

    let parsed_tags = tags
        .map(|t| stint_core::models::tag::parse_tags(&t))
        .unwrap_or_default();

    let hourly_rate_cents = rate;

    let now = OffsetDateTime::now_utc();
    let project = Project {
        id: ProjectId::new(),
        name: name.clone(),
        paths,
        tags: parsed_tags,
        hourly_rate_cents,
        status: ProjectStatus::Active,
        created_at: now,
        updated_at: now,
    };

    match storage.create_project(&project) {
        Ok(()) => println!("Created project '{name}'"),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `project list` command.
fn project_list(all: bool) {
    let storage = open_storage();

    let status_filter = if all {
        None
    } else {
        Some(ProjectStatus::Active)
    };

    let projects = match storage.list_projects(status_filter) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if projects.is_empty() {
        if !all {
            // Check if there are archived projects the user isn't seeing
            let has_archived = storage
                .list_projects(Some(ProjectStatus::Archived))
                .map(|p| !p.is_empty())
                .unwrap_or(false);
            if has_archived {
                println!("No active projects. Use 'stint project list --all' to include archived.");
                return;
            }
        }
        println!("No projects registered. Use 'stint project add' to create one.");
        return;
    }

    for project in &projects {
        let paths_str = project
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let rate_str = match project.hourly_rate_cents {
            Some(cents) => format!("${}.{:02}/hr", cents / 100, cents % 100),
            None => String::new(),
        };

        let tags_str = if project.tags.is_empty() {
            String::new()
        } else {
            format!("[{}]", project.tags.join(", "))
        };

        let status_str = if project.status == ProjectStatus::Archived {
            " (archived)"
        } else {
            ""
        };

        let mut parts = vec![project.name.clone()];
        if !paths_str.is_empty() {
            parts.push(paths_str);
        }
        if !rate_str.is_empty() {
            parts.push(rate_str);
        }
        if !tags_str.is_empty() {
            parts.push(tags_str);
        }

        println!("  {}{status_str}", parts.join("  "));
    }
}

/// Entry point.
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Project { command } => match command {
            ProjectCommands::Add {
                name,
                path,
                tags,
                rate,
            } => project_add(name, path, tags, rate),
            ProjectCommands::List { all } => project_list(all),
        },
    }
}
