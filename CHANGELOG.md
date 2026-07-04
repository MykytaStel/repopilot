# Changelog

All notable changes to RepoPilot will be documented in this file.

The format is based on Keep a Changelog, and this project follows Semantic Versioning while the public API is still evolving under `0.x`.

## [Unreleased]

### Added

- v0.20 roadmap, performance benchmark matrix, and release scorecard define the
  product, engineering, and compatibility contract before implementation starts.

### Changed

- Every registered rule now declares an internal `RuleRequirements` contract covering execution scope, required fact kinds, lifecycle, cache policy, and produced output. The generated rules reference and rule catalog expose the summary, while findings and scheduler behavior remain unchanged.
- The scan fact model now acts as a shared `FactStore` and indexes one
  `ParsedArtifact` per analyzed source file. Artifacts preserve imports,
  runtime-deferred imports, conservative export facts, file-role evidence, and a
  compact syntax summary from the same parse-once view used by file audits.
  Existing findings, output schemas, CLI behavior, baselines, and MCP contracts
  are unchanged.
- Scan, review, AI-context, baseline, and MCP analysis requests now execute through
  one immutable `AnalysisSession` that owns the target path, resolved workspace
  root, workspace revision, repository/scan configuration, and visibility profile.
  Existing CLI output, report schemas, finding IDs, baselines, and MCP tools are
  unchanged.
- Changed-scan context graph cache hits now patch cached graph nodes and outgoing
  edges for added, modified, and deleted files instead of rebuilding dependency
  edges from every cached node. Existing findings, output schemas, cache files,
  and CLI behavior are unchanged.
- GitHub Release notes now come from structurally validated curated
  `docs/releases/v*.md` files while `CHANGELOG.md` remains the full technical
  release journal.

## [0.19.0] - 2026-06-29

### Added

- **MCP can now explain emitted findings by stable ID.**
  `repopilot_explain_finding` reads a file-scoped finding from the latest scan
  or review in the current MCP session, replays its stored Knowledge Engine
  inputs against the current workspace, and returns the emitted finding, stored
  provenance, full role/decision trace, and a deterministic `matched` or
  `drifted` comparison. Repository, workspace, Git-diff, and framework-project
  findings are rejected rather than replayed with incomplete file-only context.
  The tool rejects findings without schema-`0.20` decision provenance, enforces
  the MCP root boundary, and bypasses the session cache so a newer scan or review
  cannot reuse a stale explanation. If the same stable ID appears more than once
  in a report, callers can pass `evidence_path` and `line_start` to select one
  occurrence; ambiguous calls return structured candidates. CLI commands,
  scan/review schemas, finding IDs, baseline behavior, visibility, SARIF, and
  detector decisions are unchanged.

- **Findings now preserve replayable Knowledge Engine decision provenance.**
  Knowledge-aware findings add optional `provenance.knowledge_decision` data
  containing base severity, signal id, action, decided severity, and reason.
  Enrichment and detector provenance overrides preserve the replay record.
  Scan/review report schema moves from `0.19` to `0.20`; readers continue
  accepting `0.16` through `0.19`. Finding IDs, baseline matching, visibility,
  risk scoring, SARIF behavior, and detector decisions are unchanged.

- **Rule decisions now expose an ordered, evidence-backed trace.** The Knowledge
  Engine has one trace-producing path that records rule lookup, configured
  applicability checks, the supplied base severity, every override in declaration
  order (including unmatched and test-skipped entries), severity transitions, and
  the final decision. Existing `decide*` APIs remain wrappers over the same path,
  so explanation cannot drift from scan behavior. Normal scan and review paths
  keep trace recording disabled and do not allocate trace strings or vectors.
  `repopilot_explain_file` now adds file-role evidence from the context
  classifier, including executable-package manifest context, a final
  default-profile visibility step, and explicit scope limits: repository graph context,
  `repopilot.toml` rule overrides, local feedback, baseline state, and full scan
  filtering are not applied. The MCP tool uses detector-specific signal base
  severity when known, then falls back to registry default severity for known
  rules. This is additive explain/MCP JSON; scan, review, SARIF, baseline,
  receipt, finding IDs, severity decisions, and visibility behavior are
  unchanged.

- **File-role classification now preserves explainable evidence.** The detailed
  classifier records one typed evidence entry for every assigned role, including
  whether it came from a path, content/framework marker, package manifest,
  shared context signal, mixed evidence, or the unknown-role fallback.
  Changed-scan file-role caches persist this evidence under cache schema v7 so
  later explain/decision-trace surfaces can show why a role matched without
  copying classifier heuristics. Existing role decisions, finding severity,
  visibility, and report schemas are unchanged.

- **The real-repo zoo now records snapshots and human-reviewed dispositions.** A
  curated manifest of 11 real open-source repositories is scanned reproducibly
  via `scripts/zoo.py`, with default-visible snapshots committed under
  `tests/zoo/snapshots/`. A tracked `tests/zoo/expectations/<repo>.toml` layer
  (one versioned file per manifest repo) sits alongside those snapshots:
  snapshots answer *what RepoPilot emitted*, expectations answer *how a human
  reviewed it*. Every default-visible finding must be exhaustively labeled
  `actionable`,
  `valid-but-accepted`, or `false-positive` with a non-empty reason; selected
  strict findings act as `selective` recall anchors that fail if they disappear.
  `python3 scripts/zoo.py scan` now validates labels and reports snapshot drift
  and expectation failures (unlabeled / stale / duplicate / invalid / ambiguous /
  missing-anchor / known-false-positive) on separate channels; `triage` prints
  unlabeled findings with evidence and a copy-paste `REVIEW_REQUIRED` skeleton
  (never guessing a disposition); `report` adds a reviewed signal-quality summary
  (actionable/accepted/false-positive totals, a reviewed precision *proxy*, and an
  actionability rate). The matcher disambiguates findings that share a stable id
  via line, so distinct occurrences are never merged. Labels never suppress
  analyzer output — `false-positive` is surfaced as calibration debt, not a
  suppression — and `--bless` never writes human labels. Hermetic
  `zoo_expectations_contract` test covers the pure parse/match/render helpers; the
  real zoo stays opt-in behind `REPOPILOT_ZOO=1`.

