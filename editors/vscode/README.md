# Stint Time Tracker — VS Code Extension

Shows your current [Stint](https://github.com/DaltonR121/stint) time tracking status in the VS Code status bar.

## What It Does

- Displays your current project and live elapsed time in the status bar
- Green clock icon when tracking, dim when idle
- Polls the Stint API every 3 seconds (configurable)
- **Auto-starts `stint serve`** if the API isn't running — no manual setup needed

## Requirements

Install Stint first:

```sh
# One-liner (Linux & macOS)
curl -fsSL https://daltonr121.github.io/stint/install.sh | sudo sh

# Or via Rust
cargo install stint-cli
```

Then set up the shell hook for auto-tracking:

```sh
stint init bash    # or: stint init zsh / stint init fish
```

## How It Works

1. The extension polls `http://127.0.0.1:7653/api/status` for tracking status
2. If the API isn't reachable, it automatically starts `stint serve` in the background
3. The status bar shows: `$(clock) project-name 1h 23m` when tracking, or `$(clock) idle` when not

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `stint.apiUrl` | `http://127.0.0.1:7653` | URL of the Stint API server |
| `stint.pollInterval` | `3000` | How often to poll the API (milliseconds) |
| `stint.stintPath` | `stint` | Path to the stint binary |

## About Stint

Stint is a terminal-native, local-first time tracker built in Rust. Its killer feature is **automatic time tracking via shell hooks** — just `cd` into a project directory and tracking starts. No buttons, no browser tabs.

- **Zero-config**: auto-discovers `.git` repos
- **Multi-project**: tracks multiple projects across terminals simultaneously
- **Private**: all data stays local in SQLite, no cloud, no telemetry

Learn more at [github.com/DaltonR121/stint](https://github.com/DaltonR121/stint)
