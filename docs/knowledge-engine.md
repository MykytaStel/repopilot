# Knowledge Engine

The RepoPilot Knowledge Engine is the local metadata layer that keeps findings
context-aware and explainable. It does not call external services and does not
learn from telemetry.

## Runtime Source

RepoPilot uses the bundled core knowledge pack as the only runtime source:

```text
src/knowledge/packs/core.toml
```

The loader has an explicit source boundary so future releases can add local
overlays without rewriting rule decisions. Custom external packs are not loaded
by `scan`, `review`, `ai`, or CI.

## Concepts

| Concept | Meaning |
|---|---|
| Built-in rule | A detector implemented in RepoPilot and identified by a stable `rule_id`. |
| Rule metadata | Title, category, default severity, docs URL, description, and recommendation in the rule registry. |
| Applicability | Knowledge Engine data that defines which languages, frameworks, runtimes, paradigms, and file roles a rule applies to. |
| Calibration | Deterministic severity, suppression, or risk adjustments based on local file/project context. |
| Local overlay | A future local file that can tune known rules in an inspectable way. Not enabled by default. |
| Local learning | User-controlled local calibration or fixtures, not model training or remote telemetry. |
| Declarative context | Infrastructure, config, markup, query, and data-description files that should not be judged like imperative application code. |
| Infrastructure context | Terraform, Dockerfile, Nix, Kubernetes, Helm, CI workflow, and infra-directory files where orchestration is expected. |

## Rule Lifecycle

Every rule should move through the same lifecycle:

```text
audit emits rule_id
-> audit registration declares emitted rule IDs
-> registry metadata explains the rule
-> Knowledge Engine applicability defines where it applies
-> risk calibration explains priority changes
-> docs describe behavior and limitations
-> tests guard metadata, applicability, rendering, and false positives
```

This lifecycle matters more than the raw number of rules. A new rule should not
ship unless it produces evidence-backed findings, has a recommendation, and can
be inspected through the existing diagnostic commands.

## Context Taxonomy

The context classifier separates language, framework, file role, paradigm, and runtime.
This matters because the same surface pattern can mean different things in different
contexts. A long declarative infrastructure file, a functional pipeline, and an
object-oriented service should not receive the same generic interpretation.

RepoPilot keeps this taxonomy explicit with declarative and infrastructure
context so future rules can narrow applicability instead of treating every file
as normal application code.

## Diagnostics

Use the inspection commands when debugging rule behavior:

```bash
repopilot inspect knowledge --section rules --format markdown
repopilot inspect explain src/main.rs --rule language.rust.panic-risk --signal rust.unwrap
```

`inspect knowledge` shows bundled catalog data. `inspect explain` shows how a
file is classified before rule decisions are applied.

## Runtime Boundaries

RepoPilot intentionally does not add:

- a stable `--knowledge-pack` flag;
- external rule execution;
- plugin runtime loading;
- hosted learning;
- telemetry;
- arbitrary report schema compatibility shims for older scan reports.

The goal is to make the engine contract clear enough that local overlays can be
designed in a later 0.x release without changing behavior silently.

## Shared Context Signals

The 0.12 context classifier keeps reusable signal logic in one place before it is
used by role, paradigm, and runtime classification. The signal model is
deterministic and local: each signal records path, language, or content evidence
with a confidence level and a short reason label. This avoids duplicated lists of
infrastructure paths, declarative languages, and functional-first languages.

This structure is intentional:

```text
path/language/content
-> shared context signals
-> roles
-> paradigms
-> runtimes
-> rule applicability and calibration
```

Future rules should prefer the shared context signals instead of adding another
local copy of the same path or language checks inside an individual audit.

## Local Feedback And Future Overlays

RepoPilot's first local calibration surface is `.repopilot/feedback.yml` for
explicit suppressions plus `repopilot inspect feedback` for validation. Future
overlays should remain inspectable local calibration, not a plugin runtime.
Local overlays should be normal files that users can review, commit, diff, or
delete. They may tune known rule severity, confidence, or suppression decisions,
but they must not execute arbitrary code, load remote packs, or change scan
behavior silently.
