# Security Policy

## Supported Versions

Stint is in pre-alpha. Security fixes will be applied to the latest version only.

| Version | Supported |
|---------|-----------|
| Latest  | Yes       |
| Older   | No        |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Instead, please report vulnerabilities privately:

1. **Email:** ryan@mosaicridge.com
2. **GitHub:** Use [private vulnerability reporting](https://github.com/DaltonR121/stint/security/advisories/new)

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

### Response Timeline

- **Acknowledgment:** Within 48 hours
- **Assessment:** Within 1 week
- **Fix/disclosure:** Coordinated with reporter, typically within 30 days

## Scope

Stint is a local-first CLI tool. The primary security concerns are:

- **Local data integrity** — unauthorized access to the SQLite database
- **Shell hook safety** — the hook executes on every prompt; it must not introduce injection vectors
- **Dependency supply chain** — malicious or vulnerable Rust crates
- **Future: API server** — `stint serve` will expose a local HTTP API (Phase 4)
- **Future: Cloud sync** — authentication and data privacy (Phase 5)

## Design Principles

- Data is stored locally by default with no network access
- Shell hooks execute only read operations against the database on the hot path
- No telemetry, no analytics, no phone-home behavior
- Dependencies are reviewed and kept minimal
