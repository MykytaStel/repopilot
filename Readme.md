# RepoPilot

RepoPilot is a local-first CLI for auditing codebases.

The long-term goal is to scan projects, folders, or files and produce evidence-backed reports about architecture, code quality, tests, security, and performance.

## Current v0.1 scope

RepoPilot currently supports:

- scanning a file or directory
- walking files with gitignore-aware rules
- counting files and directories
- counting non-empty lines of code
- detecting basic languages by file extension
- detecting TODO/FIXME/HACK markers
- printing console output
- printing JSON output

## Usage

```bash
cargo run -- scan .
cargo run -- scan . --format json
cargo run -- --help
cargo run -- scan --help
```
Markdown output:

```bash
cargo run -- scan . --format markdown
```

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
```