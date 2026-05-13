# Changelog

All notable changes to RepoPilot will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while the public API is still evolving under `0.x`.

## [Unreleased]

## [0.9.0] - 2026-05-13

### Added

- Added audit registry metadata for file, project, and framework audits so registered audits declare their scope and emitted rule IDs.
- Added a bundled RepoPilot Knowledge Engine foundation for language, framework, runtime, paradigm, and rule applicability decisions.
- Added a tiered language corpus covering rule-aware, context-aware, import-aware, and detect-only support levels across common and emerging languages.
- Added `repopilot explain` for inspecting file context classification and optional Knowledge Engine rule decisions.
- Added `repopilot knowledge` to inspect the bundled Knowledge Engine catalog.
- Added section filtering for languages, frameworks, runtimes, paradigms, and rule applicability records.
- Added JSON and Markdown output support for the knowledge catalog.
- Added stable AI command family: `repopilot ai context`, `repopilot ai plan`, and `repopilot ai prompt`.
- Added advanced inspection command family: `repopilot inspect explain` and `repopilot inspect knowledge`.
- Added CLI stabilization tests covering help surface, legacy AI/inspect compatibility, exit codes, and high-severity self-audit cleanliness.
- Added `scripts/smoke-product.sh` for product-readiness smoke checks across first-run, scan, review, AI, inspect, and legacy compatibility flows.
- Added `docs/product-readiness.md` to document release gates, CLI stability expectations, CI behavior, local-first guarantees, and distribution checks.
- Added product documentation for installation, security model, AI-assisted workflows, language support, and configuration.

### Changed

- Language detection now uses the Knowledge Engine corpus instead of a hardcoded extension table.
- Context-aware audits can suppress or adjust findings based on file role, runtime, paradigm, test/config/generated status, and rule applicability.
- Top-level `vibe`, `harden`, `prompt`, `explain`, and `knowledge` commands are hidden from primary help while remaining available as 0.x compatibility commands.
- CLI runtime errors now exit with code 3, invalid user input uses code 2, and findings threshold failures use code 1.
- GitHub Action inputs now prefer typed arguments for common scan, review, and AI workflow options while retaining `args` for advanced use.
- Release verification now delegates end-to-end binary smoke checks to the product-readiness smoke suite.
- The curl installer now fails closed when the release checksum file cannot be downloaded, no SHA256 tool is available, or checksum verification fails.

## [0.8.0] - 2026-05-11

### Added

- Added `repopilot vibe` command (alias: `v`): scans a project and formats findings as LLM-ready markdown for direct use in Claude Code, Cursor, or ChatGPT. Output includes risk level, tech stack summary, findings grouped by category with evidence snippets and fix recommendations, and a token-budget estimate.
  - `--focus security|arch|quality|framework|all` — restrict output to one category.
  - `--budget 2k|4k|8k|16k` — control approximate output token count (default: 4k).
  - `--no-header` — omit the intro block for piping into an LLM API.
- Added `repopilot harden` command (alias: `h`): scans a project and emits a Markdown hardening plan with P0/P1/P2/P3 priorities, locations, rule IDs, recommendations, and verification commands.
- Added `repopilot prompt` command (alias: `p`): scans a project and emits a paste-ready Markdown remediation prompt with embedded RepoPilot context.
- Added `--preset strict|balanced|lenient` flag to `scan` command for one-shot threshold tuning without editing `repopilot.toml`.
  - `strict`: tighter thresholds — catches more issues, suitable for green-field projects.
  - `balanced`: factory defaults.
  - `lenient`: relaxed thresholds for legacy codebases adopting RepoPilot incrementally.
