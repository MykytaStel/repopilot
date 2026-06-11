# Architecture Anti-patterns

RepoPilot architecture checks are designed to catch product structure risks without turning
rule fixtures, test corpora, docs, examples, generated files, vendor trees, or build output into
user-facing architecture noise.

## Product policy

Architecture findings should answer a maintainer question:

```text
Could this production structure make the codebase harder to review, change, test, or release?
```

They should not report intentionally unusual non-production layouts, such as:

```text
tests/fixtures/rules/**
examples/**
docs/**
src/generated/**
vendor/**
target/**
dist/**
```

Those paths can still be scanned by other rule families when appropriate. They are only excluded
from broad architecture structure heuristics.

## Current architecture anti-pattern family

| Rule | Product question | Default behavior |
| --- | --- | --- |
| `architecture.deep-nesting` | Is production code buried in an over-nested module hierarchy? | Visible only for production architecture candidates. |
| `architecture.large-file` | Is a production file too large to review and safely maintain? | Should avoid test/generated/vendor noise. |
| `architecture.too-many-modules` | Does a package/directory expose too many modules for its boundary? | Should focus on product module boundaries. |
| `architecture.import-coupling` | Is a file coupled to too many internal modules? | Should prefer source code over generated/test corpora. |
| `architecture.deep-relative-imports` | Are imports crossing too many directory levels? | Should focus on maintainability of production imports. |
| `architecture.barrel-file-risk` | Is a barrel file hiding coupling or unstable public boundaries? | Should explain why the barrel is risky, not merely that it exists. |
| `architecture.dead-module` | Is a production file imported by nothing and not an entrypoint? | Demoted/suppressed when unresolved imports make the graph incomplete. |
| `architecture.test-leak` | Does production code import a test or fixture file? | On by default; evidence cites the import line. |
| `architecture.layer-violation` | Does a module import against the declared layer order? | Strictly opt-in via `[[architecture.layers]]`. |
| `architecture.package-boundary-violation` | Does one package reach into another's internals instead of its public API? | Auto-enabled on a detected workspace (manifest boundaries → High confidence); also configurable via `[architecture] package_roots` (→ Medium). |

## Visibility guidance

Default profile should show architecture findings when they are:

- production-scoped
- evidence-backed
- actionable
- not better represented as strict-only cleanup

Strict profile can expose broader maintainability suggestions, but the wording should make the
scope explicit. For example, `fixture/test path` should not be described as normal product
architecture.

## Rule-author workflow

When adding or changing an architecture rule, include at least:

- one true-positive production fixture
- one false-positive fixture/test/docs/generated/vendor case
- one regression test proving default output does not report non-production structure as product risk
- documentation explaining why the rule is user-facing
