# Changelog

All notable changes to RepoPilot will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while the public API is still evolving under `0.x`.

## [Unreleased]

### Added

- Planned AI-ready review context export.
- Planned Change Risk Map.
- Planned GitHub Action integration.

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

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/MykytaStel/repopilot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MykytaStel/repopilot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MykytaStel/repopilot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MykytaStel/repopilot/releases/tag/v0.1.0
