# Language Surface Inventory

Where language-specific behavior lives today, and which 0.21 migration PR
moves each row behind the language frontend registry (`src/languages/`).
This is the working checklist for the 0.21 cycle: a row is done when its
dispatch happens through a frontend lookup and the row is checked off. Line
anchors are correct as of the PR-1 commit; expect small drift and trust the
file, not the number.

Status legend: `[ ]` not migrated · `[x]` migrated · `[-]` stays put by
design (with reason).

## Detection and identity

- [ ] `src/knowledge/packs/core.toml` — 43 `[[languages]]` profiles: id,
      name, aliases, extensions, filenames, **declared** `support` level.
      Stays the detection data source; the registry references profile ids
      (`LanguageFrontend.knowledge_ids`) instead of duplicating extensions.
      PR-9 reconciles over-declared levels (java/kotlin/csharp/c/cpp claim
      `rule-aware` today; the guard-test ledger documents the gap).
- [x] `src/knowledge/language.rs` — `language_kind_from_id`: ids claimed by
      a frontend now take their kind from the registry; the residual match
      covers only languages without a frontend and shrinks as frontends are
      added. Pinned by `frontend_claimed_ids_keep_their_kinds`. (PR-2)
- [-] `src/audits/context/model/kinds.rs:2` — `LanguageKind` enum (43
      variants). Stays; the registry routes every variant
      (`frontend_for_kind` is exhaustive).

## Parse layer

- [-] `src/analysis/parse.rs:31` — `ParseLanguage` enum (9 grammars). Stays
      as the grammar identity; frontends bind labels to it.
- [x] `src/analysis/parse.rs` — `ParseLanguage::from_label` delegates to
      `languages::grammar_for_label`; the registry bindings are the only
      label→grammar table, pinned by `grammar_label_vocabulary_is_pinned`.
      (PR-2)
- [-] `src/analysis/parse.rs:62` — thread-local parser table. Stays, keyed
      by `ParseLanguage`; only the label dispatch moved.
- [-] `src/scan/parsed_cache.rs`, `src/analysis/artifact.rs` — parse-once
      caches treat the label as an opaque cache key and route parsing via
      `from_label`; no dispatch of their own (verified in PR-2).

## Imports and graph

- [x] `src/graph/imports.rs` — `extract_imports_from` and
      `extract_deferred_imports_from` dispatch via
      `languages::imports_for_label`; the label matches are gone. Coverage
      pinned by `import_extractor_coverage_is_pinned`. (PR-3)
- [x] `src/graph/imports/{go,jvm,python,rust,ts}.rs` — extractors moved to
      `src/languages/{go,java,kotlin,python,rust,javascript}/imports.rs`
      behind `ImportExtractor` (eager/deferred/spans); `jvm.rs` split into
      java/kotlin; `common.rs` became `languages/import_support.rs`. Python
      deferred and TS type-only semantics unchanged (zoo-frozen on six
      repos). (PR-3)
- [x] `src/graph/imports/lines.rs` — `import_line_spans` uses the
      frontend's `spans` extractor; per-language match removed. The module
      stays as the public edge-evidence API. (PR-3)
- [-] `src/graph/resolver/`, `src/graph/v2/` — resolution and graph
      algorithms consume extracted imports; no language dispatch inside.
      Stays language-agnostic by contract (guard: no-dispatch lint, PR-7).

## Review signals

- [x] Taint-lite is table-driven: per-language source idioms, coercion
      lists, grammar shapes, and sink classifiers live in
      `src/languages/{javascript,python,go}/review.rs` as
      `taint::tables::TaintTables`; the engines (`flow`, `sources`,
      `sanitizers`) are language-neutral and `TaintLang` is gone — gating is
      the frontend's `taint` field, pinned by
      `taint_participation_is_pinned`. Shared callee helpers stay in
      `taint/sinks.rs` (model) and `taint/ast.rs`. Equivalence evidence: the
      taint test suite unchanged and green, plus the wagtail agent demo
      firing identical signals. (PR-4a)
- [-] `src/review/signals/behavioral/keywords.rs` — auth keyword vocabulary
      (strong/exact tokens, mutation verbs). Language-neutral English
      identifiers; stays shared by design. Per-frontend *additions* (e.g.
      Spring `@PreAuthorize`) arrive with the Java promotion (PR-8).
