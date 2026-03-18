# Stint

> **I forgot to start my timer for the 100th time. So I built one that doesn't need starting.**

![Status: Alpha](https://img.shields.io/badge/status-alpha-yellow)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/stint-cli)](https://crates.io/crates/stint-cli)

Stint is an open-source, local-first time tracker built in Rust. Its killer feature: **automatic time tracking via shell hooks**. Open a terminal in a project directory and the clock starts. Switch projects — it switches too. Close the last terminal — it stops. No buttons to click, no browser tabs to manage.

**Works across multiple projects simultaneously** — each terminal tracks its own project independently.

## Why Stint?

Most developer time trackers require you to remember to start and stop timers. You forget, your data is wrong, and the tool becomes useless.

Stint takes a different approach: it hooks into your shell prompt so tracking happens transparently as you work. You can also start/stop manually or add time retroactively — but the default path is zero friction.

## Install

### One-liner (Linux & macOS)

The install script detects your OS and package manager — on Debian/Ubuntu it sets up the apt repository for future upgrades, on other systems it downloads the binary from GitHub Releases.

```sh
curl -fsSL https://daltonr121.github.io/stint/install.sh | sudo sh
```

To inspect the script before running:

```sh
curl -fsSL https://daltonr121.github.io/stint/install.sh -o install.sh
less install.sh
sudo sh install.sh
```

### Other Methods

```sh
# Rust developers
cargo install stint-cli

# Debian/Ubuntu — manual .deb
curl -LO https://github.com/DaltonR121/stint/releases/latest/download/stint-x86_64-unknown-linux-gnu.deb
sudo dpkg -i stint-x86_64-unknown-linux-gnu.deb

# macOS (Apple Silicon)
curl -LO https://github.com/DaltonR121/stint/releases/latest/download/stint-aarch64-apple-darwin.tar.gz
tar xzf stint-aarch64-apple-darwin.tar.gz && sudo mv stint /usr/local/bin/

# From source
git clone https://github.com/DaltonR121/stint.git && cd stint
cargo build --release && sudo cp target/release/stint /usr/local/bin/
```

### VS Code Extension

Search **"Stint"** in the VS Code extension panel, or install from the [Marketplace](https://marketplace.visualstudio.com/items?itemName=mosaic-ridge.stint-vscode). The extension auto-starts the API server and shows your current project + timer in the status bar.

## Quick Start

```sh
# 1. Set up auto-tracking (one-time)
stint init bash    # or: stint init zsh / stint init fish

# 2. Restart your shell, then just work normally.
#    Navigate to any git repo — tracking starts automatically.

# Quick overview of your time
stint summary

# Detailed views
stint status
stint log --from "last monday"
stint report --group-by project
stint report --format csv > timesheet.csv

# Interactive dashboard
stint dashboard

# Local API server (used by VS Code extension)
stint serve
```

### Registering Projects Manually

Auto-discovery handles most git repos, but you can register projects explicitly for custom names, tags, or hourly rates:

```sh
stint project add my-app --path ~/Projects/my-app --tags client,frontend --rate 150
```

### Manual Tracking

If you prefer explicit control (or haven't set up auto-tracking):

```sh
stint start my-app
stint stop

# Add time retroactively
stint add my-app 2h30m --date yesterday --notes "Forgot to track"
```

### Importing Existing Data

Migrate from another time tracker with a CSV export:

```sh
stint import timesheet.csv
```

The CSV must have `project` and `start` columns. Optional: `end`, `duration_secs`, `notes`.

## Features

- **Zero-config auto-tracking** — auto-discovers `.git` repos and tracks time via shell hooks, no manual setup needed
- **Multi-project support** — track multiple projects simultaneously across different terminals
- **Manual tracking** — `stint start`, `stint stop`, `stint status`, `stint add` for full control
- **One-command setup** — `stint init bash|zsh|fish` installs the shell hook (recommended)
- **Multi-shell support** — bash, zsh, and fish
- **Idle detection** — configurable auto-pause (default 5 minutes), resumes on next prompt
- **Project management** — register projects with paths, tags, and hourly rates; archive, delete, ignore
- **Rich reporting** — grouped by project or tag, with table/CSV/JSON/Markdown export and earnings calculation
- **Quick summary** — `stint summary` for a one-line overview of today and this week
- **Entry editing** — `stint edit` and `stint delete-entry` to fix the most recent entry
- **CSV import** — `stint import <file.csv>` for one-time migration from Toggl, Clockify, or any tracker
- **TUI dashboard** — `stint dashboard` with live timer, today's entries, and weekly project totals
- **VS Code extension** — shows current project and live timer in the status bar
- **Local API** — `stint serve` provides a JSON API on localhost for editor plugins and integrations
- **Configurable** — `~/.config/stint/config.toml` for idle threshold, default rate, default tags, and auto-discovery toggle
- **Local-first storage** — SQLite with WAL mode, no account, no cloud, no telemetry

## Configuration

Stint reads optional configuration from `~/.config/stint/config.toml`:

```toml
# Idle detection threshold in seconds (default: 300 = 5 minutes)
idle_threshold = 300

# Default hourly rate in cents for auto-discovered projects (e.g., 15000 = $150/hr)
# default_rate = 15000

# Enable/disable .git auto-discovery (default: true)
auto_discover = true

# Default tags applied to auto-discovered projects
# default_tags = "rust, cli"
```

Environment variable overrides for the hook (no file I/O):
- `STINT_IDLE_THRESHOLD=600` — override idle threshold (seconds)
- `STINT_NO_DISCOVER=1` — disable auto-discovery

## How It Works

Stint installs a shell hook that fires on every prompt render. The hook calls a fast-path subcommand (`stint _hook`) that:

1. Checks your current directory against registered project paths and `.git` repos
2. Compares the detected project to the last-known context for your shell session
3. Starts, stops, or switches timers as needed
4. Detects idle gaps and trims them from tracked time

The hook is engineered to execute in **under 2 milliseconds** — you won't notice it.

### Multi-Terminal / Multi-Project

Each terminal tracks independently. If you have one terminal in `/Projects/stint` and another in `/Projects/client-app`, both projects are tracked simultaneously with separate timers. Within a single project, merge mode keeps one timer regardless of how many terminals are open — the timer only stops when the last terminal leaves.

### Data Storage

All data lives locally in `~/.local/share/stint/stint.db` (SQLite, XDG-compliant). No account, no cloud, no telemetry. Your data stays on your machine.

### Local API

`stint serve` starts a JSON API on `http://127.0.0.1:7653` with endpoints for status, entries, projects, start, and stop. The VS Code extension uses this automatically. See `stint serve --help` for options.

## Roadmap

| Phase | Milestone | Status |
|-------|-----------|--------|
| 0 — Foundation | Project scaffolding, data model, CI | Done |
| 1 — Core CLI | Manual time tracking, reporting, export | Done |
| 2 — Auto-Tracking | Shell hooks, idle detection, multi-terminal | Done |
| 3 — TUI + v0.1.0 | Interactive dashboard, first public release | Done |
| 4 — Zero-Config | Auto-discovery, config, import, entry editing | Done |
| 5 — API + Distribution | Local API, VS Code extension, apt repo, crates.io | Done |
| **6 — Cloud + Web** | Optional hosted sync, web dashboard | **Up Next** |

See [CHANGELOG.md](CHANGELOG.md) for release history.

## Who Is This For?

- **Freelance developers** tracking billable hours across client projects
- **Solo/indie developers** who want to understand where their time goes
- **Team developers** reporting time to project management systems

## Project Structure

```
stint/
  Cargo.toml                # Workspace root
  crates/
    stint-core/             # Domain logic, storage, data models, services
    stint-cli/              # CLI commands, TUI dashboard, user interaction
    stint-server/           # Local HTTP API server (axum)
  editors/
    vscode/                 # VS Code extension (TypeScript)
```

## Contributing

Stint is solo-maintained by [Ryan Dalton](https://github.com/DaltonR121). Bug reports and feature requests are welcome — pull requests are not. See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

The MIT license means you're free to fork and build your own version.

## Security

Found a vulnerability? Please report it responsibly. See [SECURITY.md](SECURITY.md) for details.

## License

Stint is licensed under the [MIT License](LICENSE).

---

Built by [Ryan Dalton](https://github.com/DaltonR121) / [Mosaic Ridge LLC](https://mosaicridge.com)
