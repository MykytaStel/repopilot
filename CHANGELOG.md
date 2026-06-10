# Changelog

All notable changes to RepoPilot will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while the public API is still evolving under `0.x`.

## [Unreleased]

### Added

- Added four architecture rules that query the import graph.
  `architecture.dead-module` (production files nothing imports that are not entrypoints or public API) and `architecture.test-leak` (production code
  importing a test or fixture file) are on by default.
  `architecture.layer-violation` and `architecture.package-boundary-violation`
  are **strictly opt-in** and emit nothing until you declare structure: ordered
  `[[architecture.layers]]` (a module may import layers at or below its own,
  never above) and `[architecture] package_roots` (e.g. `packages/*`, flagging
  imports that reach into another package's internals instead of its public
  API). Each rule ships with true-positive and false-positive fixtures; all
  four are `experimental`.
- Added per-rule configuration: `[rules] disable` drops a rule's findings and
  `[rules.severity_overrides]` sets an absolute severity per rule. Unknown rule
  ids and invalid severities are surfaced as report diagnostics, not applied.
- Added regression coverage and a release self-review quality gate for
  standard-library/local dependency imports, AST-confirmed recursion,
  prose-only removed-behavior text, and side effects in tests/fixtures.
- Added stdlib Python tests for release-note extraction, version validation,
  Cargo package allowlisting, pinned Actions, npm orchestration, and removed
  editor packaging.
- Added a real schema `0.18` report fixture and documented report schema
  versioning independently from package versions.
- Review now prints a one-line taint disclaimer when taint-lite signals are
  present (console and Markdown): they trace input → sink *reachability*, a path
  that exists, not a confirmed vulnerability.

### Changed

- The rule registry is now the single source of truth for finding severity and
  confidence. `populate_rule_metadata` applies each rule's registry default
  (an audit may only lower severity, or raise it up to a declared ceiling), so
  the MCP rules catalog and the emitted findings no longer disagree. Report
  schema bumped to `0.19` (additive). Architecture audits no longer hardcode
  severity literals.
- Internal: moved the coupling graph → graph v2 `GraphSnapshot` conversion out
  of the `architecture.circular-dependency` audit and into a shared graph stage
  (`build_coupling_graph_snapshot`), so future graph v2 consumers can reuse it.
  No user-facing behavior, finding contract, or output schema changed.

- Internal: completed the near-term graph v2 migration so the import-coupling
  rules, review blast radius, and the AI context hot-files fallback share one
  graph model, with every migrated output preserved and guarded by parity tests.
  `architecture.excessive-fan-out` and `architecture.high-instability-hub` now
  derive fan-in/fan-out/instability from a single v2-backed `coupling_file_metrics`
  (degree counting over the shared snapshot, instability owned by
  `NodeDegree::instability`); the AI context hot-files fallback uses the same
  function instead of v1 `compute_metrics`. Review's importer inversion
  (`build_importers_by_target`, feeding blast radius and boundary/tiered signals)
  is sourced from the snapshot via a new one-hop `direct_dependents` algorithm.
  A `graph_capabilities` helper adds bounded, internal metadata describing the
  dependency facts a snapshot carries (no command consumes it yet). Finding IDs,
  severities, evidence, thresholds, and review JSON are unchanged; the Rust facade
  exemption moved to an `import_coupling::rust_facade` submodule to keep each file
  under the 300-line limit.

- Internal: moved the Context Risk Graph summary and the risk graph-impact overlay
  off the v1 `compute_metrics` path onto the shared v2-backed `coupling_file_metrics`,
  and rebuilt the context summary's changed-blast-radius from the shared one-hop
  `direct_dependents` instead of a private edge inversion. This removes the last
  duplicate importer-inversion copy and leaves v1 `compute_metrics` out of every
  production path (it remains only as the parity-test reference). The
  `ContextGraphSummary` (AI context "Context Risk Graph", review CI, report schema)
  and all risk scoring are byte-for-byte unchanged, guarded by existing and new
  parity tests.