- **`repopilot ai context --format json`** emits a structured, deterministic JSON
  handoff so agents get the same facts the Markdown brief carries without parsing
  Markdown, matching the JSON the MCP tools already return. Each finding includes
  the evidence an agent needs — stable id, severity/confidence, `risk` (score,
  priority, signals), description, recommendation, docs URL, and the full
  `evidence` list (path, line range, snippet) — not just a title. Like the
  Markdown brief it is **budget-aware**: findings are ordered by risk and added
  until the serialized output reaches `--budget`, and the document reports
  `truncated`, `findings_included/omitted`, and `plan_clusters_included` so the
  budget is honest rather than advisory. `approx_tokens` is measured from the real
  (pretty) output. `risk.level` is a machine token (`moderate`) alongside the
  display `risk.label` (`🟡 MODERATE`). Markdown stays the default; `--no-header`
  and `--no-task` apply to Markdown only (JSON is always fact-only), and JSON
  never mixes the stderr token breakdown in. Pinned by a golden snapshot and
  end-to-end CLI tests.

### Changed

- **The `repopilot_context` MCP tool now includes the repository facts summary**
  (aggregate file, line, and language stats), matching `repopilot ai context`. The
  tool previously rendered without facts despite documenting a fact-only brief, so
  agents calling it received a thinner context than the CLI; they now get the same
  aggregate stack/size picture.

- **Release verification now runs both performance guard scripts.** The existing
  full-scan smoke benchmark is wired into `npm run scan:performance` and the
  local release gate now runs it next to the changed-review performance check, so
  the release checklist matches the shipped scripts.

- **Agent-facing docs now document the false-positive/noise-reduction workflow.** MCP docs and AI-context workflow docs now call out strict-mode recall, exact duplicate aggregation, changed-scan limits, and the need for false-negative guard tests before hiding or downgrading signals.

- **`language.javascript.runtime-exit-risk` downgrades `process.exit(...)` in a
  CLI tool's command handlers instead of surfacing them as a default High.** A
  command handler — a file in a `commands/` directory whose package declares an
  executable (`package.json#bin`, or a Cargo `[[bin]]`/`src/bin` target) — owns
  its own exit code, so its `process.exit` is the intended boundary the rule's
  own guidance asks for. The scanner resolves each file to its *nearest* package
  and, when that package is a CLI tool and the file is a command handler, tags it
  with a new `cli-executable` role; the knowledge pack then **downgrades the
  finding to Low** (hidden by default, still shown under `--profile strict`)
  rather than suppressing it in the detector, so the signal stays recoverable.
  The exemption is deliberately narrow — it pairs path evidence (`commands/`)
  with manifest evidence (`bin`), so a CQRS-style `domain/commands/` in a non-CLI
  package, and a reusable module (`lib/`, `utils/`) inside a CLI package, both
  keep their default-visible High. Measured on the real-repo zoo, this took the
  `ignite` CLI from 9 default-visible findings to 0 (all retained as Low in
  strict), with no loss of the genuine library-level exits elsewhere.

- **`language.python.exception-risk` no longer surfaces `assert` and `raise
  NotImplementedError` by default.** An `assert` (commonly type-narrowing or an
  internal invariant) and `raise NotImplementedError` (the idiomatic
  abstract-method declaration) are overwhelmingly intentional, so the knowledge
  pack now downgrades both to low — hidden in the default profile, still shown
  under `--profile strict`. The broad `except:` handler stays the default-visible
  signal. Measured on the zoo, default-visible exception-risk findings dropped
  from 107 to 1 (fastapi 63→0, wagtail 44→1), with all signals retained in strict.

- **`language.javascript.runtime-exit-risk` no longer flags `throw new
  Error(...)`.** A `throw` is recoverable control flow, not a runtime exit, and a
  generic thrown error is idiomatic everywhere — yet the rule flagged every one
  in any file under a `packages/`/`lib/`/`core/` path, so in monorepos it fired
  on essentially every module. The rule now matches only `process.exit(...)`
  (the genuine host-process termination) and is retitled "Risky JavaScript
  runtime exit". Measured on the real-repo zoo, this removed 113 of 124 findings
  (excalidraw 76→1, changesets 38→1) with no loss of true signal.

### Fixed

- **Workspace-dependent MCP tools no longer return stale session results.**
  Removed the arguments-only in-process result cache from
  `repopilot_review_change`, `repopilot_context`, and
  `repopilot_explain_file`. Each call now observes current source files,
  manifests, configuration, feedback, ignore state, and Git changes, while
  successful scan/review calls keep `last-scan` and `last-review` synchronized
  with the exact returned result. `repopilot_scan` retains its separate
  persistent cache with Git/config/input fingerprint validation. MCP schemas,
  CLI behavior, findings, baselines, SARIF, and report schema `0.20` are
  unchanged.

