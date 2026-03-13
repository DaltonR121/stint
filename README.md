# Stint

> **Terminal-native project time tracking that starts when you do.**

[![Status: Pre-Alpha](https://img.shields.io/badge/status-pre--alpha-orange)]()
[![License: FSL-1.1-MIT](https://img.shields.io/badge/license-FSL--1.1--MIT-blue)](LICENSE)

> **This project is in early development and is not yet functional.** There is no installable binary, no working CLI, and no released version. If you're interested, star the repo and watch for updates. Contributions are not expected at this stage.

Stint is an open-source, local-first time tracker built in Rust. Its killer feature: **automatic time tracking via shell hooks**. Open a terminal in a project directory and the clock starts. Switch projects — it switches too. Close the terminal — it stops. No buttons to click, no browser tabs to manage.

## Why Stint?

Most developer time trackers require you to remember to start and stop timers. You forget, your data is wrong, and the tool becomes useless.

Stint takes a different approach: it hooks into your shell prompt so tracking happens transparently as you work. You can also start/stop manually or add time retroactively — but the default path is zero friction.

## Features

### Available Now
> Stint is in **pre-alpha**. Nothing is available yet — we're building the foundation.

### Planned
- **Automatic time tracking** — shell hooks detect your project from `cwd` and start/stop timers
- **Manual tracking** — `stint start`, `stint stop`, `stint add` for full control
- **Project detection** — auto-detect from git repos or `.stint.toml` config files
- **Rich reporting** — daily/weekly summaries, grouped by project or tag, with CSV/JSON/Markdown export
- **Invoicing** — `stint invoice <project>` with hourly rate support
- **TUI dashboard** — interactive terminal UI with calendar heatmaps and live timers
- **Multi-shell support** — bash, zsh, fish, with tmux integration
- **Multi-terminal handling** — merge or parallel modes for concurrent sessions
- **Idle detection** — auto-pause after configurable inactivity
- **Pluggable storage** — SQLite by default, trait-based architecture for future adapters
- **Import/export** — migrate from Watson, Toggl, or generic CSV
- **Optional cloud sync** — self-hostable web dashboard with team features (future)

## Quick Start

> **Note:** Stint is not yet installable. These instructions will work once Phase 1 is complete.

### Install

```sh
# From crates.io
cargo install stint

# Or via Homebrew (macOS/Linux)
brew install daltonr121/tap/stint

# Or download a pre-built binary from GitHub Releases
```

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
stint log --from "last monday"
stint report --group-by project
stint report --format csv > timesheet.csv
```

### Auto-Tracking (the magic)

Add one line to your shell config:

```sh
# Bash (~/.bashrc)
eval "$(stint shell bash)"

# Zsh (~/.zshrc)
eval "$(stint shell zsh)"

# Fish (~/.config/fish/config.fish)
stint shell fish | source
```

That's it. Navigate to a project directory and Stint starts tracking. Switch directories — it switches. Close the terminal — it stops. No manual intervention.

### Configuration

```sh
# Global settings
stint config set auto_track true
stint config set idle_timeout 15m

# Per-project overrides (creates .stint.toml in project root)
stint config set --project my-app hourly_rate 150
```

## How It Works

Stint installs a shell hook that fires on every prompt render. The hook calls a fast-path subcommand (`stint _hook`) that:

1. Checks your current directory against registered project paths
2. Looks for `.stint.toml` or `.git` markers up the directory tree
3. Compares the detected project to the last-known context for your shell session
4. Starts, stops, or switches timers as needed

The hook is engineered to execute in **under 2 milliseconds** — you won't notice it.

### Multi-Terminal Behavior

- **Merge mode** (default): One timer per project, regardless of how many terminals are open
- **Parallel mode**: Each terminal session gets its own time entry

### Data Storage

All data lives locally in `~/.local/share/stint/stint.db` (SQLite, XDG-compliant). No account, no cloud, no telemetry. Your data stays on your machine unless you explicitly opt into cloud sync (future feature).

## Roadmap

| Phase | Milestone | Status |
|-------|-----------|--------|
| **0 — Foundation** | Project scaffolding, data model, CI | **In Progress** |
| 1 — Core CLI | Manual time tracking, reporting, export | Planned |
| 2 — Auto-Tracking | Shell hooks, idle detection, multi-terminal | Planned |
| 3 — TUI + v0.1.0 | Interactive dashboard, first public release | Planned |
| 4 — Integrations | Toggl/Clockify sync, editor plugins, local API | Planned |
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
    stint-core/             # Domain logic, storage trait, data models
    stint-cli/              # CLI interface, shell hooks, commands
  shell/                    # Shell integration scripts (bash/zsh/fish)
  docs/                     # Architecture Decision Records
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
