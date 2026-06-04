# Changelog

All notable changes to RepoPilot will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while the public API is still evolving under `0.x`.

## [Unreleased]

### Added

- `repopilot review` now presents its boundary, behavioral, and algorithmic signals as one **"Review signals (preview)"** block grouped into three confidence tiers — *definitely sensitive*, *maybe sensitive*, and *large diff / noise* — across the console, Markdown, and JSON outputs. The old standalone "Security boundary changed" block is folded into the *definitely* tier. The JSON report gains a `tiered_signals` object (`definitely`/`maybe`/`noise` arrays, each signal carrying its family, tier, path, line, neutral headline, detail, and blast radius) plus a `review.tiered_signals` count summary; the `repopilot_review_change` MCP tool returns the same tiers to agents. Every signal is a structural fact, never a verdict. `boundary_signals` and `review.boundary_missing_test` are unchanged — `tiered_signals` is additive.
- Added behavioral change signals ('removed X' family) at `preview` to review removed error handling blocks (try/catch), deleted or emptied test files, and removed authentication/authorization checks.
- Added behavioral change signals ('added X' family) at `preview` to review newly introduced network calls, subprocess executions, filesystem writes, environment variables, dependency imports, database migrations, and raw SQL queries.
- Added algorithmic change signals at `preview`: for each function the diff touched, `repopilot review` now reports structural deltas against the pre-change version — control-flow nesting got deeper, a nested loop (potential O(n²)) appeared, the function grew past a size threshold, or it became recursive. It reports the structural fact ("nesting 2 → 4"), never a verdict, and skips test files. Computed by comparing the pre/post syntax trees over the changed functions.
- Added `[behavioral]` and `[algorithmic]` config sections (each with `enabled`, default true) so the behavioral and algorithmic review signals can be turned off independently.
- Added security-boundary change signals to `repopilot review` (and the `repopilot_review_change` MCP tool). The review now flags when a change touches parts of the repo that decide *who can do what* (auth, sessions, permissions, CORS) or *how the app ships* (CI/workflows, Dockerfiles, IaC, dependency manifests, committed `.env`). It is path/filename classification over the diff — it flags, it does not prove a change is safe, so a false positive costs only a glance. Signals appear in the console and Markdown output and as a structured `boundary_signals` array in the JSON report. Configure or disable via a new `[security_boundary]` config section (`enabled`, `extra_patterns`). Ships at `preview`.
- Enriched boundary signals with review context only RepoPilot has on hand: each signal now carries its **blast radius** (how many files import the changed boundary file, from the existing coupling graph), and the review reports `boundary_missing_test` when a *code* boundary (auth / request-trust) changed but no test moved in the same diff. Both surface in the console/Markdown output and the JSON report (`boundary_signals[].blast_radius`, `review.boundary_missing_test`).
- Added a reproducible review demo: `scripts/demo-setup.sh` builds the "small login fix that quietly loosens CORS and touches auth without a test" scenario and `scripts/demo-boundary-review.sh` runs `repopilot review` on it, so the review demo GIF can be recorded reproducibly.

### Changed

- Internal: the `repopilot review` diff parser now captures each hunk's added and removed line text (`ChangedFile.hunks`), not just the changed-line ranges it derived before. This is the foundation for content-based review signals; `ranges` and all current output are unchanged and `hunks` is not serialized into the JSON report.
- Internal: added a review-layer content bridge (`review::signals::content`) that re-reads a changed file's pre- and post-change source — from the working tree, or `git show <ref>:<path>` for a ref range — and pairs it with the shared tree-sitter parse view, so content-based signals can analyze each side of a change without keeping scan-time file content in memory.
- Internal: `repopilot review` now unifies its boundary, behavioral, and algorithmic signals into three confidence tiers — *definitely sensitive*, *maybe sensitive*, and *large-diff-or-noise* — on `ReviewReport.tiered_signals`, wired through the review pipeline and reach-enriched from the coupling graph. A network call added inside an auth/request-trust file is escalated to *definitely*; a large diff with nothing flagged surfaces one volume note. `boundary_signals`/`boundary_missing_test` are unchanged; the tiered view is rendered across the console, Markdown, and JSON outputs (see the Added entry above).
- Repositioned and tightened the README around reviewing a change *before you merge* — for humans and for AI agents calling RepoPilot over MCP. The page now leads with the review/boundary workflow and the agent (MCP) angle, demotes `scan` and the rest to a compact capability table, and moves the encyclopedic sections (trust mode, commands, rule quality, reports, CI) to their `docs/` links. Cut from ~326 to ~124 lines, deterministic/local-first framing made explicit.
- Reworked the README visuals to a single hero demo plus real, current console-output examples (scan, review with boundary signals, and a baseline gate catching a newly introduced secret) instead of a GIF per command. Removed the per-command GIFs, including a misleading baseline GIF that showed an already-adopted repo rather than the gate doing its job.

