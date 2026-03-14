# Stint — Project Conventions

## What Is This?
Stint is a terminal-native project time tracker built in Rust. Killer feature: auto-tracking via shell hooks.

## Owner
Ryan Dalton / Mosaic Ridge LLC — sole developer and maintainer (BDFL).

## Tech Stack
- **Language:** Rust (2021 edition)
- **Build:** Cargo workspace (`crates/stint-core`, `crates/stint-cli`)
- **Storage:** SQLite via `rusqlite` (bundled)
- **CLI:** `clap` for argument parsing
- **Future web dashboard (Phase 5):** Next.js 16, React 19, TypeScript, Tailwind v4, Supabase, pnpm

## Build & Quality
```sh
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```
All four must pass before committing.

## License
FSL-1.1-MIT — free to self-host, commercial hosting reserved, converts to MIT after 2 years.

## Commit Messages
- Keep short and focused on what changed and why
- **NEVER** mention AI assistance or co-authorship

## Key Design Decisions
- Shell hook (`stint _hook`) must execute in **<2ms** — performance is critical
- Storage uses a trait (`Storage`) for pluggability; only `SqliteStorage` implemented initially
- Data is local-first — `~/.local/share/stint/stint.db` (XDG-compliant)
- No telemetry, no analytics, no phone-home behavior

## Current Phase
Phase 4 — Integrations (Toggl/Clockify sync, editor plugins, local API)

Phases 0 (Foundation), 1 (Core CLI), 2 (Auto-Tracking), and 3 (TUI + v0.1.0) are complete.
