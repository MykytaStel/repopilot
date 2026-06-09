# Dependency Graph v2

## Purpose

Dependency Graph v2 should become RepoPilot's shared dependency graph layer for
architecture rules, blast-radius analysis, AI context, and future rule
capabilities.

Graph v2 core types now exist internally as an isolated module. They define
nodes, edges, snapshots, and diagnostics without changing existing graph
consumers.

Graph v2 now also has an initial internal `GraphSnapshot` builder from scan
facts. It creates file nodes, external dependency nodes, and import edges by
reusing RepoPilot's shared language-aware import resolvers (Rust
`crate::`/`super::`/`self::`/`mod`, TypeScript/JavaScript relative and `tsconfig`
aliases, Python, Go, JVM). Imports that do not resolve to a scanned file become
external dependency nodes, and unresolved relative imports record a bounded
diagnostic. Edges carry typed kinds (`Imports` for resolved local files,
`DependsOn` for external dependencies, `TestOf` for co-located test files),
provenance, and a confidence tier (high for resolved files, medium for external
packages, low for unresolved relative imports). It is not yet used by scan
findings, review blast radius, AI context, or public report schemas.

Graph v2 now has internal algorithms for degree summaries, hubs, SCC-based cycle
detection, local neighborhoods, transitive blast radius (reverse dependents of a
changed set), one-hop direct dependents (the importer set per file),
directory-level dependency aggregation (cross-directory coupling), compact
deterministic graph summaries, and bounded capability metadata
(`graph_capabilities`) describing which dependency facts a snapshot carries.

All three import-coupling scan rules now run on graph v2. The
`architecture.circular-dependency` rule detects cycles through the v2
`GraphSnapshot` and SCC `find_cycles`; `architecture.excessive-fan-out` and
`architecture.high-instability-hub` derive their per-file fan-in, fan-out, and
instability from v2 degree counting. Review and AI context consume graph v2 too:
review's blast radius (the per-file importer set) is sourced from the snapshot's
one-hop `direct_dependents`, and the AI context hot-files fallback uses the same
metrics. Each migration preserves its existing rule IDs, severities, evidence,
and output shapes.

### Migration history

PR #195 migrated circular dependency detection to graph v2. PR #196 extracted the
coupling graph → `GraphSnapshot` conversion into shared graph infrastructure
(`build_coupling_graph_snapshot`): an adapter free of audit concepts (no
severities, rule IDs, findings, recommendations, or evidence) that produces file
nodes and `Imports` edges with a node-id → path map.

PR #197 completes the near-term migration. One v2-backed metric path,
`coupling_file_metrics` (degree counting over the shared snapshot, with
instability owned by `NodeDegree::instability`), replaces v1 `compute_metrics` in
both the fan-out/instability-hub rules and the AI context hot-files fallback.
Review's importer inversion (`build_importers_by_target`, feeding blast radius and
boundary/tiered signals) is sourced from the snapshot via the new one-hop
`direct_dependents` algorithm. `graph_capabilities` adds bounded metadata
describing the dependency facts a snapshot carries — internal foundation for the
roadmap's rule-capability checks, with no command or report consumer yet. The v1
`compute_metrics`/`RepoContextGraph` paths remain for consumers not yet migrated;
all migrated outputs are byte-for-byte unchanged and guarded by parity tests.

The largest remaining migration is the context graph subsystem
(`RepoContextGraph` / `context_graph_summary`), which still backs the *primary* AI
context graph projection (edit order, blast radius, high-context files). It should
move to graph v2 only once v2 has equivalent deterministic fixtures and bounded
behavior, as below.

Today, `CouplingGraph`, `RepoContextGraph`, import extraction, language
resolvers, review signals, and graph summaries provide useful behavior. Graph
v2 should give those paths one deterministic model built from repository facts,
without forcing product commands to rescan or independently reinterpret the
same imports.

## Non-Goals

Graph v2 should not:

- add new public CLI commands;
- reintroduce `repopilot inspect graph`;
- expose unstable graph internals as a public report schema;
- replace language-specific tools such as the TypeScript compiler, Cargo, or
  package managers;
- attempt to become a complete semantic call graph in its first version.

The graph should represent evidence RepoPilot can resolve locally and
deterministically. It should preserve uncertainty through diagnostics and
capability metadata rather than claim semantic completeness.

## Current Problems

The current import and graph implementation is not a sufficient long-term
platform:

- graph construction, context enrichment, caching, and consumers are
  distributed across scan, graph, audit, review, risk, and output modules;
- import resolution depth and guarantees differ by language;
- architecture rules need shared graph facts instead of rebuilding
  relationships independently;
- review blast radius and AI context need the same understanding of dependency
  direction and resolved boundaries;
- future rule capabilities need to declare whether required graph facts are
  available for a repository, language, or scan scope.

Existing graph behavior should remain operational while graph v2 is introduced
incrementally. The migration should avoid parallel rescans and preserve bounded
output, cache invalidation, and changed-scan honesty.

## Target Pipeline

```text
RepoFacts
  -> import facts
  -> GraphSnapshot
  -> graph algorithms
  -> rule inputs / review signals / AI context
```

