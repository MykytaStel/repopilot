# Graph and scan hardening notes

RepoPilot's Context Risk Graph is a repository-context optimization, not a
replacement for source scanning.

## Rules

- `RepoContextGraph` may cache file context, imports, framework metadata, graph
  edges, and derived graph summaries.
- It must not be treated as a full `ScanFacts` or source-content cache.
- Changed scans must still analyze changed file contents before scoring.
- Renderers must never trigger another scan.
- Commands should build one `ScanSummary` and pass it through filters,
  visibility, report schema, and renderers.
- Graph output must stay bounded for large repositories.

## Large repository limits

Graph output should stay capped:

```text
MAX_CONTEXT_GRAPH_CYCLES
MAX_CONTEXT_GRAPH_METRICS
MAX_CONTEXT_GRAPH_RISKY_CLUSTERS
MAX_CONTEXT_GRAPH_BLAST_RADIUS
```

If a section is truncated, the report should say which section was truncated
instead of silently hiding the limit.

## Cache invalidation

Context graph cache must be invalidated when any of these change:

- graph schema version;
- resolver version;
- RepoPilot version;
- scan configuration fingerprint;
- relevant graph inputs: files, imports, roles, frameworks, runtimes,
  paradigms, detected frameworks, React Native profile.

If a future optimization uses file hashes or mtime snapshots, it must remain
local-first and deterministic.
