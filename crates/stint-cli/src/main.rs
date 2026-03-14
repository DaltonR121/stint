//! Entry point for the Stint CLI.

mod tui;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use stint_core::dateparse::parse_date;
use stint_core::duration::{format_duration_human, parse_duration};
use stint_core::hook;
use stint_core::models::entry::EntryFilter;
use stint_core::models::project::{Project, ProjectStatus};
use stint_core::models::types::ProjectId;
use stint_core::report::{format_report, generate_report, GroupBy, ReportFormat};
use stint_core::service::StintService;
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

/// Parses a duration string for clap.
fn parse_duration_arg(s: &str) -> Result<i64, String> {
    parse_duration(s)
}

/// Returns the current local time, falling back to UTC if local time is unavailable.
fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
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
    /// Start tracking time for a project.
    Start {
        /// Project name.
        project: String,
    },

    /// Stop the currently running timer.
    Stop,

    /// Show what's currently being tracked.
    Status,

    /// Quick summary of today's and this week's tracked time.
    Summary,

    /// Edit the most recent time entry.
    Edit {
        /// New duration (e.g., "2h30m"). Replaces the existing duration.
        #[arg(short, long, value_parser = parse_duration_arg)]
        duration: Option<i64>,

        /// New notes. Replaces existing notes.
        #[arg(short, long)]
        notes: Option<String>,
    },

    /// Delete the most recent time entry.
    #[command(name = "delete-entry")]
    DeleteEntry {
        /// Skip confirmation prompt.
        #[arg(long)]
        force: bool,
    },

    /// Add time retroactively.
    Add {
        /// Project name.
        project: String,

        /// Duration (e.g., "2h30m", "45m", "1h").
        #[arg(value_parser = parse_duration_arg)]
        duration: i64,

        /// Date for the entry (e.g., "today", "yesterday", "2026-01-15").
        #[arg(short, long)]
        date: Option<String>,

        /// Notes for the entry.
        #[arg(short, long)]
        notes: Option<String>,
    },

    /// View time entries.
    Log {
        /// Start date filter (e.g., "today", "last monday", "2026-01-01").
        #[arg(long)]
        from: Option<String>,

        /// End date filter.
        #[arg(long)]
        to: Option<String>,

        /// Filter by project name.
        #[arg(short, long)]
        project: Option<String>,

        /// Filter by tag (can be specified multiple times).
        #[arg(short, long)]
        tag: Vec<String>,
    },

    /// Generate grouped time reports.
    Report {
        /// Group results by "project" or "tag".
        #[arg(long, default_value = "project")]
        group_by: String,

        /// Output format: "table", "markdown", "csv", or "json".
        #[arg(long, default_value = "table")]
        format: String,

        /// Start date filter.
        #[arg(long)]
        from: Option<String>,

        /// End date filter.
        #[arg(long)]
        to: Option<String>,

        /// Filter by project name.
        #[arg(short, long)]
        project: Option<String>,

        /// Filter by tag (can be specified multiple times).
        #[arg(short, long)]
        tag: Vec<String>,
    },

    /// Import time entries from a CSV file.
    Import {
        /// Path to the CSV file.
        file: PathBuf,
    },

    /// Open the interactive dashboard.
    #[command(alias = "tui")]
    Dashboard,

    /// Start the local HTTP API server.
    Serve {
        /// Port to listen on.
        #[arg(short, long, default_value = "7653")]
        port: u16,
    },

    /// Manage projects.
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    /// Output shell hook script for eval.
    Shell {
        /// Shell type: bash, zsh, or fish.
        shell: String,
    },

    /// Add the shell hook to your shell config file.
    Init {
        /// Shell type: bash, zsh, or fish.
        shell: String,
    },

    /// Internal: called by shell hooks on every prompt render.
    #[command(name = "_hook", hide = true)]
    Hook {
        /// Current working directory.
        #[arg(long)]
        cwd: PathBuf,

        /// Shell PID.
        #[arg(long)]
        pid: u32,

        /// Shell type (bash, zsh, fish).
        #[arg(long)]
        shell: Option<String>,

        /// Signal that the shell is exiting.
        #[arg(long)]
        exit: bool,
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

    /// Archive a project (hide from default listings).
    Archive {
        /// Project name.
        name: String,
    },

    /// Delete a project and all its time entries.
    Delete {
        /// Project name.
        name: String,

        /// Skip confirmation prompt.
        #[arg(long)]
        force: bool,
    },

    /// Ignore a directory for auto-discovery (prevents auto-tracking).
    Ignore {
        /// Directory path to ignore.
        path: PathBuf,
    },

    /// Remove a directory from the ignore list.
    Unignore {
        /// Directory path to unignore.
        path: PathBuf,
    },
}