- Added `--verbose` flag to `scan` command: prints scan phase timing (engine ms, render ms) to stderr after the report.
- Java and Kotlin language support: long function detection, import extraction for the coupling graph, and JVM import resolution supporting Maven and Gradle source-root layouts (`src/main/java`, `src/main/kotlin`, `app/src/main/java`, `app/src/main/kotlin`).
- Python framework detection: Django, Flask, and FastAPI are detected from `requirements.txt` (with pinned version when present) and included in the tech-stack summary produced by `repopilot vibe`.
- Go framework detection: Gin, Echo, and Fiber are detected from `go.mod` and included in the tech-stack summary.
- Added scan input controls on `repopilot scan`: `--exclude`, `--include-low-signal`, `--max-file-size`, and `--max-files`.
- Added JSON scan accounting fields: `files_discovered`, `files_skipped_low_signal`, and `binary_files_skipped`.
- Added default ignores for `.nuxt`, `.cache`, `vendor`, `Pods`, and `DerivedData`.

### Changed

- First-party GitHub Action now supports `command: vibe`, `command: harden`, and `command: prompt` without passing unsupported `--format` flags.
- Release publishing keeps npm Trusted Publishing in `publish-npm.yml`; optional crates.io and Homebrew channels are skipped when their secrets are absent.
- Workspace scans (`--workspace`) now run per-package scans in parallel using rayon, giving roughly 50% speedup on monorepos with ten or more packages.
- Import deduplication in the coupling graph now uses `HashSet` instead of `BTreeSet`, reducing import extraction time by 3-7% on large TypeScript/Rust projects.
- Coupling graph construction avoids a redundant `PathBuf` clone per file; `compute_metrics` no longer pre-allocates two full-size maps — both reduce allocations on large graphs.
- `files_count` now represents analyzed text files; skipped large, binary, low-signal, and capped files are tracked separately.
- Console scan input reporting now labels files skipped by `--max-files` as limit-skipped instead of ignore-skipped.

## [0.7.0] - 2026-05-08

### Added

- Added per-workspace scans: `--workspace` flag on `scan` command detects npm, yarn, pnpm, and Cargo workspace packages and runs audits scoped to each package root.
- Added React Native dependency health audits: `framework.rn-navigation-compat`, `framework.rn-async-storage-legacy`, `framework.rn-reanimated-compat`, `framework.rn-gesture-handler-old`, and `framework.rn-new-arch-incompatible-dep`.
- Added first-party GitHub Action (`action.yml`) for running RepoPilot audits in CI with optional SARIF upload to GitHub Code Scanning.
- Automated crates.io publishing in the release workflow via `CRATES_IO_TOKEN` secret.
- Automated Homebrew tap formula updates in the release workflow via `HOMEBREW_TAP_TOKEN` secret.
- Complete rule registry: all 36 rule IDs emitted by audits now have `RuleMetadata` entries (title, description, recommendation, docs URL) across architecture, code-quality, security, testing, framework JS/React, React Native, and code-marker categories.
- SARIF enrichment: `tool.driver.rules[*].help.text` is now populated from the registry's `recommendation` field for every rule that has one; `results[*].properties` now always includes `category` (e.g. `"security"`) and `workspacePackage` (when set).
- Workspace Risk Summary: `--workspace` scans now render a compact per-package severity breakdown table in console and markdown output.
- React Native rule metadata completeness: `framework.react-native.architecture-mismatch` now links to the official New Architecture docs; all five `framework.rn-*` dependency-health findings now carry `docs_url`.

### Changed

- Version 0.6.0 was published to npm without a matching git tag; v0.7.0 is the first fully tagged release since v0.5.0 and restores version consistency across crates.io, npm, and GitHub Releases.

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

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/MykytaStel/repopilot/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/MykytaStel/repopilot/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/MykytaStel/repopilot/compare/v0.5.0...v0.7.0
[0.6.0]: https://www.npmjs.com/package/repopilot/v/0.6.0
[0.5.0]: https://github.com/MykytaStel/repopilot/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MykytaStel/repopilot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MykytaStel/repopilot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MykytaStel/repopilot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MykytaStel/repopilot/releases/tag/v0.1.0
