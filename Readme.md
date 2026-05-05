# RepoPilot

RepoPilot is a local-first CLI for auditing codebases.

The long-term goal is to scan projects, folders, or files and produce evidence-backed reports about architecture, code quality, tests, security, and performance.

```md
RepoPilot currently supports:

- scanning a file or directory
- walking files with gitignore-aware rules
- counting files and directories
- counting non-empty lines of code
- detecting basic languages by file extension
- detecting TODO/FIXME/HACK markers
- printing console output
- printing JSON output
- printing Markdown output
- writing reports to files with `--output`
- configuring large-file threshold with `--max-file-loc`
```
## Findings

RepoPilot uses an evidence-first finding model.

Every finding should include:

- stable rule id
- title
- description
- category
- severity
- evidence

Current finding categories:

- architecture
- code quality
- testing
- security
- performance

Current evidence includes:

- file path
- line number
- source snippet

Current rules:

- `code-marker.todo`
- `code-marker.fixme`
- `code-marker.hack`
- `architecture.large-file`

### Configurable thresholds

The `architecture.large-file` rule uses a default threshold of 300 non-empty LOC.

You can override it:

```bash
cargo run -- scan . --max-file-loc 500
```

## Example

```bash
cargo run -- scan . --format markdown --output report.md
```

## Usage

```bash
cargo run -- scan .
cargo run -- scan . --format json
cargo run -- --help
cargo run -- scan --help
```
Custom large file threshold:

```bash
cargo run -- scan . --max-file-loc 500
cargo run -- scan . --max-file-loc 500 --format markdown --output report.md
```

Markdown output:

```bash
cargo run -- scan . --format markdown
```
Write report to file:

```bash
cargo run -- scan . --format markdown --output report.md
cargo run -- scan . --format json --output report.json
cargo run -- scan . --format console --output report.txt
```