- **Finding analysis scope now comes from the audit execution boundary.**
  File, project, and framework runners stamp `file`, `repository`, and
  `framework-project`; graph/coupling paths explicitly stamp repository scope.
  Registry enrichment preserves this value instead of deriving it from
  `signal_source`, so project text heuristics such as
  `architecture.too-many-modules` cannot enter file-only replay. Signal source,
  severity, confidence, visibility, finding IDs, schema `0.20`, baseline
  behavior, and detector decisions are unchanged.

- **Real-repo zoo scans now build and run the current workspace RepoPilot by
  default.** `python3 scripts/zoo.py scan` invokes Cargo once, uses the
  executable Cargo reports, and prints scanner provenance before scanning so
  stale `target/release` or `target/debug` artifacts can no longer make the gate
  pass silently. Explicit external binaries remain supported with `--repopilot`,
  but they are version/report-metadata checked and require
  `--allow-version-mismatch` for intentional cross-version runs.

- **`language.javascript.runtime-exit-risk` (and other generated-file-aware
  rules) no longer flag vendored/generated bundles such as Emscripten/WASM
  glue.** An Emscripten `process.exit` shim in compiled WASM bindings is not the
  reusable-library runtime-exit hazard the rule targets — the file is generated
  output, not hand-maintained source. Generated-file detection now recognizes
  these bundles by their Emscripten runtime marker or a bare, whole-file
  `/* eslint-disable */` banner as the first line (a signature hand-written code
  does not use), classifying them with the existing `generated` file role so the
  rules that already opt into `suppress_generated` skip them. A scoped
  `eslint-disable` for specific rules (or one that is not the first line) is left
  as ordinary production code. Measured on the real-repo zoo, this removed
  excalidraw's default-visible `woff2-bindings.ts` runtime-exit finding.

- **`security.secret-candidate` treats an Algolia DocSearch `apiKey` as a
  Low-confidence (strict-only) finding instead of a default-visible secret.** The
  frontend search widget is configured with Algolia's **search-only** API key,
  which is public by design — it ships in the browser bundle — so flagging it as a
  committed secret is noise. A DocSearch block is identified by its signature trio
  (`algolia` + `appId` + `indexName`); only the `apiKey` field in such a file is
  downgraded, so a real credential elsewhere in the same config still flags High,
  and the value remains visible under `--profile strict`. Measured on the
  real-repo zoo, this removed excalidraw's default-visible Docusaurus search-key
  finding while retaining strict-profile review.

- **`language.managed.fatal-exception-risk` no longer surfaces placeholder/fatal
  throws in build-tooling or test-support modules by default.** A `TODO()` or
  `throw` in a Gradle convention plugin (`build-logic/` / `buildSrc/`) fails the
  build by design, and one in a dedicated test-double module (a `testing` source
  tree such as `core/testing/`, which lives under `src/main` so it is not a test
  file) is test plumbing — neither is the runtime hazard the rule targets. Two
  file roles, `build-tooling` and a managed-language `test-support` extension
  (previously Rust-filename only), tag these modules; the knowledge pack
  downgrades the finding to Low for both — hidden by default, kept under
  `--profile strict` — while the file keeps its production role for every other
  rule. Changed-scan file-role caches are bumped so upgrades do not reuse stale
  role classifications. Genuine unfinished application code (e.g. a
  `TopicUiState.Error -> TODO()` in a feature screen) stays default-visible.
  Measured on the real-repo zoo, this took the nowinandroid Android app from 3
  default-visible findings to 1, with all signals retained in strict.

- **`architecture.circular-dependency` no longer counts TypeScript/JavaScript
  type-only imports/exports as runtime cycle edges, including inline specifiers
  like `import { type A, type B }`.** The compiler erases type-only imports, so a
  cycle that exists solely through them is not a real dependency in any sense. A
  module imported type-only (and never as a value — a mixed `import { type A, B }`
  or a dynamic `import()` keeps the runtime edge) is subtracted from cycle
  detection while staying a full edge for coupling, fan-out, and dead-module
  analysis. Changed-scan file-role caches and context-graph caches are bumped so
  upgrades do not reuse stale TS/JS graph edges from previous versions. Unlike a
  Python function-body deferred import — which runs lazily and so is still
  surfaced as an informational Low cycle — a purely type-only cycle is erased at
  compile time and is suppressed outright, in the default profile and under
  `--profile strict`. Measured on the real-repo zoo, this removed wagtail's four
  default-visible type-only cycles (its `CommentApp`/`Sidebar`/`StreamField`
  client components, 6 → 2 default; 15 → 11 strict), with the genuine Python
  cycles retained.

- **`security.env-file-committed` is now content- and confidence-aware for shared
  `.env.development` / `.env.production` / `.env.staging` files.** Local `.env`
  and `.env.local` files are still flagged on sight, but shared build variants are
  inspected: ordinary public build config is skipped, ambiguous public browser
  credentials such as Firebase web config are Low confidence (hidden by default,
  retained in strict), and explicit sensitive keys such as `PASSWORD`,
  `CLIENT_SECRET`, `AUTH_TOKEN`, or `PRIVATE_KEY` stay High confidence even with a
  public framework prefix like `VITE_`/`NEXT_PUBLIC_`. Measured on the real-repo
  zoo, this removed both of excalidraw's default-visible env-file findings while
  retaining strict-profile review of the public credential-shaped config and still
  flagging genuine committed secrets.

