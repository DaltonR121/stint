# Contributing to Stint

Thanks for your interest in Stint!

## How This Project Works

Stint is maintained by [Ryan Dalton](https://github.com/DaltonR121). Contributions are welcome, with one ask: **talk before you build anything non-trivial.**

**Welcome any time:**
- Bug reports
- Feature requests and ideas
- Questions and feedback
- Small pull requests — typo fixes, docs improvements, obvious bug fixes

**Open an issue first:**
- New features, behavior changes, refactors, or anything beyond a small fix

Opening an issue before a larger PR isn't a hoop to jump through — it's so we can agree on the problem and the approach before you spend time on code that might not fit the project's direction. Describe the **problem** you're solving (not just the solution you have in mind), and I'll let you know whether it fits the roadmap and how best to approach it.

## Reporting Bugs

1. Check [existing issues](https://github.com/DaltonR121/stint/issues) to avoid duplicates
2. Include your OS, shell, Stint version (`stint --version`), and steps to reproduce
3. Paste any error output

## Requesting Features

1. Open an issue describing the **problem** you're trying to solve (not just the solution you want)
2. I'll respond with whether it fits the roadmap and when I might get to it

## Submitting Pull Requests

For a small fix, or once an issue has settled the approach for a larger change:

1. Fork the repo and branch from `main` (`fix/...`, `feat/...`, or `chore/...`).
2. Make the change **with tests** — every feature and bug fix ships with a test.
3. Run the full quality gate locally before pushing — all four must pass:
   ```sh
   cargo build
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```
4. Open the PR against `main`. CI runs the same gate, and the maintainer is automatically requested for review.
5. Keep PRs small and focused — one fix or one feature per PR.

Shell hooks (`stint _hook`) are on a hot path and must stay fast (<2ms), so be mindful of work added there.

## Recognition

Every contribution is credited. Merged PRs appear in the auto-generated release
notes, and contributors of all kinds (code, docs, bug reports, ideas) are added
to the [Contributors](README.md#contributors) list via the
[all-contributors](https://allcontributors.org) bot — comment
`@all-contributors please add @username for code` on any issue or PR to add
someone.

## Forking

The [MIT license](LICENSE) means you're free to fork Stint and take it in any direction you want. If there's something you need that doesn't fit this project's vision, forking is encouraged.

## Discussions

For questions, ideas, and general conversation, use [GitHub Discussions](https://github.com/DaltonR121/stint/discussions) rather than issues. Issues are for actionable work items.
