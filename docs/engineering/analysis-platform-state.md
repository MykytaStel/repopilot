# Analysis Platform State

RepoPilot is converging on a smaller public CLI backed by a deeper, shared
analysis platform. New analysis capabilities should strengthen existing user
workflows rather than create additional public diagnostic commands.

## Current Product Surface

The active public commands are:

```text
scan
review
baseline
cache
snapshot
ai context
init
mcp
```

The following legacy or diagnostic commands have been removed from the public
surface:

```text
ai plan
ai prompt
compare
doctor
inspect
```

RepoPilot should avoid adding public diagnostic commands unless a diagnostic
becomes a durable user workflow with a stable product contract. Internal
analysis data should normally be exposed through existing reports, AI context,
MCP tools, tests, or development scripts.

## Current Analysis Stack

The current engine already has layers that move repository contents toward
reviewable findings:

```text
filesystem scan
-> file facts / scan facts
-> tree-sitter parsing
-> import extraction
-> framework detection
-> audits / rules
-> findings
-> risk scoring
-> baseline / feedback / suppressions
-> review signals
-> AI context handoff
-> MCP tools
```

These layers are useful but do not yet form one complete, first-class fact and
dependency model. Some capabilities remain distributed across scan, graph,
rule, review, and reporting code.

## What Already Exists

RepoPilot already provides:

- `ScanFacts` and `FileFacts` as structured scan inputs;
- shared parse-once tree-sitter parsing;
- import extraction and graph-related logic;
- rule metadata and a rule lifecycle;
- a finding contract with evidence, confidence, severity, provenance, and risk;
- explainable risk scoring;
- local feedback and suppressions;
- review signals for boundary, behavioral, algorithmic, taint-lite, and blast
  radius concerns;
- an AI context handoff through `repopilot ai context`;
- MCP access to local analysis tools.

These capabilities are the foundation for the next architecture. The goal is to
compose and deepen them, not replace working product paths with parallel
commands.

The first internal implementation step now exists in `src/facts`: a facts-core
skeleton with repository, file, source, confidence, and diagnostic types. It is
not yet connected to scan, review, report output, AI context, or MCP.

## What Is Missing

RepoPilot still needs a deeper fact-based platform with:

- a populated `RepoFacts` model assembled from existing file and scan facts;
- a first-class dependency graph with explicit core types and contracts;
- symbol facts for definitions, references, ownership, and relationships;
- rule capabilities that declare which facts and analysis levels a rule needs;
- language support tiers that make analysis depth and guarantees explicit;
- runtime evidence ingestion that can enrich static facts without making
  analysis nondeterministic;
- a suppression decision model that records why findings are retained, hidden,
  or accepted;
- evaluation fixtures that measure fact, graph, rule, and finding quality.

## Target Architecture

The target pipeline is:

```text
files
  -> facts
  -> graph
  -> rules
  -> findings
  -> risk
  -> suppressions / decisions
  -> reports / AI context / MCP
```

Facts and graph data should be computed through shared engine paths. Rules
should declare their requirements, findings should retain evidence and
provenance, and all product outputs should consume the same decisions rather
than reimplementing analysis.

## Rule For Future Diagnostics

Do not add public diagnostic commands by default.

Prefer:

- scan JSON;
- review output;
- AI context sections;
- MCP tools;
- tests;
- internal development scripts.

Avoid public commands such as:

```text
repopilot inspect facts
repopilot inspect graph
repopilot inspect rules
```

`inspect` was intentionally removed from the public CLI surface. Reintroducing
it for internal architecture visibility would expand the product contract
without creating a durable user workflow.

## Next Planned Implementation Steps

1. Add a `ScanFacts -> RepoFacts` bridge.
2. Expose facts through scan JSON and AI context, not `inspect`.
3. Design graph v2.
4. Add graph v2 core types.
5. Feed graph v2 into scan, review, and AI context.
6. Add rule capabilities metadata.
7. Add language support tiers.
8. Add runtime evidence ingestion.
9. Add evaluation fixtures.
