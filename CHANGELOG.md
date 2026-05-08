# Changelog

All notable changes to RepoPilot will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while the public API is still evolving under `0.x`.

## [Unreleased]

### Added

- Planned AI-ready review context export.
- Planned Change Risk Map.

## [0.6.0] - 2026-05-08

### Added

- Added JavaScript, React, and React Native framework audits.
- Added React Native architecture profiling for project kind, Expo config, Android/iOS New Architecture, Hermes, Codegen, package manager, and platform mismatch signals.
- Added framework project detection for JavaScript workspaces through `package.json` workspaces.
- Added React Native findings for architecture mismatch, Hermes mismatch, missing Codegen config, AsyncStorage from core, old React Navigation imports, and direct state mutation.
- Added npm package wrapper for installing RepoPilot as a Node CLI package.
- Added React Native documentation, sample report, baseline + SARIF workflow example, release template, labels metadata, and v0.7 roadmap notes.

### Changed

- React Native old-architecture detection now uses the shared architecture profile instead of duplicate config string checks.
- Release workflow now validates npm packaging and can publish the npm package when `NPM_TOKEN` is configured.

## [0.5.0] - 2026-05-07

### Added

- Added `code-quality.long-function` audit detecting function bodies that exceed the configured line threshold across Rust, Go, Python, TypeScript, and JavaScript.
- Added `architecture.excessive-fan-out` audit for files that import more project-internal files than the configured threshold.
- Added `architecture.high-instability-hub` audit for files with high fan-in and high fan-out (high instability), which amplify change propagation risk.
- Added `architecture.circular-dependency` audit for import cycles across Rust, TypeScript, JavaScript, Python, and Go.
- Added `security.env-file-committed` audit with Critical severity for committed `.env`, `.env.local`, `.env.production`, `.env.staging`, and `.env.development` files.
- Added multi-language import graph (Rust, TypeScript/JavaScript, Python, Go) used by coupling and circular dependency audits.
- Added SARIF 2.1.0 output format (`--format sarif`) for `scan`, `review`, and `compare` commands.
- Added GitHub Code Scanning integration: documentation and copy-paste workflow at `docs/integrations/github-code-scanning.md`.
- Added blast radius computation in `review` reports: shows which files import changed files and may need extra review.
- Added `max_function_lines` config key under `[architecture]` in `repopilot.toml` (default: 50).
- Added `--force` flag to `baseline create` for overwriting an existing baseline file.

### Changed

- Enhanced `code-quality.complex-file` audit with branch-count density calculations for more accurate complexity measurement.
- Enhanced `review` console output with metadata (path, git root, changed files count) and clearer in-diff vs out-of-diff grouping.
- Enhanced `compare` console and Markdown output with file/LOC deltas, new findings, resolved findings, and severity changes.

## [0.4.0] - 2026-05-06

### Added

- Added `repopilot review` for Git diff-aware review of changed lines.
- Added working tree review against `HEAD`, including staged, unstaged, and untracked files.
- Added `repopilot review --base <ref> [--head <ref>]` for branch and CI review.
- Added console, JSON, and Markdown review output with changed files, in-diff findings, out-of-diff findings, severity counts, and baseline status.
- Added review CI gate support so `--fail-on` evaluates only in-diff findings.
- Added review diff parser, Git integration, renderer, and CLI tests.

## [0.3.0] - 2026-05-06

### Added

- Added `repopilot baseline create` for creating a baseline of accepted existing findings.
- Added baseline JSON format with stable finding keys.
- Added `repopilot scan --baseline` support for marking findings as `new` or `existing`.
- Added CI gate support with `--fail-on new-low`, `new-medium`, `new-high`, and `new-critical`.
- Added `--fail-on low`, `medium`, `high`, and `critical` thresholds for strict scans without a baseline.
- Added baseline creation, reading, stable key, and CI failure tests.

### Changed

- Updated scan output with baseline-aware information when a baseline or CI gate is used.
- Updated JSON output with baseline summary metadata when a baseline is used.
- Updated default scan ignores to include `.repopilot/`.

## [0.2.0] - 2026-05-05

### Added

- Added `repopilot init` for generating a default `repopilot.toml`.
- Added project-level `repopilot.toml` configuration support.
- Added config loading with built-in defaults.
- Added tests for config generation, config loading, and scan behavior with config.

### Changed

- Updated `scan` so project configuration is used automatically when available.

## [0.1.0] - 2026-05-05

### Added

- Initial public release.
- Added repository scanning with gitignore-aware walking.
- Added language detection by file extension.
- Added file and directory counting with non-empty lines-of-code measurement.
- Added evidence-backed findings with stable rule IDs, categories, severity levels, and source snippets.
- Added code marker findings for `TODO`, `FIXME`, and `HACK`.
- Added architecture findings for large files, too many modules, and deep nesting.
- Added testing findings for missing test folders and source files without detectable tests.
- Added security findings for secret-like values, private key candidates, and committed `.env` files.
- Added console, JSON, Markdown, and HTML scan output.
- Added `--output` for writing reports to files.
- Added `compare` for diffing two JSON scan reports.
- Added CI workflow, release workflow, distribution docs, release docs, and ruleset docs.

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/MykytaStel/repopilot/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/MykytaStel/repopilot/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MykytaStel/repopilot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MykytaStel/repopilot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MykytaStel/repopilot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MykytaStel/repopilot/releases/tag/v0.1.0
