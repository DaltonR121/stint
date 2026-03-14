# Stint — Project Conventions

## What Is This?
Stint is a terminal-native project time tracker built in Rust. Killer feature: auto-tracking via shell hooks.

## Owner
Ryan Dalton / Mosaic Ridge LLC — sole developer and maintainer (BDFL).

## Tech Stack
- **Language:** Rust (2021 edition)
- **Build:** Cargo workspace (`crates/stint-core`, `crates/stint-cli`, `crates/stint-server`)
- **Storage:** SQLite via `rusqlite` (bundled)
- **CLI:** `clap` for argument parsing
- **TUI:** `ratatui` + `crossterm`
- **API Server:** `axum` + `tokio`
- **VS Code Extension:** TypeScript (`editors/vscode/`)
- **Future web dashboard (Phase 6):** Next.js 16, React 19, TypeScript, Tailwind v4, Supabase, pnpm

## Build & Quality
```sh
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```
All four must pass before committing.

## License
MIT — free to use, modify, and distribute.

## Commit Messages
- Keep short and focused on what changed and why
- **NEVER** mention AI assistance or co-authorship

## Key Design Decisions
- Shell hook (`stint _hook`) must execute in **<2ms** — performance is critical
- Storage uses a trait (`Storage`) for pluggability; only `SqliteStorage` implemented initially
- Data is local-first — `~/.local/share/stint/stint.db` (XDG-compliant)
- No telemetry, no analytics, no phone-home behavior
- Multi-project tracking: each terminal tracks independently, merge mode within a single project

## Distribution
- **crates.io:** `cargo install stint-cli`
- **GitHub Releases:** pre-built binaries + `.deb` for Linux, macOS, Windows
- **Apt repository:** `curl | sh` install script at `daltonr121.github.io/stint/install.sh`
- **VS Code Marketplace:** `mosaic-ridge.stint-vscode`
- **Automated:** all publishing triggered by `git tag vX.Y.Z && git push origin vX.Y.Z`

## Current Phase
Phase 6 — Cloud + Web Dashboard

Phases 0 (Foundation), 1 (Core CLI), 2 (Auto-Tracking), 3 (TUI + v0.1.0), 4 (Zero-Config & Daily Use), and 5 (API + Distribution) are complete.