- **`security.secret-candidate` no longer misreads namespace path qualifiers as
  hardcoded secrets, and lowercase slug-like values are strict-only Low.** A
  `key::Rest` path qualifier (a Rust/C++ enum or type path such as
  `Token::RecursiveSuffix`) was parsed as a `token: <secret>` assignment. Values
  made only of lowercase words joined by `-`/`_` (for example
  `excalidraw-oai-api-key`) are now downgraded to Low confidence instead of
  removed, so default output hides storage identifiers while `--profile strict`
  still retains passphrases and token-like slugs. Genuine mixed-case/digit
  credentials stay High and default-visible. Measured on the real-repo zoo, this
  removed ripgrep's 2 default-visible findings (`glob.rs` enum paths) and hid 1
  excalidraw storage-key slug from default output while retaining it in strict;
  eShopOnWeb's 7 genuine hardcoded credentials were correctly retained.

- **`repopilot_scan` MCP disk caching no longer serves stale scan reports after
  agent edits.** The scan tool now bypasses the MCP session cache, keys disk
  entries from the git-root working tree, stores entries under Git-owned
  metadata instead of the working tree, and invalidates when a changed-scope base
  ref resolves to a different commit or when path-discovered config,
  `.repopilot/feedback.yml`, `.repopilotignore`, or effective Git/ignore-file
  inputs change. Corrupt cache files are ignored and recomputed, and retention
  keeps at most 32 valid scan reports per repository cache directory.

- **Scan pipelines now collapse only exact duplicate finding emissions, not stable-baseline-key collisions.** Full and changed scans aggregate repeated emissions for the same rule, file, line, snippet, and compatible metadata before risk scoring, preserving a single finding with distinct evidence. The guard deliberately ignores `Finding::id` as a dedupe identity because that id is line-insensitive and literal-normalized for baseline stability; separate occurrences such as two hardcoded secrets on different lines remain separate findings and still contribute independently to counts, hotspots, and risk overlays.

- **`testing.source-without-test` no longer flags documentation example code or standard Python entrypoints.** A `docs_src/` tree (an unambiguous documentation name) is skipped wherever it appears, and a `docs/` directory is skipped at a package root (repo root or a monorepo package such as `packages/<pkg>/docs/`) — but a `docs` module *nested inside a source tree* (`src/docs/`, `lib/docs/`, `app/docs/`) stays visible, since in a document-management app that is ordinary production code. The standard Django entrypoints `manage.py`/`wsgi.py`/`asgi.py` are also exempt (the analogue of the already-skipped `__main__.py`). A bare `apps.py` is deliberately *not* skipped by name: the filename alone is not evidence of a declarative `AppConfig` stub (it can carry a `ready()` with real startup behaviour, or be an ordinary module), so skipping it would hide untested production code. Measured on the real-repo zoo, this took fastapi's strict-profile count from 389 to 36 (its `docs_src/` tutorial tree was 349 of those findings) and wagtail's from 671 to 668, with no change to any default-visible output.

- **`architecture.circular-dependency` now handles Python deferred imports consistently across full scans and changed-scan cache hits.** Deferred imports that resolve to a file already reached by an eager import no longer strip that eager edge, context-graph and per-file caches now preserve `deferred_imports`, and deferred-only cycles are retained as Low severity so they are visible under `--profile strict` without becoming default-visible module-load cycles. The per-file cache schema was bumped (3→4) so a cache written before `deferred_imports` existed is rejected and re-analyzed rather than rehydrated with an empty list — which would have dropped the deferred edges and resurrected the phantom cycle on the next cache hit. A strongly-connected component that only forms in the full graph because a deferred edge widened a component that already contains an eager cycle is no longer labelled "deferred-only": the eager sub-cycle is reported as High and no misleading Low finding is emitted for the larger component.

- **`architecture.circular-dependency` no longer reports cycles that exist only through a deferred (function-body) import.** Importing a module *inside a function* is the idiomatic way Python breaks a load-time import cycle — the import runs only when the function is called, so it forms no module-load cycle. The coupling graph still records these as edges (a real, if lazy, dependency that `dead-module`/`excessive-fan-out` must see), but cycle detection now subtracts them, so a deferral is no longer flagged as the very cycle it was written to break. A module imported both eagerly and lazily keeps its eager edge. Measured on the real-repo zoo, this took wagtail from 15 default-visible circular-dependency findings to 6 (all genuine module-level cycles) with `dead-module`/fan-out counts unchanged; fastapi's count rose 1→2 as a real `_compat` cycle, previously masked inside a larger strongly-connected component, is now reported on its own.

- **`language.rust.panic-risk` downgrades a type-inferred `"literal".parse().unwrap()` to Low instead of a visible High.** Parsing a string *literal* (e.g. the default colour/spec tables many crates build with `"path:fg:magenta".parse().unwrap()`) parses authored input — it can still panic (`"999".parse::<u8>()`), but as a deterministic bug caught on the first run, not a runtime panic from external input — yet the `.parse(` external-input signal pushed these to a visible High. The rule now **downgrades** such a call to Low (hidden in the default profile, kept under `--profile strict`) rather than removing it, via a structural syntax-tree check plus a text fallback for the macro form (`vec!["x".parse().unwrap(), …]`) whose body is an unparsed token tree. Only the type-inferred `.parse()` is downgraded: an explicit `.parse::<T>()` is a deliberate typed conversion and a `.parse()` on a runtime value (`raw.parse()`) both keep their default-visible severity. Measured on the real-repo zoo this hid all 5 false positives in ripgrep's `printer/color.rs` from the default profile.

