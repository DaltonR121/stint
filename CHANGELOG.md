# Changelog

All notable changes to Stint will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Phase 5 — API + Distribution
- `stint serve` local HTTP API server (axum, localhost:7653)
- API endpoints: `/api/health`, `/api/status`, `/api/entries`, `/api/projects`, `/api/start`, `/api/stop`
- VS Code extension published to Marketplace (auto-starts server, live status bar timer)
- Published to crates.io (`cargo install stint-cli`)
- Apt repository on GitHub Pages with GPG-signed packages
- Universal install script (`curl | sh` with auto-detection of OS and package manager)
- Automated crates.io publishing in release workflow with idempotent version checks
- License changed from BSL-1.1 to MIT

#### Phase 4 — Zero-Config & Daily Use
- `.git` auto-discovery: hook detects git repos and creates projects automatically
- `stint project ignore <path>` / `stint project unignore <path>` to suppress auto-discovery
- `stint summary` for a quick one-line overview of today and this week
- `stint edit --duration --notes` to modify the most recent entry
- `stint delete-entry` to delete the most recent entry with confirmation
- `stint import <file.csv>` for one-time CSV import with auto project creation
- Config file (`~/.config/stint/config.toml`) for idle threshold, default rate, auto-discovery toggle, default tags
- Environment variable overrides for hook (`STINT_IDLE_THRESHOLD`, `STINT_NO_DISCOVER`)
- `ProjectSource` field (manual vs discovered) on projects
- Schema v3 migration: ignored paths table, project source column
- RFC 4180-aware CSV parsing for quoted fields
- Row validation before project creation in import (no orphaned projects)
- License changed from FSL-1.1-MIT to MIT

## [0.1.0] — 2026-03-14

### Added

#### Release & Packaging
- `.deb` package generation via `cargo-deb` in release workflow
- Pre-built binaries for Linux (x86_64, aarch64), macOS (x86_64, Apple Silicon), and Windows (x86_64)
- Install instructions for GitHub Releases, `.deb`, and building from source

#### Phase 3 — TUI + v0.1.0
- `stint dashboard` (alias: `stint tui`) interactive terminal dashboard
- Live-ticking timer status in header (green when tracking, dim when idle)
- Today's entries panel with time, project, duration, source, and notes
- Weekly project totals panel with proportional bar chart
- Keyboard navigation: q/esc quit, tab switches panels, arrows scroll
- RAII terminal guard for panic-safe cleanup
- `stint init bash|zsh|fish` one-command shell hook installation
- Duplicate detection for already-installed hooks

#### Phase 2 — Auto-Tracking
- `stint shell bash|zsh|fish` outputs hook scripts for `eval` in shell config
- `stint _hook` (hidden) called by shell hooks on every prompt render
- Automatic project detection from cwd via registered project paths
- Session lifecycle: cold start, heartbeat, cwd change detection, exit cleanup
- Merge mode: one timer per project across multiple terminals
- Idle detection (5-minute threshold) with automatic gap trimming
- Stale session reaping on cold start (1-hour threshold)
- Hook-only entry management: manual `stint start`/`stint stop` entries are never modified by hooks
- Fast hook path: `SqliteStorage::open_existing` skips directory creation and migrations
- Schema v2 migration with partial indexes for session and hook queries
- DB-level unique constraint preventing duplicate running entries per project

#### Phase 1 — Core CLI
- `stint start <project>` / `stint stop` / `stint status` for manual time tracking
- `stint add <project> <duration>` for retroactive time entries with `--date` and `--notes`
- `stint log` with `--from`, `--to`, `--project`, and `--tag` filtering
- `stint report` with `--group-by project|tag` and `--format markdown|csv|json` export
- `stint project archive <name>` to hide projects from default listings
- `stint project delete <name>` with confirmation prompt and `--force` flag
- Service layer (`StintService`) for business logic with validation
- Duration parser for human-friendly input (`2h30m`, `45m`, `1h`)
- Date parser for relative dates (`today`, `yesterday`, `last monday`) and ISO dates
- Report generation with grouped aggregation, earnings calculation, and deduplicated totals
- Local time detection for date resolution with UTC fallback

#### Phase 0 — Foundation
- Cargo workspace with `stint-core` and `stint-cli` crates
- Domain models: `Project`, `TimeEntry`, `ShellSession`, ULID-based ID newtypes
- `Storage` trait with full CRUD for projects, entries, and sessions
- `SqliteStorage` with WAL mode, embedded schema migrations, partial indexes
- Longest-prefix-match project path lookup via single SQL query
- Transactional writes for atomicity across parent/child rows
- `stint project add <name>` and `stint project list` for project management
- Project scaffolding: README, CONTRIBUTING, SECURITY, LICENSE, CI workflows
- GitHub Actions CI: check, test, clippy, fmt, docs
- GitHub issue and PR templates