/// Opens the database and creates a service, exiting on failure.
fn open_service() -> StintService<SqliteStorage> {
    let path = SqliteStorage::default_path();
    let storage = match SqliteStorage::open(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to open database at {}: {e}", path.display());
            process::exit(1);
        }
    };
    StintService::new(storage)
}

/// Opens raw storage for operations that don't need the service layer.
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

/// Builds an EntryFilter from common CLI args.
fn build_filter(
    service: &StintService<SqliteStorage>,
    from: &Option<String>,
    to: &Option<String>,
    project: &Option<String>,
    tags: &[String],
) -> EntryFilter {
    let now = now_local();

    let from_dt = from.as_ref().map(|s| match parse_date(s, now) {
        Ok(dt) => dt,
        Err(e) => {
            eprintln!("error: --from: {e}");
            process::exit(1);
        }
    });

    let to_dt = to.as_ref().map(|s| match parse_date(s, now) {
        Ok(dt) => dt + time::Duration::days(1), // inclusive end date
        Err(e) => {
            eprintln!("error: --to: {e}");
            process::exit(1);
        }
    });

    let project_id = project
        .as_ref()
        .map(|name| match service.resolve_project_id(name) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        });

    EntryFilter {
        project_id,
        from: from_dt,
        to: to_dt,
        tags: tags.to_vec(),
        source: None,
    }
}

// --- Command handlers ---