- **`language.rust.panic-risk` no longer treats `panic!`/`unwrap` in Rust test-support modules as production risk.** A `testutil.rs`/`test_utils.rs`/`test_support.rs`/`test_helpers.rs` file is test infrastructure compiled in normal builds (so not behind `#[cfg(test)]`), where `panic!`/`unwrap` are assertion plumbing. Rather than reclassify it as a *test file* globally (which would change every rule), the scanner now tags it with a dedicated `test-support` role *alongside* its production role, and only the panic-risk knowledge pack opts in: `unwrap`/`expect`/`panic!` are **downgraded to Low** — hidden in the default profile but kept under `--profile strict` (the high-recall mode), since the module still compiles in normal builds and the filename alone is not enough evidence to delete a finding outright. Every other rule still sees a normal production file. The collision-prone `test_*` prefix is excluded so production modules like `test_edges.rs` are unaffected. This hid the 5 false positives in ripgrep's `searcher/src/testutil.rs` from the default profile; combined with the literal-parse fix, ripgrep drops from 14 default-visible panic-risk findings to 4 (the genuine library-level `panic!`/`unwrap` calls) while strict still surfaces all of them.

- **`security.secret-candidate` no longer flags high-entropy tokens embedded inside URL values.** A Firebase Storage download URL contains a `?token=<UUID>` query parameter that looks exactly like a secret assignment, but a URL is not a hardcoded credential. The detector now checks whether a matched key (e.g. `token`) falls inside a URL string literal on the same line (between an opening quote and the key, there is a `https://` or `http://` prefix) and skips it. This removed 25 false positives from the nowinandroid zoo repo (all from `topics.json` Firebase image URLs) while leaving eshoponweb's 7 legitimate hardcoded secrets (JWT keys, passwords, connection strings) untouched. URL-embedded patterns like `?alt=media&token=<UUID>` are now silently ignored; a real `auth_token: "fixtureToken-..."` assignment outside a URL still fires.

- **`security.secret-candidate` no longer flags `SCREAMING_SNAKE_CASE` env-var names passed as string literals.** `envvar="GITHUB_TOKEN"` — an argument that references an env var by name — was treated as a secret value because `token` matched a key and `GITHUB_TOKEN` passed the entropy test. An all-caps identifier that contains an underscore is now recognized as an env-var name reference, not a secret value. The underscore requirement is deliberate so that genuinely all-caps alphanumeric secrets without underscores (e.g. an AWS access key id `AKIA…`) still fire.

- **`security.secret-candidate` skips files whose path contains `tutorial`.** Tutorial and docs-src files in frameworks like FastAPI (`docs_src/security/tutorial*.py`) intentionally contain example secret keys that are teaching material. The path-based skip list (already covering `test`, `example`, `fixture`, `mock`) now also covers `tutorial`, removing 4 FPs from fastapi without affecting any real credential detection.

- **`architecture.package-boundary-violation` no longer flags deep imports into
  a package that publishes a wildcard `exports` subpath.** When a workspace
  package's `package.json` declares `"exports": { "./*": ... }`, the author has
  explicitly published every subpath as public API, so a sibling importing
  `@scope/pkg/foo` (resolved to `pkg/src/foo`) is not reaching past a boundary.
  The detector now reads the `exports` map during workspace detection and
  suppresses violations into such packages. Caught by the real-repo zoo:
  excalidraw dropped from 317 false positives to 0; packages without wildcard
  exports keep their boundary.

## [0.18.0] - 2026-06-18

### Fixed

- **`language.rust.panic-risk` no longer escalates a literal
  `Regex::new("…")`/`Selector::parse("…")` to a visible High.** A regex or CSS
  selector built from a string *literal* only fails on a malformed
  compile-time pattern — a deterministic programmer error any test catches, not
  a runtime panic risk from external input — yet on scraper-shaped files the
  context-driven escalation pushed these to a **visible High**. A structural
  syntax-tree check now skips an `unwrap`/`expect` chained onto a literal
  `Regex::new`/`Selector::parse` (handling the multi-line
  `Regex::new(\n    r"…",\n).unwrap()` form the previous text heuristic could
  not see); a pattern built from a runtime value still stays a panic risk.
- **`architecture.dead-module` no longer claims High confidence on monorepo
  files whose importers it cannot resolve.** The "nothing imports this file"
  safety-net only weakened its claim on unresolved *relative* imports, so a
  workspace/monorepo whose internal imports are absolute or package-style
  (`from app.services import x`, the `@/…` path alias, multi-crate Rust paths)
  looked like a fully-resolved graph and every unreferenced file was reported at
  **High**. The resolver now also records an unresolved import as graph-incomplete
  evidence when it is a recognized alias or its leading segment names a directory
  that actually exists in the repository — a workspace package it could not wire
  up — while genuine third-party packages (`react`, `numpy`) stay out. With the
  graph provably incomplete the finding drops to **Low** confidence (the prior
  demotion silently collapsed back to High because `Medium` is the registry's
  "use the default" sentinel), and it is suppressed entirely when an unresolved
  import's final path or dotted segment matches the candidate file's name. On a
  sampled monorepo this moved 61 dead-module findings from High to Low; a cleanly
  resolved repository is unaffected and keeps High-confidence claims. Resolved
  edges (cycles, fan-out) are unchanged — only absence claims are weakened.
- **Rust production modules named `test_*.rs`/`*_test.rs` are no longer silently
  dropped from the scan.** The low-signal path filter applied the singular
  `test_`/`_test` name conventions (Python/Go/JS) to `.rs` files too, so ordinary
  Rust modules such as `test_edges.rs` and `source_without_test.rs` were excluded
  from every audit and from the import graph — matching the gap the refined
  `is_test_file` already closed. For `.rs` files only the plural Rust forms
  (`tests.rs`, `*_tests.rs`) and the `tests/` directory now mark a file as
  low-signal; non-Rust conventions are unchanged.
