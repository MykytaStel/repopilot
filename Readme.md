# RepoPilot

[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml)

RepoPilot is a local-first CLI for auditing codebases.

The long-term goal is to scan projects, folders, or files and produce evidence-backed reports about architecture, code quality, tests, security, and performance.

## Features

- Gitignore-aware file walker
- File and directory counting with non-empty LOC measurement
- Language detection by file extension
- `TODO` / `FIXME` / `HACK` marker detection with per-line evidence
- Evidence-backed finding model (stable rule IDs, categories, severity, source snippets)
- `architecture.large-file` finding with configurable LOC threshold
- Console, JSON, and Markdown output formats
- `--output` flag to write reports to a file

See [docs/rulesets.md](docs/rulesets.md) for the full list of audit rules and severity levels.

## Installation

RepoPilot is not yet published to crates.io. Install from source:

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
# binary: ./target/release/repopilot
```

Or install directly with cargo:

```bash
cargo install --git https://github.com/MykytaStel/repopilot.git
```

See [docs/distribution.md](docs/distribution.md) for planned distribution channels (crates.io, pre-built binaries, Homebrew).

## Usage

```bash
cargo run -- scan .
cargo run -- scan . --format json
cargo run -- scan . --format markdown
cargo run -- --help
cargo run -- scan --help
```

Custom large-file threshold:

```bash
cargo run -- scan . --max-file-loc 500
```

Write report to file:

```bash
cargo run -- scan . --format markdown --output report.md
cargo run -- scan . --format json --output report.json
cargo run -- scan . --format console --output report.txt
```

## Findings

RepoPilot uses an evidence-first finding model. Every finding includes:

- Stable rule ID
- Title and description
- Category and severity
- Evidence (file path, line number, source snippet)

Current finding categories: `ARCHITECTURE`, `CODE_QUALITY`, `TESTING`, `SECURITY`, `PERFORMANCE`.

Current rules:

| Rule ID | Category | Severity |
|---|---|---|
| `code-marker.todo` | CODE_QUALITY | LOW |
| `code-marker.fixme` | CODE_QUALITY | MEDIUM |
| `code-marker.hack` | CODE_QUALITY | MEDIUM |
| `architecture.large-file` | ARCHITECTURE | MEDIUM / HIGH |

The `architecture.large-file` rule uses a default threshold of 300 non-empty LOC. Override with `--max-file-loc`.

See [docs/rulesets.md](docs/rulesets.md) for full rule documentation.

## Documentation

| Document | Description |
|---|---|
| [docs/rulesets.md](docs/rulesets.md) | All audit rules, categories, and severity levels |
| [docs/release.md](docs/release.md) | How to cut a release |
| [docs/distribution.md](docs/distribution.md) | Distribution channels (current and planned) |
| [docs/github-ruleset.md](docs/github-ruleset.md) | GitHub branch ruleset configuration |
| [CHANGELOG.md](CHANGELOG.md) | Version history |

## Contributing

- Open a PR against `main`. All PRs require CI to pass and at least one review.
- Follow the [pull request template](.github/pull_request_template.md).
- Update `CHANGELOG.md` for any user-visible change.
