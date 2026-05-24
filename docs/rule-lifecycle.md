# Rule Lifecycle

Every registered rule has lifecycle and signal-source metadata.
Rule metadata lookup is indexed at runtime, so repeated finding enrichment uses
a prebuilt rule-id map instead of scanning the full registry for every finding.

Lifecycle values:

| Lifecycle | Meaning |
|---|---|
| `experimental` | Broad or noisy heuristic. Useful signal, but not a stable product contract. |
| `preview` | Supported rule that is still being calibrated through fixtures and feedback. |
| `stable` | Fixture-backed rule with clear evidence, recommendation, false-positive notes, and clean local evaluation. |
| `deprecated` | Rule kept temporarily for compatibility while a replacement exists or behavior is removed. |

Signal source values:

| Source | Meaning |
|---|---|
| `text-heuristic` | Text or token heuristic. Usually preview or experimental. |
| `ast` | Tree-sitter or parser-backed code structure signal. |
| `config-file` | Project configuration file signal. |
| `dependency-manifest` | Package/dependency manifest signal. |
| `import-graph` | Repository graph or dependency edge signal. |
| `framework-detector` | Framework/profile detector signal. |
| `git-diff` | Changed-code review signal. |
| `mixed` | Rule combines multiple sources. |

Inspect the catalog locally:

```bash
repopilot inspect rules
repopilot inspect rules --format json
repopilot inspect rules --lifecycle preview
repopilot inspect rules --source text-heuristic
repopilot inspect rule security.secret-candidate
repopilot inspect eval-rules --format json
```

`inspect eval-rules --format json` includes aggregate counts and per-rule
fixture details, including expected findings, actual findings, missing findings,
unexpected findings, contract violations, stable-id failures, fixture coverage,
and quality-gate failures.

Rule lifecycle does not enable telemetry, hosted scanning, remote model calls, or
arbitrary plugin execution. It is local metadata used for reports, provenance,
quality summaries, and rule-author discipline.