- Documented the gate model as one coherent two-axis story: a **finding gate**
  (`--fail-on` by severity/status, or the mutually exclusive `--fail-on-priority`
  by risk; in-diff only on `review`) and a separate, opt-in **review-signal gate**
  (`--fail-on-review`, config peer `[review] fail_on`). Harmonized the `scan` and
  `review` flag help and the CLI/configuration references so the config key
  `[review] fail_on` is no longer mistaken for the CLI `--fail-on`. No flags or
  behavior changed — only help text and documentation.

- Consolidated the AI handoff into a single `ai context` command. Its output now
  bundles repository facts, evidence, the Context Risk Graph edit order, a
  prioritized P0–P3 remediation plan, working rules, and a verification checklist
  in one document. `--no-task` drops the agent guidance (task, rules, verification)
  to emit fact-only context — the form the MCP `context` tool now returns for
  agents that bring their own instructions. The README, CLI reference, GitHub
  Action, and smoke checks now describe one handoff.
- Changed the CLI help, doctor guidance, contributor setup, and documentation to
  lead with changed-code review while preserving full scan and baseline flows.
- Changed the release workflow to pin third-party Actions and installed tools,
  use job-level permissions, serialize releases per tag, call npm publishing
  directly, and require every official publishing channel to succeed.
- Updated `actions/github-script` to v9, `ignore` to 0.4.26, and `chrono` to
  0.4.45. `actions/cache` remains on v4 for compatibility with older
  self-hosted runners.
- Changed the repository self-gate to reject current P0/P1 findings without a
  committed baseline.
- Removed the unsupported Rust `1.87` MSRV declaration and dedicated CI check.
  RepoPilot `0.x` now builds with the pinned Rust `1.95.0` release/CI toolchain without
  promising compatibility with an older compiler floor.

### Removed

- Removed the `inspect` command group (`explain`, `knowledge`, `cache`, `graph`,
  `feedback`, `rules`, `rule`, `eval-rules`) — maintainer/rule-author diagnostics
  that exposed internal concepts (knowledge catalog, rule registry, context graph,
  cache, feedback validation) as product surface. The engines stay: the Knowledge
  Engine, rule registry, and Context Risk Graph still drive `scan`/`review`/`ai
  context`/`mcp`. The rule quality gate moved fully to `cargo test`
  (`tests/rule_eval_fixture_coverage.rs`); the context graph is observable in the
  `scan --format json` report.
- Removed the `compare` and `doctor` commands and their modules. `compare`
  duplicated the baseline/review change story (a JSON-report diff); `doctor`
  diagnosed a setup surface that barely exists for a single static binary, and
  `scan`/`review` already error clearly. `init` keeps config scaffolding
  (`InitOptions` moved to `src/cli/options/init.rs`). This is an intentional
  breaking change under `0.x`: the `repopilot compare`/`repopilot doctor` CLI
  commands and the (previously doc-hidden) `repopilot::compare`/`repopilot::doctor`
  library modules are gone. Track changes and risk with `scan --baseline` +
  `review` instead — the before/after JSON diff is not coming back as a command.
- Removed the standalone `ai plan` and `ai prompt` commands; their prioritized
  plan and operating-rules output is now folded into `ai context`.
- Removed the preview VS Code extension, platform VSIX packaging, editor CI
  matrix, release assets, and related active documentation.
- Removed the stale repository baseline that still referenced deleted files.

### Fixed

- Made review dependency detection workspace-aware: `DependencyContext` now
  collects local package names from Cargo `[workspace].members`, npm `workspaces`,
  and `go.work` members, so importing one local package from another in a monorepo
  is no longer reported as a newly added external dependency. The repo's own scoped
  name (e.g. `@acme/core`) no longer swallows an unrelated bare `core` import, and a
  manifest that exists but fails to parse is reported once on stderr instead of
  silently making every import look external.