- **`security.secret-candidate` no longer flags shell expansions or
  inline-test fixtures.** A value that is a shell command substitution
  (`token="$(...)"`, backticks) or a variable expansion is a captured value,
  not a literal secret — and the value is now extracted from its first quoted
  segment, so a trailing line continuation (`PASSWORD="$PGPASSWORD" \`) or a
  Rust `.to_string()` suffix no longer defeats placeholder detection. Secret
  candidates inside an inline `#[cfg(test)]` module (e.g. `password: "short"` in
  a unit test) are also skipped now: the file-path test skip only caught whole
  test files, missing inline test modules in production sources. Real
  assignments in production code are unchanged.
- **`architecture.dead-module` no longer flags Cargo build scripts or
  React/Vite entrypoints.** Entrypoint detection is consulted by the import
  graph after per-file content has been dropped, so it must work from the
  filename alone — but the content-only `fn main()` check meant a `build.rs`
  (`fn main()` present, content unavailable) was reported as a dead module, and every `src/main.tsx`/`index.tsx` Vite/React entry was treated as ordinary
  importable code. `is_app_entrypoint` now recognises `build.rs` and the
  `.tsx`/`.jsx` entry filenames by name, independent of content.
- **`language.rust.panic-risk` no longer escalates a serde *serialization*
  unwrap to High.** `serde_json::to_string(&value).unwrap()` (and `to_value`,
  `to_vec`, `to_writer`, and the YAML/TOML equivalents) serializes an owned,
  in-memory value — an in-process, effectively-infallible operation — yet the
  `serde_json`/`json` external-failure signals escalated it to a **visible
  High** in the default profile, the same way a genuine parse of untrusted
  input is. On real services that persist structured data this was the dominant
  panic-risk false positive. A serialization unwrap is now the ordinary
  (Medium, hidden-by-default) panic risk; *deserialization*
  (`from_str`/`from_slice`/`from_reader`/`from_value`) still escalates to High
  and stays visible.
- **`language.rust.panic-risk` no longer escalates a poisoned-mutex unwrap to
  High just because the lock is named `db`.** A `Mutex`/`RwLock` field called
  `db`/`conn`/`pool` (`self.db.lock().unwrap()`) matched the external-failure
  keyword list (`db.`, `conn.`, `pool.`) and was reported as a **visible High**
  finding in the default profile, even though a lock acquisition is an in-process invariant, not an external I/O or database call. A `.lock()` unwrap
  is now treated as the ordinary (Medium, hidden-by-default) panic risk it is;
  real external-failure unwraps (`.parse()`, `fs::`, query/request paths) still
  escalate to High and stay visible.
- Default finding visibility is now lifecycle- and provenance-aware: low-confidence
  and experimental heuristics stay hidden even when risk scoring ranks them highly, while stable import-graph risks, validated security findings, direct
  runtime evidence, and High-confidence manifest-backed package-boundary
  violations remain visible. `--profile strict` still preserves the complete
  finding set and existing finding IDs/report schemas are unchanged.
- **Rust inline tests no longer poison file-role classification.** A file was
  treated as a *test file* whenever it carried inline tests
  (`has_inline_tests`), but in Rust a `#[cfg(test)] mod tests` block is
  idiomatic in ordinary production modules. This mislabeled most of the codebase
  as test code and made every production-to-production import look like a leak:
  on RepoPilot's own tree `architecture.test-leak` dropped from **294 findings
  to 0**. File role is now decided purely by path/name conventions; the
  inline-test flag stays a separate coverage signal. The singular `test_*`/
  `*_test.rs` Rust name patterns (which are Python/Go/JS conventions and collide
  with production modules like `test_edges.rs` and `source_without_test.rs`) no
  longer classify a `.rs` file as a test — real Rust test files use `tests/`,
  `#[cfg(test)]`, or the plural `*_tests.rs`/`tests.rs`.
- **`architecture.dead-module` no longer flags non-code files.** Docs, config,
  stylesheets, lockfiles, images, and shell scripts (Markup / Shell / Unknown
  language families) are never "imported", so they always had `fan_in=0` and
  were all reported as dead — on real repositories this meant every
  `.claude/*.md`, `*.css`, `*.json`, lockfile, and image. The rule now only
  considers languages the resolver actually wires up (Rust, TS/JS, Python, Go,
  JVM). On three sampled multi-language repos this removed 72–86% of the
  `dead-module` findings.
- **`architecture.dead-module` no longer flags live modules wired with
  `#[path = "..."] mod x;` or `include!("...")`.** The Rust import resolver now
  follows both, so files such as `language_risk/{go,js,managed,python}.rs` and
  the `output/*/sections/*.rs` include-targets are correctly seen as reachable.
  Files that carry their own inline tests are also exempt from `dead-module`,
  since their only importer is often a `#[cfg(test)]` declaration whose edge is
  excluded from the production graph.
- **`language.rust.panic-risk` no longer reports `panic!`/`unwrap`/`expect`
  inside `#[cfg(test)]` blocks.** Test bodies are expected to panic; the AST
  walk now skips `#[cfg(test)]`/`#[test]` items so only production panic risks
  remain.
- Imports gated by `#[cfg(test)]` are kept out of the import graph entirely, so
  test-only `use` statements can no longer form phantom
  `architecture.circular-dependency` edges between production modules.
- `architecture.deep-directory-nesting` now measures nesting depth relative to
  the scan root, so scanning a repository by an absolute path no longer counts
  the directories above the repository and over-reports every file as deeply
  nested.

### Changed

