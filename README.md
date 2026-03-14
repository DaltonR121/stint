# Stint

> **Terminal-native project time tracking that starts when you do.**

[![Status: Pre-Alpha](https://img.shields.io/badge/status-pre--alpha-orange)]()
[![License: FSL-1.1-MIT](https://img.shields.io/badge/license-FSL--1.1--MIT-blue)](LICENSE)

> **This project is in early development.** The CLI is fully functional for manual and automatic time tracking, but there is no published binary or crate yet — build from source to try it out.

Stint is an open-source, local-first time tracker built in Rust. Its killer feature: **automatic time tracking via shell hooks**. Open a terminal in a project directory and the clock starts. Switch projects — it switches too. Close the last terminal — it stops. No buttons to click, no browser tabs to manage.

## Why Stint?

Most developer time trackers require you to remember to start and stop timers. You forget, your data is wrong, and the tool becomes useless.

Stint takes a different approach: it hooks into your shell prompt so tracking happens transparently as you work. You can also start/stop manually or add time retroactively — but the default path is zero friction.

## Features

### Available Now
- **Automatic time tracking** — shell hooks detect your project from `cwd` and start/stop timers transparently
- **Manual tracking** — `stint start`, `stint stop`, `stint status`, `stint add` for full control
- **Multi-shell support** — bash, zsh, and fish hook scripts via `stint shell <type>`
- **Multi-terminal handling** — merge mode keeps one timer per project across terminals
- **Idle detection** — auto-pause after 5 minutes of inactivity, resumes on next prompt
- **Project management** — register projects with paths, tags, and hourly rates; archive and delete
- **Rich reporting** — grouped by project or tag, with CSV/JSON/Markdown export
- **Retroactive entries** — `stint add 2h30m --date yesterday --notes "..."` with human-friendly duration and date parsing
- **TUI dashboard** — `stint dashboard` with live timer, today's entries, and weekly project totals
- **One-command setup** — `stint init bash|zsh|fish` installs the shell hook automatically
- **Pluggable storage** — SQLite by default (WAL mode), trait-based architecture for future adapters

### Planned
- **Project auto-discovery** — automatically detect unregistered projects from `.git` repos or `.stint.toml` markers (no manual `project add` needed)
- **Invoicing** — `stint invoice <project>` with hourly rate support
- **Import/export** — migrate from Watson, Toggl, or generic CSV
- **Optional cloud sync** — self-hostable web dashboard with team features (future)

## Quick Start

> **Note:** Stint is not yet published to crates.io. Build from source for now (see [Building From Source](#building-from-source)).

### Basic Usage

```sh
# Register a project
stint project add my-app --path ~/Projects/my-app --tags client,frontend

# Manual tracking
stint start my-app
stint stop

# Add time retroactively
stint add my-app 2h30m --date yesterday --notes "Forgot to track"

# View your time
stint status
stint log --from "last monday"
stint report --group-by project
stint report --format csv > timesheet.csv

# Interactive dashboard
stint dashboard
```

### Auto-Tracking

Set up auto-tracking with one command:

```sh
# Install the shell hook (appends to your shell config)
stint init bash    # or: stint init zsh / stint init fish

# Or add manually to your shell config:
# eval "$(stint shell bash)"
```

Navigate to a registered project directory and Stint starts tracking. Switch directories — it switches. In merge mode, the timer stops when the last terminal tracking that project closes or leaves the directory. No manual intervention.

The hook is engineered to execute in **under 2 milliseconds** — you won't notice it.

## How It Works

Stint installs a shell hook that fires on every prompt render. The hook calls a fast-path subcommand (`stint _hook`) that:

1. Checks your current directory against registered project paths
2. Compares the detected project to the last-known context for your shell session
3. Starts, stops, or switches timers as needed
4. Detects idle gaps (>5 min) and trims them from tracked time

### Multi-Terminal Behavior

- **Merge mode** (default): One timer per project, regardless of how many terminals are open. The timer only stops when the last terminal tracking that project closes or leaves the directory.

### Data Storage

All data lives locally in `~/.local/share/stint/stint.db` (SQLite, XDG-compliant). No account, no cloud, no telemetry. Your data stays on your machine unless you explicitly opt into cloud sync (future feature).

## Roadmap

| Phase | Milestone | Status |
|-------|-----------|--------|
| 0 — Foundation | Project scaffolding, data model, CI | Done |
| 1 — Core CLI | Manual time tracking, reporting, export | Done |
| 2 — Auto-Tracking | Shell hooks, idle detection, multi-terminal | Done |
| 3 — TUI + v0.1.0 | Interactive dashboard, first public release | Done |
| **4 — Integrations** | Toggl/Clockify sync, editor plugins, local API | **Up Next** |
| 5 — Cloud + Web | Optional hosted sync, web dashboard, billing | Planned |

See [CHANGELOG.md](CHANGELOG.md) for release history.

## Who Is This For?

- **Freelance developers** tracking billable hours across client projects
- **Solo/indie developers** who want to understand where their time goes
- **Team developers** reporting time to project management systems

## Building From Source

```sh
git clone https://github.com/DaltonR121/stint.git
cd stint
cargo build
cargo test
```

### Requirements

- Rust 1.75+ (2021 edition)
- SQLite (bundled via `rusqlite`, no system dependency needed)

## Project Structure

```
stint/
  Cargo.toml                # Workspace root
  crates/
    stint-core/             # Domain logic, storage, data models, services
    stint-cli/              # CLI commands and user interaction
```

## Contributing

Stint is maintained by a single developer under a [BDFL governance model](CONTRIBUTING.md). Contributions are welcome but please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a pull request.

**TL;DR:** Open an issue first to discuss. PRs without prior discussion may not be reviewed.

## Security

Found a vulnerability? Please report it responsibly. See [SECURITY.md](SECURITY.md) for details.

## License

Stint is licensed under the [Functional Source License, Version 1.1, MIT Future License (FSL-1.1-MIT)](LICENSE).

- **Free** to use, modify, and self-host
- **Commercial hosting** rights reserved to Mosaic Ridge LLC
- **Converts to MIT** automatically after 2 years

See [LICENSE](LICENSE) for the full text.

---

Built by [Ryan Dalton](https://github.com/DaltonR121) / [Mosaic Ridge LLC](https://mosaicridge.com)
