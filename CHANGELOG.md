# Changelog

All notable changes to Stint will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
