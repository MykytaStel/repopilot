# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `FileAudit` and `ProjectAudit` traits — audit rules now implement a trait instead of free functions, making it trivial to add new rules without modifying the pipeline
- `Markdown ## Markers` section — dedicated table for `TODO`/`FIXME`/`HACK` findings in Markdown reports
- `ScanConfig` fields: `max_directory_modules`, `max_directory_depth`, `long_function_loc_threshold` — groundwork for upcoming architecture and code-quality rules
- `Cargo.toml` crates.io metadata: description, license, repository, keywords, categories
- GitHub Actions release workflow — builds and uploads pre-built binaries for Linux x86_64/ARM64, macOS x86_64/ARM64, and Windows x86_64 on `v*` tags; publishes to crates.io

### Changed

- `code_markers` audit now reuses `scan::markers::detect_markers` instead of duplicating detection logic
- `scan::markers` module promoted to a first-class scan primitive (`MarkerKind`, `Marker`, `detect_markers`)

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