## [0.14.0] - 2026-05-31

RepoPilot 0.14 is the AI-guardrail release. Every runtime-risk and code-quality
rule now detects from a shared, parse-once syntax tree instead of line text —
fewer false positives, restored `ast` provenance, and one parse per file. And a
new local `repopilot mcp` server lets AI coding agents call RepoPilot's review,
scan, context, and explain capabilities directly over stdio, with nothing
uploaded and no AI service called.

### Added

- Added `repopilot mcp`, a local Model Context Protocol server over stdio so AI agents (Claude Code, Cursor, …) can call RepoPilot as a tool. It speaks JSON-RPC 2.0 synchronously (no async runtime or extra dependencies) and exposes the `repopilot_review_change` tool, which runs the same local scan + diff review as `repopilot review` and returns a JSON report. Nothing is uploaded and no AI service is called.
- Added three more MCP tools to `repopilot mcp`: `repopilot_scan` (full repository audit as JSON), `repopilot_context` (budgeted AI-ready Markdown brief, with optional `focus`/`budget`), and `repopilot_explain_file` (how a single file is classified and which rules/signals apply). Each wraps the existing `scan`, `ai context`, and `inspect explain` pipelines and runs entirely locally.
- Added architecture anti-pattern documentation describing production-scope policy and false-positive expectations for architecture rules.
- Added a Criterion scan-throughput benchmark (`cargo bench --bench scan_bench`) over a generated 480-file multi-language synthetic repository to baseline full-scan performance ahead of the shared parsed-AST work.
- Added a `scan_timings.parse_us` field reporting aggregate tree-sitter parsing time during file analysis (a sub-measure of `file_analysis_us`, excluded from `accounted_engine_us`).
- Added tree-sitter grammars for Go, Java, C#, and Kotlin so code-quality AST audits analyze real syntax trees for those languages instead of line/brace heuristics.
- Added true-positive and false-positive rule fixtures for `code-quality.long-function` so `inspect eval-rules` covers it.
- Added AST-precision false-positive fixtures for the JavaScript/TypeScript, Python, and Go runtime-risk rules, asserting that risky tokens inside comments and string literals are not flagged.
- Documented an AST-backed vs heuristic-fallback signal-source matrix for the rule catalog in the CLI reference.
- Added a `log.Fatalf` case to the `language.go.panic-exit-risk` true-positive fixture so the variadic fatal-log form is covered by `inspect eval-rules`.
- Added a regression test asserting the runtime-risk AST audits stamp `text-heuristic` provenance on the line-scanner fallback path and that enrichment does not overwrite it.
- Added a `repopilot inspect rules --source ast` example to the CLI reference.

### Changed

