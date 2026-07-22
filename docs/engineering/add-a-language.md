# Add a Language

Language support is added by writing tables behind the frontend contract
(`src/languages/`), not by editing engines. Every step below is enforced by
a guard test or a generated artifact, so a partial integration fails loudly
instead of shipping quietly.

Scaffold the module tree first:

```bash
python3 scripts/new-language.py <id> "<Display Label>"
```

## Checklist

1. **Knowledge profile** — add (or verify) the `[[languages]]` entry in
   `src/knowledge/packs/core.toml`: id, name, aliases, extensions,
   filenames, and an honest `support` level. Detection data lives here and
   only here; the frontend references the profile by id.
2. **Descriptor** — `src/languages/<id>/mod.rs`: a `LanguageFrontend` static
   with the id, label, `LanguageKind`, claimed knowledge ids (dialects
   included), and grammar bindings. Route the kind in
   `frontend_for_kind` and register the frontend in `ALL_FRONTENDS`
   (`src/languages/mod.rs`) — the exhaustive match will not compile until
   you decide the routing.
3. **Grammar (optional)** — add the tree-sitter crate to `Cargo.toml`, a
   `ParseLanguage` variant with a thread-local parser in
   `src/analysis/parse.rs`, and grammar bindings on the descriptor. The
   `every_grammar_is_claimed_by_exactly_one_frontend` and pinned-vocabulary
   guards force the bindings to stay coherent.
4. **Imports** — `imports.rs` implementing eager edges, line spans, and (only
   if the language has load-order-neutral imports) deferred edges. Update
   the `import_extractor_coverage_is_pinned` test deliberately.
5. **Review tables** — `review.rs`: boundary node kinds, algorithmic node
   kinds, removed-behavior recognizers, and — for web-facing languages —
   taint tables (sources, coercions, sink classifier). Update the
   corresponding pins.
6. **Runtime risk** — pattern definitions and emitters under
   `src/audits/code_quality/language_risk/`, referenced from `risk.rs`.
7. **Conventions** — test-file naming, `test_` prefix applicability,
   test-support recognizer, entrypoint content probe. Update
   `path_conventions_are_pinned`.
8. **Knowledge applicability** — extend `core.toml` rule applicability and
   calibration for the new language where rules should (or should not)
   fire.
9. **Fixtures** — a TP and FP fixture per signal family you wired, under
   `tests/fixtures/` (see the fixtures README for layout).
10. **Zoo acceptance** — add a pinned real-world repository to
    `tests/zoo/manifest.toml`, bless its snapshot, and label the
    default-visible findings in `tests/zoo/expectations/`. This is the
    evidence the release rides on.
11. **Docs** — regenerate the support matrix
    (`REPOPILOT_BLESS=1 cargo test --test language_support_doc`) and the
    rules reference if rules changed. The matrix derives capabilities from
    wiring; if a column is wrong, the wiring is wrong.

## Ground rules

- Refactors of existing languages must be behavior-frozen: zoo snapshots
  byte-identical, goldens unchanged. New capabilities change snapshots and
  must arrive with reviewed expectations.
- New signals ship at `preview` visibility with conservative,
  high-specificity patterns; the 0.19 lesson applies — downgrade noisy
  findings via the Knowledge Engine, don't delete detection.
- The registry is static data assembled at compile time. No plugin runtime,
  no dynamic loading (see the Knowledge Engine runtime boundaries).
