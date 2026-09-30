# Dependency Graph: Current Implementation

RepoPilot builds an internal `GraphSnapshot` from scan facts and the shared
language-aware import resolvers. The snapshot provides one dependency view for
architecture audits, review impact, and repository context.

## Current model

A snapshot can contain file, directory, package, workspace, and external
dependency nodes. The current scan builder creates file nodes, external
dependency nodes for imports outside the scanned files, and package nodes for
detected workspaces. It emits resolved `Imports` edges, external `DependsOn`
edges, and heuristic `TestOf` edges. Unresolved relative imports add bounded
diagnostics and low-confidence edges.

The graph stores edge kind, provenance, and confidence. Node and edge ordering
is deterministic, and duplicate relationships are collapsed before consumers
use the snapshot. Graph internals remain crate-internal; public report schemas
expose bounded summaries and findings instead of raw snapshots.

## Consumers

- `architecture.circular-dependency` uses strongly connected components and
  excludes deferred imports that do not create a module-load cycle.
- `architecture.excessive-fan-out` and
  `architecture.high-instability-hub` use shared degree metrics.
- Review uses direct dependents and reverse reachability to describe the impact
  of changed files.
- Repository context and `repopilot ai context` use the same graph metrics for
  dependency summaries and hot-file selection.
- Workspace package boundaries use manifest-derived package membership.

The [engine pipeline](engine-pipeline.md) defines when the graph is built and
how its outputs are reused.

## Limits

Import resolution is bounded and differs by language. A resolved edge points to
a scanned file. An unresolved relative import is reported as a diagnostic; a
bare package import is represented as an external dependency. Dynamic loading,
full compiler semantics, call graphs, and every workspace or alias convention
are outside this graph's guarantees. Capability metadata and diagnostics keep
unsupported or incomplete relationships visible to internal consumers.

See [language support](../language-support.md) for user-visible coverage and
[architecture audit policy](../architecture-antipatterns.md) for the rules that
consume graph evidence.