`GraphSnapshot` should be built once for a command path from facts and resolver
results. Algorithms and product consumers should operate on that snapshot
rather than reconstructing edges.

## Core Concepts

The planned concepts are:

- `GraphNode`: a stable identity and kind for an entity in the repository or
  its dependency boundary;
- `GraphEdge`: a directed, typed relationship with provenance, confidence, and
  optional evidence;
- `GraphSnapshot`: the immutable nodes, edges, resolver metadata, diagnostics,
  and capability state for one analysis scope;
- `GraphNodeKind`: the category of an entity represented by a node;
- `GraphEdgeKind`: the meaning of a directed relationship;
- `GraphResolver`: a language-aware component that converts import or
  dependency facts into resolved graph relationships;
- `GraphDiagnostic`: a bounded explanation of unresolved, ambiguous, skipped,
  or unsupported graph evidence.

Initial node kinds:

```text
File
Directory
Package
Workspace
ExternalDependency
```

Initial edge kinds:

```text
Imports
ReExports
DependsOn
TestOf
GeneratedFrom
Configures
```

Not every kind needs to ship in the first implementation. The core model should
allow these relationships without requiring consumers to infer their meaning
from paths or untyped strings.

Node and edge identities must be deterministic. Paths should be normalized
relative to the repository root where possible, and external dependencies
should use stable ecosystem-aware identities rather than local installation
paths.

## Resolver Strategy

Resolver support should be explicit and capability-based. Unresolved imports
should produce bounded diagnostics or unresolved external nodes, not invented
edges.

### JavaScript And TypeScript

Initial scope:

- relative imports;
- extensionless imports;
- index files;
- `tsconfig` path aliases.

Later scope:

- package and workspace imports;
- package export maps where deterministic local metadata is available.

The resolver should not claim parity with the TypeScript compiler for every
module mode, plugin, or generated configuration.

### Rust

Initial scope:

- `mod`;
- `pub mod`;
- `use crate::`;
- `use super::`.

Later scope:

- workspace crates;
- package dependency relationships derived from Cargo metadata or manifests.

The first implementation should focus on source-module relationships and avoid
pretending to replace Cargo's complete dependency and feature resolution.

### Python

Initial scope:

- relative imports;
- package roots;
- `__init__.py`.

Later scope:

- FastAPI and Django project roots;
- environment-aware package resolution where inputs remain local and
  deterministic.

Python path mutation, dynamic imports, and environment-specific import hooks
should remain explicit limitations.

Other languages can continue using current import extraction until their
resolver contract and support tier are defined. Graph v2 should not imply equal
resolution depth across all detected languages.

## Algorithm Roadmap

Graph algorithms should consume `GraphSnapshot` through shared, deterministic
implementations:

```text
cycles
fan-in / fan-out
hubs
shared dependency detection
blast radius
layer violation support
workspace/package boundaries
```

Algorithms must have explicit direction semantics, bounded output, stable
ordering, and tests for disconnected nodes, unresolved edges, cycles, and
cross-boundary relationships. Expensive derived results should be computed once
per command path or cached against versioned graph inputs.

## Product Integration

Graph v2 should feed existing durable workflows:

- scan findings should use shared graph facts for coupling, cycles, layer
  boundaries, and related architecture evidence;
- review should use the same graph direction and boundary model for blast
  radius;
- AI context should derive hot files, dependency context, and edit order from
  graph v2 summaries;
- future architecture policies should consume typed nodes and edges rather than
  path heuristics alone;
- future rule capabilities should declare the graph facts, resolver support,
  and analysis scope they require.

Integration should be incremental. Existing `CouplingGraph` and
`RepoContextGraph` consumers should migrate only after graph v2 has equivalent
deterministic fixtures and bounded behavior.

## Public Surface Rule

Graph v2 diagnostics should not create a new public `inspect graph` command.

Use:

- scan JSON;
- review output;
- AI context;
- MCP tools;
- tests;
- internal development scripts.

Any future public projection should be aggregate, bounded, versioned, and tied
to an existing durable workflow. Raw or unstable graph internals should remain
internal.

## Implementation Steps

1. **Done.** Migrate the import-coupling architecture rules to graph v2 —
   `architecture.circular-dependency` (cycles), `architecture.excessive-fan-out`,
   and `architecture.high-instability-hub` (degrees) — sharing the coupling graph
   → `GraphSnapshot` conversion (`build_coupling_graph_snapshot`). Other
   graph-related rules can migrate onto the same snapshot next.
2. **Done.** Feed graph v2 into review blast radius: the importer inversion is
   sourced from the snapshot via one-hop `direct_dependents`, output unchanged.
3. **Done (fallback).** Feed graph v2 into AI context hot files: the coupling
   fallback uses `coupling_file_metrics`. The primary `context_graph_summary`
   projection still needs migration (see below).
4. **Done (foundation).** Add graph capability metadata (`graph_capabilities`).
   No rule consumes it yet; wiring rules to declare required graph facts is next.
5. Migrate the context graph subsystem (`RepoContextGraph` /
   `context_graph_summary`) to graph v2 so the primary AI context projection and
   review signals share one model. This is the largest remaining step and should
   land only behind equivalent deterministic fixtures.
