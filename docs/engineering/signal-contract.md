# Signal Contract

RepoPilot findings and review signals are local, deterministic evidence. They
identify structural facts worth reviewing; they do not prove that a change is
safe or unsafe.

## Lifecycle And Source

Registered rules use `experimental -> preview -> stable -> deprecated`.

| Lifecycle | Contract |
|---|---|
| `experimental` | Useful but broad or noisy; strict-mode only. |
| `preview` | Supported and fixture-backed, but still being calibrated. |
| `stable` | Deterministic IDs, evidence, recommendations, false-positive coverage, and a clean quality gate. |
| `deprecated` | Temporarily retained while consumers migrate. |

Signal sources are explicit: `text-heuristic`, `ast`, `config-file`,
`dependency-manifest`, `import-graph`, `framework-detector`, `git-diff`, or
`mixed`. A text heuristic must not claim AST provenance.

The rule catalog and its quality gate are validated by `cargo test`
(`tests/rule_eval_fixture_coverage.rs`), which runs every rule against its bundled
true-positive/false-positive fixtures.

## File-role Evidence

Role classification is explainable data, not an opaque boolean. The detailed
classifier records exactly one evidence entry for every assigned file role,
including the source (`path`, `content`, `framework`, `manifest`, `signal`,
`mixed`, or `fallback`) and a stable human-readable reason.

The compatibility `classify_file` API still returns semantic context.
Scanner/cache/explain surfaces use `classify_file_with_evidence` when role
provenance must survive beyond classification. Role evidence does not alter a
rule by itself; knowledge-pack decisions remain responsible for deciding how a
role affects severity or suppression.

## Finding Contract

Every rendered finding must have:

- non-empty stable `id`, `rule_id`, title, description, and recommendation;
- concrete evidence with a path and valid line range;
- provenance with lifecycle, detector, source, and analysis scope;
- explainable `risk-v3` signals;
- documentation for high/critical findings.

The pipeline validates this contract after enrichment and risk scoring:

```text
raw findings -> enrich once -> risk scoring -> contract validation -> report
```

Contract diagnostics remain visible even when the default profile hides a
strict-only suggestion.

## Quality Gate

A stable or default-visible rule requires true-positive and false-positive
fixtures, deterministic finding IDs, concrete evidence, false-positive notes,
and a clean local evaluation.

These aggregate fields must be zero:

- `missing_findings`
- `unexpected_findings`
- `contract_violations`
- `stable_id_failures`
- `quality_gate_failures`

`scripts/smoke-product.sh` and `scripts/verify-release.sh` enforce the gate.

## Visibility

The default profile prioritizes actionable security, runtime, import-graph, and
review risks. Broad maintainability, testing, marker, and text-heuristic signals
remain available through `--profile strict`.

Hidden suggestion summaries keep default output transparent: RepoPilot reports
that additional findings exist without allowing weak heuristics to dominate
normal review or CI output.

## Signal Quality

Reports include raw and visible quality summaries with confidence, lifecycle,
source, evidence coverage, recommendation coverage, documentation coverage, and
contract violations.

RepoPilot's self-scan release gate rejects contract violations and
default-visible noisy findings:

```bash
python3 scripts/check-signal-quality.py \
  --scan-json /tmp/repopilot-self-scan.json
```

No lifecycle, quality, or visibility feature enables telemetry, source upload,
hosted scanning, implicit LLM calls, or arbitrary plugin execution.