- Stabilized `architecture.deep-nesting` rule semantics to analyze production directory nesting depth rather than AST control-flow nesting depth.
- Scoped `architecture.deep-nesting` through a shared production architecture path policy so fixtures, tests, docs, examples, generated files, vendor trees, and build output do not become user-facing architecture findings.
- Folded the thin internal `engine` module into the scan pipeline as a co-located `scan::scanner::contract_stage` finding-contract validation stage and tidied scan-engine imports. Internal refactor only; no behavior or public API change.
- Split the 519-line import `resolver` into per-language submodules (`graph::resolver::{rust, ts, python, go, jvm}`) with the dispatch entry point and shared `probe`/`normalize_path` helpers in the module root. Internal refactor only; no behavior or public API change.
- Consolidated the four single-type `analysis` model files (`context`, `file_role`, `module_kind`, `language_family`) into one `analysis::model` module behind the existing `analysis::{ArchitectureContext, FileRole, ModuleKind, LanguageFamily}` re-exports. Internal refactor only; no behavior or public API change.
- Centralized tree-sitter parser instances and grammar selection into a shared `analysis::parse` module (`ParseLanguage`, `parse`, `parse_label`) and migrated the deep-control-flow audit onto it, removing its duplicated thread-local parsers. Internal refactor only; no behavior or public API change.
- Added a lazy, parse-once `ParsedFile` view and a defaulted `FileAudit::audit_parsed` method so the scan pipeline parses each file at most once and shares the syntax tree across audits. The deep-control-flow audit now consumes the shared tree; other file audits inherit the default and are unaffected. Internal change only; no behavior or finding change.
- Migrated the import graph (`graph::imports`) onto the shared `analysis::parse` parsers and the per-file `ParsedFile`, removing the duplicated thread-local tree-sitter parsers in the Rust, TypeScript/JavaScript, and Python extractors. Import extraction and the AST audits now share a single parse per file instead of parsing it separately, roughly halving per-file parse work in full scans. Internal refactor only; no behavior, finding, or public API change.
- Upgraded `code-quality.long-function` to AST-based function-span detection for every parseable language (Rust, TypeScript/JavaScript, Python, Go, Java, C#, Kotlin), restoring `ast` signal-source provenance and cutting heuristic false positives. The line/brace heuristic remains only as a parse-failure fallback and reports `text-heuristic` provenance.
- Extended the `code-quality.deep-control-flow` audit to Go, Java, C#, and Kotlin with language-specific control-flow node handling, sharing the same parse-once syntax tree.
- Upgraded `language.javascript.runtime-exit-risk` and `language.python.exception-risk` to AST-based detection over the shared parse view — `process.exit` calls, library-boundary `throw new Error(...)`, bare `except` clauses, `assert` statements, and `NotImplementedError` are matched from the syntax tree, so the same tokens inside comments or string literals are no longer flagged. This restores `ast` signal-source provenance; a sanitized line scanner with `text-heuristic` provenance remains only as a parse-failure fallback.
- Upgraded `language.go.panic-exit-risk` to AST-based detection over the shared parse view — `panic`, `log.Fatal`/`log.Fatalf`, and `os.Exit` calls are matched from the syntax tree, so the same tokens inside comments or string literals are no longer flagged. Restores `ast` provenance; the line scanner remains a parse-failure fallback.
- Upgraded `language.rust.panic-risk` to AST-based detection over the shared parse view — `unwrap`/`expect`/`unwrap_err`/`expect_err` calls and `panic!`/`todo!`/`unimplemented!` macros are matched from the syntax tree (and infallible `write!`/`writeln!` result unwraps in report renderers are recognized structurally), so the same tokens inside comments or string literals, including multi-line raw strings, are no longer flagged. Severity context (test/CLI/library/domain) and external-input escalation stay heuristic. Restores `ast` provenance; the line scanner remains a parse-failure fallback.
- Upgraded `language.managed.fatal-exception-risk` (Java, Kotlin, C#) to AST-based detection over the shared parse view — generic fatal-exception `throw`s and `TODO()`/not-implemented placeholders are matched from the syntax tree, so the same tokens inside comments or string literals are no longer flagged. Restores `ast` provenance; the line scanner remains a parse-failure fallback.

## [0.13.0] - 2026-05-25

RepoPilot 0.13 is the trust and usability release: cleaner default output,
stronger rule quality checks, safer baseline adoption, and better evidence for
CI and AI-assisted remediation.

### Added

- Added `repopilot inspect rules`, `repopilot inspect rule <rule_id>`, and
  `repopilot inspect eval-rules` for local rule catalog and fixture evaluation.
- Added rule lifecycle, signal source, default confidence, false-positive notes,
  tags, fixture coverage, false-positive risk, and stability gate status to rule
  metadata and inspection output.
- Added finding contract validation with diagnostics, release-test coverage, and
  scan timing for contract validation.
- Added finding provenance with detector, rule lifecycle, signal source, and
  analysis scope.
- Added signal quality summaries to JSON, console, and Markdown reports.
- Added a local signal-quality checker and made release smoke self-scans fail on
  visible low-quality noisy findings.
- Added local rule evaluation fixtures across security, runtime, and
  import-graph rule families.
- Added golden output fixtures for compact console, full console, Markdown, JSON,
  and SARIF report contracts.
- Added a 0.13 release checklist and announcement focused on trust, usability,
  baseline adoption, and release evidence.

### Changed

- Bumped the crate, npm wrapper, optional platform package pins, and GitHub Action default version to `0.13.0`.
- Bumped scan report schema to `0.17` for raw-vs-visible finding counts and signal quality metrics while accepting `0.15` and `0.16` reports during the transition.
- Default scan visibility now surfaces high-trust security, runtime, and stable
  import-graph risks while keeping low-signal testing, marker, and broad
  maintainability noise strict-only.
- Downgraded line/token runtime and long-function rule provenance from `ast` to `text-heuristic` until those rules consume shared parsed facts.
- Updated risk scoring metadata to `risk-v3` with typed risk signal sources and clearer signal reasons.
- Centralized scan finding enrichment before risk scoring and report construction.
- Cached report signal quality on `ScanSummary` so renderers reuse one summary per report path.
- Split rule inspection catalog, fixture evaluation, and rendering out of the CLI command orchestration.
- Updated AI context and AI plan headings to use the stable product names while
  keeping the public CLI as `repopilot ai context|plan|prompt`.
- AI context, AI plan, and AI prompt now use the same default visibility and local feedback behavior as product scans.
- `repopilot review` now uses the shared default product visibility path before diff classification.
- `repopilot compare` now requires current scan reports with matching top-level and envelope schema metadata instead of migrating older scan report shapes.
- `repopilot baseline create` now uses the shared product scan path and supports `--config` and `--ignore-feedback`.
- `scripts/smoke-product.sh` now covers an install-like release path across
  scan, review, baseline creation, `scan --baseline --fail-on new-high`, AI
  context, `inspect eval-rules`, JSON, SARIF, and Markdown output.

### Removed

- Removed default parsing compatibility for pre-current scan report schemas in the compare reader.

## [0.12.0] - 2026-05-20

### Added

- Expanded context classification for functional, declarative, and infrastructure-oriented files.
- Added an internal deterministic context signal model with path, language, and content evidence for classifier decisions.
- Added a pre-1.0 roadmap with the 0.12 through 0.20 release train, local-first learning policy, and v1 release gates.
- Added Knowledge Engine documentation covering built-in rules, metadata, applicability, calibration, local overlays, and the rule lifecycle.
- Added a 0.12 release checklist focused on core foundation work, rule lifecycle completeness, workspace scan behavior, and v1 readiness.
- Added guard coverage for Knowledge Engine catalog rendering, rule lifecycle completeness, and library-level workspace scan merging.

### Changed

- Centralized reusable context signals for infrastructure, declarative, and functional-first classification.
- Split context model and console/Markdown rendering internals into smaller modules without changing public report schemas.
- Introduced an internal workspace scan plan stage model while preserving existing `--workspace` behavior.
- Improved Knowledge Engine taxonomy so future audits can distinguish application code from infrastructure and declarative files.
- Hardened npm distribution by replacing the install-time binary downloader with platform-specific optional `@repopilot/*` native packages.
- Moved workspace scan orchestration from the CLI command layer into the library scan core while preserving existing `--workspace` behavior.
- Split Knowledge Catalog model construction from rendering while preserving the `inspect knowledge` JSON shape.
- Shared workspace report row construction between console and Markdown renderers.

## [0.11.0] - 2026-05-17

### Added

- Added v0.11 release checklist focused on signal quality, GitHub Action parity, and release trust.
- Added first-party GitHub Action support for `command: doctor`, `fail-on-priority`, `min-priority`, `rule`, and `timing`.
- Added product smoke coverage for priority filtering, rule filtering, scan timing, and priority gates.

### Changed

- Reduced RepoPilot self-audit P2 noise by treating `write!`/`writeln!` result unwraps in report renderers as infallible string-rendering boilerplate while still reporting unwraps inside formatting arguments.
- Documented the v0.11 adoption loop: doctor diagnostics, focused scan/review filters, AI context, and CI gates.

## [0.10.0] - 2026-05-16

### Added

- Added doctor adoption checks for readable `repopilot.toml`, readable `.repopilot/baseline.json`, RepoPilot-specific CI gates, and default report/receipt output readiness.
- Added `schema_version: 1` to audit receipt JSON produced by `repopilot scan --receipt`.
- Added typed GitHub Action `receipt` input and `receipt-file` output for scan receipt generation.
- Added context-aware language runtime-risk audits for Go, JavaScript/TypeScript, Python, and Java/Kotlin/C#.
- Added Django security audits for `DEBUG = True`, empty `ALLOWED_HOSTS`, and raw SQL string formatting.
- Added context-aware confidence and recommendations to long-function and Rust panic-risk findings.
- Added `docs/release-checklist-0.10.md` for the adoption UX release.

### Changed

- `repopilot doctor` now distinguishes generic CI from a RepoPilot `--fail-on` gate and recommends an adoption path from config to report, baseline, CI gate, and AI context.
- `repopilot doctor` now renders diagnostics for invalid config files using built-in scan defaults instead of aborting before the report.
- Restored hidden 0.x compatibility commands: `repopilot vibe`, `repopilot harden`, `repopilot prompt`, `repopilot explain`, and `repopilot knowledge`.
- Grouped AI and inspection workflows remain the stable command shape: `repopilot ai ...` and `repopilot inspect ...`.
- Product smoke checks now generate and validate an audit receipt.

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

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.13.0...HEAD
[0.13.0]: https://github.com/MykytaStel/repopilot/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/MykytaStel/repopilot/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/MykytaStel/repopilot/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/MykytaStel/repopilot/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/MykytaStel/repopilot/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/MykytaStel/repopilot/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/MykytaStel/repopilot/compare/v0.5.0...v0.7.0
[0.6.0]: https://www.npmjs.com/package/repopilot/v/0.6.0
[0.5.0]: https://github.com/MykytaStel/repopilot/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MykytaStel/repopilot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MykytaStel/repopilot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MykytaStel/repopilot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MykytaStel/repopilot/releases/tag/v0.1.0
