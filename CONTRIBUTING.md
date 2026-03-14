# Contributing to Stint

Thank you for your interest in contributing to Stint! This document explains how the project is governed and how to participate.

## Governance Model: BDFL

Stint follows a **Benevolent Dictator for Life (BDFL)** governance model. Ryan Dalton ([@DaltonR121](https://github.com/DaltonR121)) is the sole developer and maintainer, and has final say on all project decisions including:

- Feature additions and removals
- Architecture and design decisions
- Release timing and versioning
- Pull request acceptance

This isn't a democracy, but it is a benevolent dictatorship — feedback and ideas are genuinely valued.

## How to Participate

### Reporting Bugs

1. Check [existing issues](https://github.com/DaltonR121/stint/issues) to avoid duplicates
2. Use the [bug report template](https://github.com/DaltonR121/stint/issues/new?template=bug_report.md)
3. Include your OS, shell, Stint version, and steps to reproduce

### Requesting Features

1. Check [existing issues](https://github.com/DaltonR121/stint/issues) and [discussions](https://github.com/DaltonR121/stint/discussions)
2. Use the [feature request template](https://github.com/DaltonR121/stint/issues/new?template=feature_request.md)
3. Explain the problem you're trying to solve, not just the solution you want

### Submitting Pull Requests

**Important: Open an issue first.** PRs without prior discussion may not be reviewed. This protects your time and mine.

The workflow:

1. Open an issue describing the change you'd like to make
2. Wait for acknowledgment or discussion
3. Fork the repo and create a branch from `main`
4. Make your changes
5. Ensure all checks pass:
   ```sh
   cargo build
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```
6. Submit a PR referencing the issue

### What Makes a Good PR

- **Small and focused** — one logical change per PR
- **Tests included** — new functionality should have tests
- **No unrelated changes** — don't refactor adjacent code
- **Clear description** — explain what and why, not just how

### What I Won't Merge

- PRs without a corresponding issue
- Large PRs that change multiple unrelated things
- Changes that don't pass CI
- Features that conflict with the project's design philosophy

## Setting Expectations

- **Review timelines are not guaranteed.** This is a side project maintained by one person. I'll review when I can.
- **Not all contributions will be accepted.** Even good ideas may not align with the project's direction.
- **Forks are encouraged.** If you want to take Stint in a different direction, the license allows it.

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy -- -D warnings` and fix all warnings
- Follow existing code conventions in the project
- Write doc comments (`///`) on all public items

## Discussions

For questions, ideas, and general conversation, use [GitHub Discussions](https://github.com/DaltonR121/stint/discussions) rather than issues. Issues are for actionable work items.

## License

By contributing to Stint, you agree that your contributions will be licensed under the project's [BSL-1.1 license](LICENSE).
