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
BSL-1.1 — free to use and self-host, commercial hosting rights reserved to Mosaic Ridge LLC.

## Commit Messages
- Keep short and focused on what changed and why
- **NEVER** mention AI assistance or co-authorship

## Key Design Decisions
- Shell hook (`stint _hook`) must execute in **<2ms** — performance is critical
- Storage uses a trait (`Storage`) for pluggability; only `SqliteStorage` implemented initially
- Data is local-first — `~/.local/share/stint/stint.db` (XDG-compliant)
- No telemetry, no analytics, no phone-home behavior

## Current Phase
Phase 4.5 — Invoicing (invoice generation from tracked time)

Phases 0 (Foundation), 1 (Core CLI), 2 (Auto-Tracking), 3 (TUI + v0.1.0), and 4 (Zero-Config & Daily Use) are complete.

## Revised Roadmap
- Phase 4.5: Invoicing
- Phase 5: Local API + Editor Plugins + Apt Repository
- Phase 6: Cloud + Web Dashboard