- `architecture.package-boundary-violation` now **auto-enables on a detected
  workspace** — npm/yarn, pnpm, Cargo, or Go (`go.work`). Each workspace package
  is treated as a boundary, so reaching into another package's internals (anything but its `index.ts`/`mod.rs`/`lib.rs` public entry) is flagged
  without any configuration, at **High** confidence because the boundary is
  declared by the repository's own manifests. Explicit `[architecture]
  package_roots` still take priority and keep the Medium default confidence;
  with neither a workspace nor config the rule stays silent. A non-workspace repository (including RepoPilot itself) is unaffected.
- Configuration discovery now walks up from the current directory to the git
  root (stopping at `.git` or the filesystem root) looking for `repopilot.toml`,
  so running RepoPilot from a subdirectory picks up the repository's config
  instead of silently using defaults. An explicit `--config <path>` still wins
  and is never affected by discovery.
- Architecture audits (`circular-dependency`, `excessive-fan-out`, `dead-module`, `test-leak`, `layer-violation`, `package-boundary-violation`) now pinpoint the exact import statement lines in their evidence snippets instead of defaulting to line 1, using language-aware AST extraction across all supported languages.
- Internal: rewrote the strongly-connected-components (Tarjan) search from a recursive `visit` to an explicit loop with a call stack. This prevents stack overflows on pathological dependency graphs (e.g. 10k deep chains) while returning byte-identical results.
- Internal: extracted the AST function-boundary walker shared by the
  `long-function` and `complex-function` audits into `function_spans`, so both
  agree on what a "function" is across the supported languages. `long-function`
  behavior is unchanged (parity tests hold).
- Internal: added a golden before/after harness for the `review` change signals
  (`tests/fixtures/review/<family>/<scenario>/`). Each fixture commits a
  `before/` tree, overlays an `after/` diff, runs the real
  `repopilot review --format json`, and matches the emitted tiered signals
  against an `expected.json` of `expect`/`forbid` constraints. Seeded with one
  proven-firing fixture per family (taint, boundary, behavioral, algorithmic);
  the matrix grows by dropping in directories.
- Internal: added true-positive/false-positive fixtures for the five
  convention-shaped heuristic rules that were previously unfixtured
  (`architecture.too-many-modules`, `architecture.deep-directory-nesting`,
  `architecture.deep-relative-imports`, `architecture.barrel-file-risk`,
  `testing.source-without-test`), pinning their behaviour under the rule
  quality gate. The fixture harness now scans with `detect_missing_tests`
  enabled (matching a real default scan) so the testing audits are exercisable.
- Internal: the rule fixture coverage test now evaluates the whole registry in
  one aggregate gate (`evaluate_rule_fixtures(None, …)`) instead of only the
  rules listed in manual category arrays, so a fixture can no longer fall
  outside the harness by being left off a list. A fixture directory whose name
  matches no registered rule is now a hard error rather than being silently
  skipped, and the test reports which preview/experimental rules still lack
  fixtures.
- Internal: added true-positive/false-positive fixtures for seven previously
  unfixtured web framework rules (`framework.js.var-declaration`,
  `framework.js.console-log`, `framework.react.class-component`,
  `framework.react.prop-types`, `framework.react-native.inline-style`,
  `framework.react-native.flatlist-missing-key`,
  `framework.rn-new-arch-incompatible-dep`). The false-positive fixtures pin the
  known safe shapes — `var`/`console.log` in identifiers or comments,
  `prop-types` absent from a TypeScript project, a `StyleSheet` instead of an
  inline style, a `FlatList` with a `keyExtractor`, and a New-Architecture
  compatible dependency — so the text-/AST-lite matchers stay honest.
- Internal: added true-positive/false-positive fixtures for six previously
  unfixtured code-quality, comment-marker, and Django rules
  (`code-quality.complex-file`, `code-quality.deep-control-flow`,
  `code-marker.todo`, `code-marker.fixme`, `code-marker.hack`,
  `framework.django.raw-sql-query`). The false-positive fixtures pin the known
  safe shapes — a flat low-density file, control flow sitting exactly at the
  nesting limit, a marker keyword appearing only in a string literal or
  identifier rather than a comment, and a parameterized `cursor.execute(sql,
  params)` query — leaving only `architecture.large-file`,
  `architecture.layer-violation`, and `testing.missing-test-folder` uncovered.
- Internal: added true-positive/false-positive fixtures for `architecture.large-file`
  (the false positive sits exactly on the 300-line threshold to pin the
  off-by-one boundary) and `testing.missing-test-folder` (the false positive is a
  project with a `tests/` directory). With these, every Preview/Experimental rule
  now has true-/false-positive coverage: the fixture coverage report drops to
  zero gaps. `architecture.layer-violation` is purely config-driven — it emits
  nothing without `[[architecture.layers]]`, which the default-config fixture
  engine cannot supply — so it stays covered by `tests/architecture_opt_in_rules.rs`
  and is allow-listed in the coverage report rather than duplicated.
- Internal: the `review` golden harness now models **deletions and renames**.
  `expected.json` takes an optional `delete` array of repo-relative paths removed
  from the working tree after the overlay (a rename is `delete` of the old path
  plus the new path in `after/`); `after/` is now optional for pure-deletion
  scenarios. Backwards-compatible — fixtures without `delete` behave as before.
  Five scenarios were seeded: deleting a `src/auth/**` file still flags the
  security boundary, deleting a `*.test.ts` flags removed coverage, moving an auth
  file flags both old and new paths, a local-variable rename stays silent, and a
  >5-file/>200-line data diff trips only the volume/noise note. (A lockfile-only
  change is *not* noise — lockfiles classify as a supply-chain boundary in the
  definitely-sensitive tier — so the planned `noise/lockfile-only` fixture was
  dropped in favour of the real large-diff case.)
- Internal: added a golden snapshot for the AI context brief
  (`tests/ai_context_golden.rs`). It scans one committed sample project
  (`tests/fixtures/projects/ai-context-sample`), applies the default visibility
  profile, and snapshots `output::ai_context::render` for the default and
  `--focus security` variants, so a change to the agent-facing brief is a
  reviewed diff. Determinism is via normalization (scan wall-clock zeroed,
  context-graph cache line and approximate token count pinned, crate version
  templated); bless with `REPOPILOT_UPDATE_GOLDEN=1`. A companion MCP unit test
  asserts `repopilot_context` returns its declared `{ markdown }` output shape.
- Internal: added `scripts/check-scan-performance.js`, a release-checklist smoke
  test mirroring `check-review-performance.js`. It builds the same synthetic
  480-file corpus as `benches/scan_bench.rs` and asserts the warm scan
  wall-clock median stays under a generous budget (default 3000 ms, overridable)
  — a coarse guard against gross regressions, not a CI criterion gate.
- Internal: documented the test-fixture families and how to add/update them in
  `tests/fixtures/README.md`.

### Added

- Added `code-quality.complex-function` (preview): a per-function cognitive-complexity rule that weights **nesting depth** rather than counting
  branches flatly. A deeply nested handler is flagged while a wide-but-flat
  `switch`/`match` dispatcher is not — the case where the file-level
  `code-quality.complex-file` over-reports. Nested closures are scored
  independently rather than folded into their enclosing function. Hidden in the
  default profile (a strict-mode maintainability suggestion); threshold is
  configurable via `[code_quality] complex_function_threshold` (default 15, the
  conventional cognitive-complexity limit). `complex-file` is unchanged. Ships
  with true-positive/false-positive fixtures.
- The dependency graph now models workspace packages as first-class `Package` nodes. The builder detects npm/yarn, pnpm, Cargo, and Go (`go.work`) workspaces and records which package each file belongs to (by longest path
  prefix), so future rules can reason about real package boundaries instead of
  path globs. A non-workspace repository produces no package nodes and leaves
  the graph unchanged. Internal only for now — no command consumes it yet.
- Added a published [rules reference](docs/rules-reference.md) generated
  straight from the rule registry: every rule's id, category, default
  severity/confidence, lifecycle, signal source, recommendation, and known
  false positives. A drift test (`tests/rules_reference_doc.rs`) re-renders it
  in memory and fails if the committed file falls behind the code, so the
  catalog cannot drift from what actually ships. Regenerate with
  `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`.

## [0.17.0] - 2026-06-11

### Added

- Added four architecture rules that query the import graph.
  `architecture.dead-module` (production files nothing imports that are not entrypoints or public API) and `architecture.test-leak` (production code importing a test or fixture file) are on by default.
  `architecture.layer-violation` and `architecture.package-boundary-violation`
  are **strictly opt-in** and emit nothing until you declare structure: ordered
  `[[architecture.layers]]` (a module may import layers at or below its own, never above) and `[architecture] package_roots` (e.g. `packages/*`, flagging
  imports that reach into another package's internals instead of its public
  API). Each rule ships with true-positive and false-positive fixtures; all
  four are `experimental`.
- Added per-rule configuration: `[rules] disable` drops a rule's findings and
  `[rules.severity_overrides]` sets an absolute severity per rule. Unknown rule ids and invalid severities are surfaced as report diagnostics, not applied.
- Added regression coverage and a release self-review quality gate for
  standard-library/local dependency imports, AST-confirmed recursion,
  prose-only removed-behavior text, and side effects in tests/fixtures.
- Added stdlib Python tests for release-note extraction, version validation, Cargo package allowlisting, pinned Actions, npm orchestration, and removed
  editor packaging.
- Added a real schema `0.18` report fixture and documented report schema
  versioning independently from package versions.
- Review now prints a one-line taint disclaimer when taint-lite signals are present (console and Markdown): they trace input → sink *reachability*, a path
  that exists, not a confirmed vulnerability.

### Changed

- Graph rules built on *absence* claims now reflect import-resolution quality
  in their confidence. The coupling graph build records unresolved relative
  (`./`, `../`) imports; when any exist, `architecture.dead-module` and
  `architecture.high-instability-hub` are reported at `Medium` confidence
  instead of `High`, with the evidence snippet stating how many unresolved
  imports make the graph incomplete (fan-in is then a lower bound).
  `architecture.dead-module` is suppressed entirely when an unresolved import's
  final segment matches the candidate file's name — that import is a plausible
  importer. Edge-existence proofs (`architecture.circular-dependency`,
  `architecture.excessive-fan-out`, test-leak/layer/package rules) are
  unaffected: unresolved imports cannot invalidate a resolved edge. Both
  demoted rules are declared `contextual_confidence` in the registry, and their
  false-positive notes document the demotion.

- `architecture.circular-dependency` findings now lead with the **minimal cycle**
  within a strongly-connected component (e.g. `a -> b -> c -> a`) and carry the
  component size as context, instead of repeating the entire component into every
  evidence snippet. Evidence now points at the files in the shortest cycle only,
  so a large tangled component yields one actionable loop rather than a wall.
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

[Unreleased]: https://github.com/MykytaStel/repopilot/compare/v0.19.0...HEAD
[0.19.0]: https://github.com/MykytaStel/repopilot/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/MykytaStel/repopilot/compare/v0.17.0...v0.18.0
[0.17.0]: https://github.com/MykytaStel/repopilot/compare/v0.16.0...v0.17.0
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