- Hardened the AST `AuthCheckRemoved` and `ErrorHandlingRemoved` review signals to
  match only a call's callee (its `object.method` path), never its arguments or
  string literals, and to count each call once. Removing
  `getUserProfile(session.userId)`, `log("authenticate")`, or a role-mutation such
  as `applyRole(x)` no longer reports a removed auth check, and nested calls no
  longer inflate the count. The Go `if err != nil` and Rust `.map_err`/`.unwrap_or`
  error-handling detectors likewise inspect only the condition/callee.
- Recursion signals now require an AST call to the function itself instead of a
  repeated method name in the function text.
- Behavioral runtime side effects are no longer emitted from tests/fixtures,
  and removed auth/error coarse fallbacks no longer run on prose documents.
- Corrected the taint flow module's doc comment: taint seeding is a single
  document-order forward pass (chains like `a = src; b = a` carry through), not
  the "iterated to a small fixpoint" the header previously claimed.

## [0.16.0] - 2026-06-08

RepoPilot 0.16 makes change review the fast, low-noise default and turns the
existing local engine into a practical PR, MCP, and editor integration surface.
Review signals now have stable machine-readable contracts and an explicit
opt-in gate, while the GitHub Action and VS Code extension consume the same
single-pass JSON/SARIF review.

### Added

- Added taint-lite review signals at `preview` for JavaScript/TypeScript, Python, and Go. `repopilot review` now follows HTTP request fields and process arguments through direct use and simple local assignment chains into raw SQL, subprocess/exec, filesystem-write, and outbound-network sinks. Analysis is intra-procedural, limited to changed sink lines, skips test files, and treats parameterized SQL as safe. Signals join the existing confidence tiers under the new `taint` family and can be disabled with `[taint] enabled = false`.
- Added `review --scope changed|full`, `--profile default|strict`,
  `--fail-on-review none|definitely`, and `--sarif-output PATH`. Changed/default
  is now the standard review; full/strict preserves the previous repository
  audit behavior.
- Added stable review signal IDs, namespaced kinds, confidence, line ranges,
  provenance, suppression state, gate eligibility, and merged evidence lines.
- Added review-signal suppressions by `kind` and path glob in
  `.repopilot/feedback.yml`, with a reason and optional expiry date.
- Added a reusable, fork-safe PR workflow and expanded the first-party Action
  with automatic PR refs, JSON/SARIF artifacts, capped annotations, job
  summaries, typed outputs, verified binary caching, and opt-in sticky comments.
- Expanded MCP stdio support for protocol versions `2025-11-25` and
  `2024-11-05`, structured tool output, output schemas, read-only annotations,
  parse errors, workspace-root confinement, cached last-result resources, a
  rule catalog resource, and review/fix prompts.
- Added a platform-specific VS Code extension under `editors/vscode` with
  bundled checksum-verified binaries, Problems diagnostics, Markdown reports,
  status, commands, and MCP registration.
- Added true-positive and false-positive fixtures for six high-confidence React
  Native rules.

### Changed

- Changed review scope now loads the Git diff once, passes resolved changed
  files into the scanner, preserves repository graph context, and removes
  out-of-diff findings from the report.
- Human review surfaces show at most 20 detailed signals/findings and aggregate
  the remainder; JSON remains complete.
- Review SARIF contains in-diff scan findings and concrete taint issues without
  rerunning the scan.
- Bumped the JSON report schema to `0.18`; readers continue to accept `0.16` and
  `0.17` transition reports.

### Fixed

- Taint-lite now clears taint when a tracked local is reassigned to a clean
  value. Compound assignment keeps taint because it combines with the prior
  value.
- Numeric and boolean coercions such as `Number`, `parseInt`, `int`, `bool`,
  and `strconv.Atoi` now neutralize taint while raw sibling values in the same
  expression remain tracked. Context-specific shell, URL, and HTML sanitizers
  are intentionally not recognized yet.
