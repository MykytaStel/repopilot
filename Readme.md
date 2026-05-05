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
- Architecture, testing, and security audit rules with configurable thresholds
- `compare` command for diffing two JSON scan reports
- Console, JSON, Markdown, and HTML scan output formats
- `--output` flag to write reports to a file

See [docs/rulesets.md](docs/rulesets.md) for the full list of audit rules and severity levels.

## Installation

RepoPilot v0.1.0 is prepared for source installs, GitHub Release binaries, and crates.io publishing.
Until the first release is cut, install from source:

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

See [docs/distribution.md](docs/distribution.md) for distribution channels. Homebrew support is planned after GitHub Release artifacts are available.

## Usage

```bash
cargo run -- scan .
cargo run -- scan . --format json
cargo run -- scan . --format markdown
cargo run -- scan . --format html --output report.html
cargo run -- --help
cargo run -- scan --help
```

Custom thresholds:

```bash
cargo run -- scan . --max-file-loc 500
cargo run -- scan . --max-directory-modules 25
cargo run -- scan . --max-directory-depth 6
```

Write report to file:

```bash
cargo run -- scan . --format markdown --output report.md
cargo run -- scan . --format json --output report.json
cargo run -- scan . --format html --output report.html
cargo run -- scan . --format console --output report.txt
```

Compare two JSON reports:

```bash
cargo run -- scan . --format json --output before.json
cargo run -- scan . --format json --output after.json
cargo run -- compare before.json after.json
cargo run -- compare before.json after.json --format markdown
cargo run -- compare before.json after.json --format json --output diff.json
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
| `architecture.too-many-modules` | ARCHITECTURE | MEDIUM |
| `architecture.deep-nesting` | ARCHITECTURE | LOW |
| `testing.missing-test-folder` | TESTING | MEDIUM |
| `testing.source-without-test` | TESTING | LOW |
| `security.secret-candidate` | SECURITY | HIGH |
| `security.private-key-candidate` | SECURITY | CRITICAL |
| `security.env-file-committed` | SECURITY | HIGH |

The `architecture.large-file` rule uses a default threshold of 300 non-empty LOC. Override with `--max-file-loc`.
Directory architecture thresholds default to 20 files per directory and 5 directory levels below the scan root.

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
