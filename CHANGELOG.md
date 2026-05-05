# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - Unreleased

### Added

- Gitignore-aware file walker using the `ignore` crate
- File and directory counting with non-empty lines-of-code measurement
- Language detection by file extension (Rust, Python, JavaScript, TypeScript, Go, Java, C/C++, HTML, CSS, TOML, YAML, Markdown)
- Code marker detection: `TODO`, `FIXME`, `HACK` with per-line evidence
- Evidence-backed finding model — stable rule IDs, categories, severity levels, and source snippets
- `architecture.large-file` finding with configurable LOC threshold (`--max-file-loc`)
- File facts audit pipeline separating scan, audit, and report responsibilities
- Console, JSON, and Markdown output formats
- `--output` flag to write reports to a file
- CI workflow running `cargo fmt`, `cargo clippy`, and `cargo test` on every PR and push to `main`
- Repository governance docs: release process, distribution channels, audit rulesets, GitHub ruleset setup

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/MykytaStel/repopilot/releases/tag/v0.1.0