- `review --fail-on-review definitely` now returns exit code `1` for
  unsuppressed, gate-eligible definitely-sensitive signals. Existing
  `--fail-on` and `--fail-on-priority` remain finding-only gates.
- Malformed MCP JSON now produces a JSON-RPC parse error instead of being
  silently ignored.
- JavaScript `process.exit` in the npm CLI entrypoint no longer creates an
  expected-wrapper runtime-risk finding.

## [0.15.0] - 2026-06-05

RepoPilot 0.15 turns review from *scanning the code* into *understanding the
change*. `repopilot review` now reads the diff itself — security boundaries,
behavioral shifts (added network/subprocess/filesystem/SQL, removed error
handling or auth checks), and algorithmic deltas (deeper nesting, a new nested
loop, a function that grew or became recursive) — and presents them grouped into
three confidence tiers, across the console, Markdown, and JSON outputs and over
MCP to AI agents. Every signal is a structural fact, never a verdict: it flags,
it does not prove. New `repopilot snapshot` + `review --since-snapshot` mark the
repository before an agent or manual change and review everything since — the
deterministic, local check an agent can run on its own edits before handing you
the PR. Security-boundary detection now also reads file content via the shared
syntax tree with token-aware matching. Everything new ships at `preview` and runs
entirely locally — nothing uploaded, no AI service called.

### Added

- Added true-positive and false-positive rule-evaluation fixtures so `inspect eval-rules` now covers `architecture.excessive-fan-out` (including the new Rust-facade false-positive case) and the Django and React Native framework rules (`framework.django.debug-true`, `framework.django.missing-allowed-hosts`, `framework.react-native.{architecture-mismatch, async-storage-from-core, deprecated-api, direct-state-mutation}`, `framework.rn-{navigation-compat, reanimated-compat, gesture-handler-old}`).
- Added `repopilot snapshot` and `repopilot review --since-snapshot` so users can mark the repository before an agent/manual change and then review every committed and uncommitted edit since that marker.
- README now opens with a "first five minutes" AI-assisted quickstart (`doctor → scan → ai context → ai plan → ai prompt → review`).
- `repopilot review` now presents its boundary, behavioral, and algorithmic signals as one **"Review signals (preview)"** block grouped into three confidence tiers — *definitely sensitive*, *maybe sensitive*, and *large diff / noise* — across the console, Markdown, and JSON outputs. The old standalone "Security boundary changed" block is folded into the *definitely* tier. The JSON report gains a `tiered_signals` object (`definitely`/`maybe`/`noise` arrays, each signal carrying its family, tier, path, line, neutral headline, detail, and blast radius) plus a `review.tiered_signals` count summary; the `repopilot_review_change` MCP tool returns the same tiers to agents. Every signal is a structural fact, never a verdict. `boundary_signals` and `review.boundary_missing_test` are unchanged — `tiered_signals` is additive.
- Added behavioral change signals ('removed X' family) at `preview` to review removed error handling blocks (try/catch), deleted or emptied test files, and removed authentication/authorization checks.
- Added behavioral change signals ('added X' family) at `preview` to review newly introduced network calls, subprocess executions, filesystem writes, environment variables, dependency imports, database migrations, and raw SQL queries.
- Added algorithmic change signals at `preview`: for each function the diff touched, `repopilot review` now reports structural deltas against the pre-change version — control-flow nesting got deeper, a nested loop (potential O(n²)) appeared, the function grew past a size threshold, or it became recursive. It reports the structural fact ("nesting 2 → 4"), never a verdict, and skips test files. Computed by comparing the pre/post syntax trees over the changed functions.
- Added `[behavioral]` and `[algorithmic]` config sections (each with `enabled`, default true) so the behavioral and algorithmic review signals can be turned off independently.
- Added security-boundary change signals to `repopilot review` (and the `repopilot_review_change` MCP tool). The review now flags when a change touches parts of the repo that decide *who can do what* (auth, sessions, permissions, CORS) or *how the app ships* (CI/workflows, Dockerfiles, IaC, dependency manifests, committed `.env`). It is path/filename classification over the diff — it flags, it does not prove a change is safe, so a false positive costs only a glance. Signals appear in the console and Markdown output and as a structured `boundary_signals` array in the JSON report. Configure or disable via a new `[security_boundary]` config section (`enabled`, `extra_patterns`). Ships at `preview`.
- Security-boundary detection now also inspects a changed file's *content* via the shared syntax tree, not just its path. When the path alone doesn't reveal a boundary, RepoPilot parses the post-change source and flags access-control decorators/annotations/attributes (e.g. `@login_required`, `#[requires_role(...)]`, `@PreAuthorize`) and auth/CORS library imports (passport, jsonwebtoken, casbin, flask-jwt-extended, Spring Security, …) across JavaScript/TypeScript, Python, Rust, Go, Java/Kotlin, and C#. Matching is token-aware: vague words like `auth`, `role`, and `session` must appear as whole tokens, so `author`, `sessionStorage`, and `security_logger` are not flagged, while high-specificity library/scheme names match as substrings. Still a structural hint that flags, never proves.
- Enriched boundary signals with review context only RepoPilot has on hand: each signal now carries its **blast radius** (how many files import the changed boundary file, from the existing coupling graph), and the review reports `boundary_missing_test` when a *code* boundary (auth / request-trust) changed but no test moved in the same diff. Both surface in the console/Markdown output and the JSON report (`boundary_signals[].blast_radius`, `review.boundary_missing_test`).
- Added a reproducible review demo: `scripts/demo-setup.sh` builds the "small login fix that quietly loosens CORS and touches auth without a test" scenario and `scripts/demo-boundary-review.sh` runs `repopilot review` on it, so the review demo GIF can be recorded reproducibly.