/// Handles the `start` command.
fn cmd_start(project: String) {
    let service = open_service();
    match service.start_timer(&project) {
        Ok((_, proj)) => println!("Started timer for '{}'", proj.name),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `stop` command.
fn cmd_stop() {
    let service = open_service();
    match service.stop_timer() {
        Ok((entry, project)) => {
            let duration = entry.duration_secs.unwrap_or(0);
            println!(
                "Stopped '{}' after {}",
                project.name,
                format_duration_human(duration)
            );
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `status` command.
fn cmd_status() {
    let service = open_service();
    match service.get_status() {
        Ok(Some((entry, project))) => {
            let elapsed = (OffsetDateTime::now_utc() - entry.start).whole_seconds();
            println!(
                "Tracking '{}' for {}",
                project.name,
                format_duration_human(elapsed)
            );
        }
        Ok(None) => println!("No timer running."),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `summary` command — quick overview of today and this week.
fn cmd_summary() {
    let service = open_service();
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());

    // Today
    let today_start = now.replace_time(time::Time::MIDNIGHT);
    let today_filter = stint_core::models::entry::EntryFilter {
        from: Some(today_start),
        ..Default::default()
    };
    let today_entries = match service.get_entries(&today_filter) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };
    let today_secs: i64 = today_entries
        .iter()
        .map(|(e, _)| e.computed_duration_secs().unwrap_or(0))
        .sum();
    let today_count = today_entries.len();

    // This week (Monday to now)
    let weekday = now.weekday().number_days_from_monday();
    let week_start = today_start - time::Duration::days(weekday as i64);
    let week_filter = stint_core::models::entry::EntryFilter {
        from: Some(week_start),
        ..Default::default()
    };
    let week_entries = match service.get_entries(&week_filter) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };
    let week_secs: i64 = week_entries
        .iter()
        .map(|(e, _)| e.computed_duration_secs().unwrap_or(0))
        .sum();
    let week_count = week_entries.len();

    // Currently tracking
    let status = match service.get_status() {
        Ok(Some((entry, project))) => {
            let elapsed = (OffsetDateTime::now_utc() - entry.start).whole_seconds();
            format!(
                "Tracking '{}' for {}",
                project.name,
                format_duration_human(elapsed)
            )
        }
        Ok(None) => "Idle".to_string(),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    println!("  {status}");
    println!(
        "  Today: {}  ({} entries)",
        format_duration_human(today_secs),
        today_count
    );
    println!(
        "  Week:  {}  ({} entries)",
        format_duration_human(week_secs),
        week_count
    );
}

/// Handles the `edit` command — modifies the most recent entry.
fn cmd_edit(duration: Option<i64>, notes: Option<String>) {
    let service = open_service();
    let (mut entry, project) = match service.get_last_entry() {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            println!("No entries to edit.");
            return;
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let mut changed = false;

    if let Some(dur) = duration {
        entry.duration_secs = Some(dur);
        entry.end = Some(entry.start + time::Duration::seconds(dur));
        changed = true;
    }

    if let Some(n) = notes {
        entry.notes = if n.is_empty() { None } else { Some(n) };
        changed = true;
    }

    if !changed {
        println!("Nothing to change. Use --duration or --notes.");
        return;
    }

    entry.updated_at = OffsetDateTime::now_utc();
    match service.update_entry(&entry) {
        Ok(()) => {
            let dur_str = entry
                .duration_secs
                .map(format_duration_human)
                .unwrap_or_else(|| "running".to_string());
            println!("Updated entry: '{}' {}", project.name, dur_str);
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `delete-entry` command — deletes the most recent entry.
fn cmd_delete_entry(force: bool) {
    let service = open_service();
    let (entry, project) = match service.get_last_entry() {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            println!("No entries to delete.");
            return;
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let dur_str = entry
        .computed_duration_secs()
        .map(format_duration_human)
        .unwrap_or_else(|| "running".to_string());

    if !force {
        print!(
            "Delete entry: '{}' {} ({})? [y/N] ",
            project.name,
            dur_str,
            entry.start.date()
        );
        if let Err(e) = io::stdout().flush() {
            eprintln!("error: failed to flush stdout: {e}");
            process::exit(1);
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("Cancelled.");
                return;
            }
            Err(e) => {
                eprintln!("error: failed to read input: {e}");
                process::exit(1);
            }
            Ok(_) => {}
        }

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return;
        }
    }

    match service.delete_entry(&entry.id) {
        Ok(()) => println!("Deleted entry: '{}' {}", project.name, dur_str),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `add` command.
fn cmd_add(project: String, duration_secs: i64, date: Option<String>, notes: Option<String>) {
    let now = now_local();
    let date_dt = date.as_ref().map(|s| match parse_date(s, now) {
        Ok(dt) => dt,
        Err(e) => {
            eprintln!("error: --date: {e}");
            process::exit(1);
        }
    });

    let service = open_service();
    match service.add_time(&project, duration_secs, date_dt, notes.as_deref()) {
        Ok((_, proj)) => {
            let date_str = date.as_deref().unwrap_or("today");
            println!(
                "Added {} to '{}' ({})",
                format_duration_human(duration_secs),
                proj.name,
                date_str
            );
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `log` command.
fn cmd_log(from: Option<String>, to: Option<String>, project: Option<String>, tags: Vec<String>) {
    let service = open_service();
    let filter = build_filter(&service, &from, &to, &project, &tags);

    let entries = match service.get_entries(&filter) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if entries.is_empty() {
        println!("No entries found.");
        return;
    }

    for (entry, proj) in &entries {
        let date = entry.start.date();
        let duration = entry.computed_duration_secs().unwrap_or(0);
        let source = entry.source.as_str();
        let notes = entry.notes.as_deref().unwrap_or("");
        let running = if entry.is_running() { " (running)" } else { "" };

        println!(
            "  {}  {:<16}  {:>8}  {:<7}  {}{}",
            date,
            proj.name,
            format_duration_human(duration),
            source,
            notes,
            running,
        );
    }
}

/// Handles the `report` command.
fn cmd_report(
    group_by: String,
    format: String,
    from: Option<String>,
    to: Option<String>,
    project: Option<String>,
    tags: Vec<String>,
) {
    let group = match GroupBy::from_str_value(&group_by) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };
    let fmt = match ReportFormat::from_str_value(&format) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let service = open_service();
    let filter = build_filter(&service, &from, &to, &project, &tags);

    let entries = match service.get_entries(&filter) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let result = generate_report(&entries, &group);
    print!("{}", format_report(&result, &fmt));
}

/// Handles the `import` command.
fn cmd_import(file: PathBuf) {
    let storage = open_storage();
    match stint_core::import::import_csv(&storage, &file) {
        Ok(result) => {
            println!(
                "Imported {} entries ({} projects created, {} rows skipped)",
                result.entries_imported, result.projects_created, result.rows_skipped
            );
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `project add` command.
fn cmd_project_add(name: String, path: Option<PathBuf>, tags: Option<String>, rate: Option<i64>) {
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

    let now = OffsetDateTime::now_utc();
    let project = Project {
        id: ProjectId::new(),
        name: name.clone(),
        paths,
        tags: parsed_tags,
        hourly_rate_cents: rate,
        status: ProjectStatus::Active,
        source: stint_core::models::project::ProjectSource::Manual,
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
fn cmd_project_list(all: bool) {
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
            let has_archived = match storage.list_projects(Some(ProjectStatus::Archived)) {
                Ok(p) => !p.is_empty(),
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            };
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

/// Handles the `project archive` command.
fn cmd_project_archive(name: String) {
    let service = open_service();
    match service.archive_project(&name) {
        Ok(project) => println!("Archived project '{}'", project.name),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `project delete` command.
fn cmd_project_delete(name: String, force: bool) {
    if !force {
        print!("Delete project '{name}' and all its entries? [y/N] ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("error: failed to flush stdout: {e}");
            process::exit(1);
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("Cancelled.");
                return;
            }
            Err(e) => {
                eprintln!("error: failed to read input: {e}");
                process::exit(1);
            }
            Ok(_) => {}
        }

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return;
        }
    }

    let service = open_service();
    match service.delete_project(&name) {
        Ok(deleted_name) => println!("Deleted project '{deleted_name}'"),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `project ignore` command.
fn cmd_project_ignore(path: PathBuf) {
    let resolved = match path.canonicalize() {
        Ok(abs) => abs,
        Err(e) => {
            eprintln!("error: invalid path '{}': {e}", path.display());
            process::exit(1);
        }
    };

    let storage = open_storage();
    match storage.add_ignored_path(&resolved) {
        Ok(()) => println!("Ignoring '{}'", resolved.display()),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `project unignore` command.
fn cmd_project_unignore(path: PathBuf) {
    // Try canonicalize first, fall back to absolute path if the directory no longer exists
    let resolved = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&path)
        }
    });

    let storage = open_storage();
    match storage.remove_ignored_path(&resolved) {
        Ok(true) => println!("Removed '{}' from ignore list", resolved.display()),
        Ok(false) => println!("'{}' was not in the ignore list", resolved.display()),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

/// Handles the `_hook` command (called by shell hooks).
///
/// Must never call process::exit or print to stdout/stderr — the hook
/// must be invisible to the user's shell. Uses open_existing to skip
/// directory creation and migrations for <2ms performance.
/// Config is loaded only if the file exists to avoid unnecessary I/O.
fn cmd_hook(cwd: PathBuf, pid: u32, shell: Option<String>, exit: bool) {
    let path = SqliteStorage::default_path();
    let storage = match SqliteStorage::open_existing(&path) {
        Ok(s) => s,
        Err(_) => return, // Silently bail — DB doesn't exist yet or can't open
    };
    // Use default config in the hook hot path to avoid any filesystem I/O.
    // Users who need custom config can set STINT_IDLE_THRESHOLD env var as a
    // lightweight override without file reads.
    let mut config = stint_core::config::StintConfig::default();
    if let Ok(val) = std::env::var("STINT_IDLE_THRESHOLD") {
        if let Ok(secs) = val.parse::<i64>() {
            config.idle_threshold_secs = secs;
        }
    }
    if std::env::var("STINT_NO_DISCOVER").is_ok() {
        config.auto_discover = false;
    }
    if exit {
        let _ = hook::handle_hook_exit(&storage, pid, &config);
    } else {
        let _ = hook::handle_hook(&storage, pid, &cwd, shell.as_deref(), &config);
    }
}

/// Handles the `shell` command — outputs hook script for eval.
fn cmd_shell(shell: String) {
    let script = match shell.to_lowercase().as_str() {
        "bash" => {
            r#"_stint_hook() {
    stint _hook --cwd "$PWD" --pid $$ --shell bash
}
_stint_exit() {
    stint _hook --cwd "$PWD" --pid $$ --shell bash --exit
}
PROMPT_COMMAND="_stint_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
trap '_stint_exit' EXIT
"#
        }
        "zsh" => {
            r#"_stint_hook() {
    stint _hook --cwd "$PWD" --pid $$ --shell zsh
}
_stint_exit() {
    stint _hook --cwd "$PWD" --pid $$ --shell zsh --exit
}
precmd_functions+=(_stint_hook)
zshexit_functions+=(_stint_exit)
"#
        }
        "fish" => {
            r#"function _stint_hook --on-event fish_prompt
    stint _hook --cwd "$PWD" --pid %self --shell fish
end
function _stint_exit --on-event fish_exit
    stint _hook --cwd "$PWD" --pid %self --shell fish --exit
end
"#
        }
        _ => {
            eprintln!("error: unsupported shell '{shell}' (use bash, zsh, or fish)");
            process::exit(1);
        }
    };
    print!("{script}");
}

/// Handles the `init` command — appends the shell hook to the user's config file.
fn cmd_init(shell: String) {
    let (config_path, eval_line) = match shell.to_lowercase().as_str() {
        "bash" => {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            (home.join(".bashrc"), "eval \"$(stint shell bash)\"")
        }
        "zsh" => {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            (home.join(".zshrc"), "eval \"$(stint shell zsh)\"")
        }
        "fish" => {
            let config = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
            (
                config.join("fish").join("config.fish"),
                "stint shell fish | source",
            )
        }
        _ => {
            eprintln!("error: unsupported shell '{shell}' (use bash, zsh, or fish)");
            process::exit(1);
        }
    };

    // Check if already installed
    if config_path.exists() {
        let contents = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to read {}: {e}", config_path.display());
                process::exit(1);
            }
        };
        if contents.contains(eval_line) {
            println!("Stint hook already installed in {}", config_path.display());
            return;
        }
    }

    // Append the eval line
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: failed to open {}: {e}", config_path.display());
            process::exit(1);
        }
    };

    use std::io::Write as _;
    if let Err(e) = writeln!(file, "\n# Stint auto-tracking hook\n{eval_line}") {
        eprintln!("error: failed to write to {}: {e}", config_path.display());
        process::exit(1);
    }

    println!("Installed Stint hook in {}", config_path.display());
    println!(
        "Restart your shell or run: source {}",
        config_path.display()
    );
}

/// Entry point.
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Import { file } => cmd_import(file),
        Commands::Serve { port } => {
            if let Err(e) = stint_server::run_server(port) {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Commands::Dashboard => {
            if let Err(e) = tui::run() {
                eprintln!("error: {e}");
                process::exit(1);
            }
        }
        Commands::Start { project } => cmd_start(project),
        Commands::Stop => cmd_stop(),
        Commands::Status => cmd_status(),
        Commands::Summary => cmd_summary(),
        Commands::Edit { duration, notes } => cmd_edit(duration, notes),
        Commands::DeleteEntry { force } => cmd_delete_entry(force),
        Commands::Add {
            project,
            duration,
            date,
            notes,
        } => cmd_add(project, duration, date, notes),
        Commands::Log {
            from,
            to,
            project,
            tag,
        } => cmd_log(from, to, project, tag),
        Commands::Report {
            group_by,
            format,
            from,
            to,
            project,
            tag,
        } => cmd_report(group_by, format, from, to, project, tag),
        Commands::Project { command } => match command {
            ProjectCommands::Add {
                name,
                path,
                tags,
                rate,
            } => cmd_project_add(name, path, tags, rate),
            ProjectCommands::List { all } => cmd_project_list(all),
            ProjectCommands::Archive { name } => cmd_project_archive(name),
            ProjectCommands::Delete { name, force } => cmd_project_delete(name, force),
            ProjectCommands::Ignore { path } => cmd_project_ignore(path),
            ProjectCommands::Unignore { path } => cmd_project_unignore(path),
        },
        Commands::Shell { shell } => cmd_shell(shell),
        Commands::Init { shell } => cmd_init(shell),
        Commands::Hook {
            cwd,
            pid,
            shell,
            exit,
        } => cmd_hook(cwd, pid, shell, exit),
    }
}