- [x] `src/review/signals/behavioral/removed_ast.rs` — the walks consult the
      frontend's `RemovedTables` (test-case / error-handling recognizers,
      auth call kinds), dispatched by extension exactly as before; a file no
      frontend claims counts zero everywhere, matching the old fall-through.
      Extension vocabulary pinned (uniqueness + the knowingly-dead `cts`) by
      `removed_recognizer_extensions_are_pinned`. (PR-4c)
- [-] `src/review/signals/behavioral/removed.rs` —
      `supports_coarse_removed_detection(ext)`: the coarse text fallback
      covers languages that deliberately have *no* frontend (shell, C/C++,
      PHP, Ruby, Swift). It is fallback policy, not a frontend capability;
      revisit only when those languages gain frontends.
- [x] `src/review/signals/classify.rs` — `match_node_for_boundary` takes its
      node kinds from the frontend's `BoundaryKinds` table; keyword
      vocabularies stay shared in the engine. C#'s boundary is deliberately
      unwired (the old dispatch matched the label "CSharp" while detection
      emits "C#" — a dead arm; enabling it is a behavior change queued for
      the honesty pass). Pinned by `review_table_coverage_is_pinned`. (PR-4b)
- [x] `src/review/signals/algorithmic/lang.rs` — matchers consume the
      frontend's `AlgorithmicKinds` table (function/loop/call/control-flow/if
      kinds); label matches are gone from the engine. (PR-4b)

## Runtime risk and code-quality spans

- [ ] `src/audits/code_quality/language_risk.rs:68` —
      `is_ast_runtime_language(language_id)` allowlist +
      `language_risk/{js,go,python,managed}.rs` pattern modules. → PR-5
      (`frontend().risk`).
- [ ] `src/audits/code_quality/rust_panic_risk/` — Rust-only audit wired as
      its own rule family; pattern data joins the Rust frontend, severity
      calibration stays in the Knowledge Engine. → PR-5.
- [ ] `src/audits/code_quality/function_spans.rs:42` — function-node kinds
      per label (long/complex function). → PR-5.
- [ ] `src/audits/code_quality/long_function/brace.rs`,
      `complex_function/score.rs` — brace-family vs Python span logic via
      `LanguageFamily`. `LanguageFamily` (syntax shape) stays a shared
      concept; per-language node kinds move. → PR-5.

## Conventions and context

- [ ] `src/scan/path_classification.rs:34` — test-like filename conventions
      (`test_*`, `*_test`, `tests.rs`, Rust-specific carve-outs from the
      0.18 low-signal fix). → PR-6 (`frontend().conventions`).
- [ ] Test-support allowlist (`testutil.rs`, `test_utils.rs`, …) and
      `FileRole::TestSupport` wiring from the 0.19 cycle — role logic stays
      in the context classifier; the *name lists* become convention data. →
      PR-6.
- [-] `src/audits/context/classify/` — role/paradigm/runtime classification
      consumes shared context signals; stays the owner of cross-language
      context. Frontend conventions feed it; it does not move. (Declared
      `context-aware` pack levels are justified here, which is why the PR-1
      honesty ledger only covers `rule-aware` claims.)
- [ ] `src/audits/framework/` + manifest readers (`package.json`,
      `pyproject.toml`, `go.mod`, …) — framework probes per ecosystem. →
      PR-6 (probe registration), detection logic unchanged.

## Output and docs

- [ ] `docs/language-support.md` — hand-maintained tier table, already
      drifted from the pack (pack says java/kotlin/csharp/c/cpp are
      rule-aware; doc says Tier 2; the wiring says context at best). → PR-7
      generates it from the registry with a BLESS drift test.
- [-] `src/output/ai_context/repo_facts.rs`, report renderers — print
      labels/tiers; consume registry values after PR-7, no language logic of
      their own.

## Out of scope for 0.21

- `src/facts/`, `src/scan/facts.rs` — `FileFacts.language` stays a label
  string this cycle; swapping to a typed id is a 0.22+ decision after the
  consumers are registry-driven.
- MCP/SARIF surfaces — no language dispatch; they inherit whatever the
  engine emits.