### Changed

- `repopilot ai prompt` now grounds its remediation prompt with a **Repository Facts** block (files analyzed, non-empty lines, per-language file counts, and visible-finding count), a **Non-goals** section that tells the assistant not to fix everything at once or add unrequested commands/flags/schema fields, and a **Required Checks** list (always `repopilot scan`/`review`, plus `cargo fmt`/`clippy`/`test` when the repo contains Rust). The final response heading was renamed from "Final Response Format" to "Expected Output".
- Internal: split nine 300–580-line "god files" into focused submodules so every non-facade source file stays under 300 lines, with no behavior, finding, or public API change. `language_risk::pattern` moved each language's AST node-emitter into its sibling pattern module (`go`/`js`/`python`/`managed`), keeping only dispatch + the shared `RiskPattern`/`push_pattern_finding` infrastructure; `rust_panic_risk` extracted its AST candidate-collection into `ast`; `review::signals::behavioral::removed` extracted its AST walks into `removed_ast`; `commands::graph` moved its console/Markdown/DOT/Mermaid renderers into `graph::render`; `secret_candidate` split finding construction, masking, and JWT detection into an included `finding.rs`; `risk::overlays` moved its derivation helpers into `overlays::support`; and the compact console header detail lines into `compact::header`. Large inline test modules (`rust_panic_risk`, `architecture::barrel_file_risk`, `findings::visibility`) were also split into sibling test files. The cohesive `scan::cache` and `graph` facades are intentionally left as-is.
- Internal: split the `review` orchestrator (329 lines) and the scanner into focused modules, and introduced a small command/output dispatch layer, with no behavior, finding, or public API change. `review` moved its blast-radius, CI-gate, content-signal, and report-assembly helpers into `review::{blast_radius, ci, content_signals, report}`; `scan::scanner` separated the full-repository scan path (`scanner::full`) from the changed-files path (`scanner::changed::{file_analysis, finalize, repo_context}`); and CLI command routing and output-format selection moved into `commands::dispatch` and `output::dispatch`.
- Internal: the `repopilot review` diff parser now captures each hunk's added and removed line text (`ChangedFile.hunks`), not just the changed-line ranges it derived before. This is the foundation for content-based review signals; `ranges` and all current output are unchanged and `hunks` is not serialized into the JSON report.
- Internal: added a review-layer content bridge (`review::signals::content`) that re-reads a changed file's pre- and post-change source — from the working tree, or `git show <ref>:<path>` for a ref range — and pairs it with the shared tree-sitter parse view, so content-based signals can analyze each side of a change without keeping scan-time file content in memory.
- Internal: `repopilot review` now unifies its boundary, behavioral, and algorithmic signals into three confidence tiers — *definitely sensitive*, *maybe sensitive*, and *large-diff-or-noise* — on `ReviewReport.tiered_signals`, wired through the review pipeline and reach-enriched from the coupling graph. A network call added inside an auth/request-trust file is escalated to *definitely*; a large diff with nothing flagged surfaces one volume note. `boundary_signals`/`boundary_missing_test` are unchanged; the tiered view is rendered across the console, Markdown, and JSON outputs (see the Added entry above).
- Repositioned and tightened the README around reviewing a change *before you merge* — for humans and for AI agents calling RepoPilot over MCP. The page now leads with the review/boundary workflow and the agent (MCP) angle, demotes `scan` and the rest to a compact capability table, and moves the encyclopedic sections (trust mode, commands, rule quality, reports, CI) to their `docs/` links. Cut from ~326 to ~124 lines, deterministic/local-first framing made explicit.
- Reworked the README visuals to a single hero demo plus real, current console-output examples (scan, review with boundary signals, and a baseline gate catching a newly introduced secret) instead of a GIF per command. Removed the per-command GIFs, including a misleading baseline GIF that showed an already-adopted repo rather than the gate doing its job.

### Fixed

- `architecture.excessive-fan-out` no longer flags pure Rust facade modules — `lib.rs`/`mod.rs` files whose body is only `mod`, `use`, `pub use`, or `extern crate` declarations. A barrel/facade that re-exports many submodules is intentional structure, not change-amplifying coupling, so it is no longer reported by `repopilot scan` (or the `repopilot_scan` MCP tool). The threshold and behavior for real orchestration files that import many internal modules are unchanged.
- Hardened `repopilot review`'s coarse (non-AST) behavioral fallback, which scans raw diff lines when a changed file can't be parsed. It now matches whole tokens instead of substrings, so a removed `author` line no longer reads as a removed `auth` check and a `catchy` variable no longer reads as a removed `catch` block (high-specificity auth substrings like `oauth`/`jwt`/`authentic`/`authoriz` still match). Coarse fallback signals are also demoted out of the *definitely sensitive* tier into *large diff / noise* — they remain visible as hints but no longer sit beside AST-confirmed findings. AST-based detection on parseable files is unchanged.
- `repopilot review` no longer raises a security-boundary signal for *test* files. A change to `auth_service_test.ts` previously matched the access-control boundary on its filename and landed in the *definitely sensitive* tier, even though a test for a boundary decides neither who-can-do-what nor how the app ships. Boundary detection now skips test files; whether a test *moved alongside* a code-boundary change remains a separate signal (`review.boundary_missing_test`), which is unchanged.
- `architecture.large-file` no longer flags stylesheet and markup files (CSS, SCSS, HTML) as oversized source modules. They are now treated as non-logic the same way JSON, YAML, TOML, and Markdown already were — the rule applies only to logic languages — so a long stylesheet is no longer reported as a large file by `repopilot scan` (and the `repopilot_scan` MCP tool). The line-count threshold and behavior for real source files are unchanged.

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

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.16.0...HEAD
[0.16.0]: https://github.com/MykytaStel/repopilot/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/MykytaStel/repopilot/compare/v0.14.0...v0.15.0
[0.14.0]: https://github.com/MykytaStel/repopilot/compare/v0.13.0...v0.14.0
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
