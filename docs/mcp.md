# MCP Server

`repopilot mcp --root PATH` runs RepoPilot as a local Model Context Protocol
server over stdio. It supports protocol versions `2025-11-25` and `2024-11-05`.
No source, telemetry, or tool result leaves the machine.

All filesystem arguments are resolved under `--root`; paths outside that
workspace are rejected.

## Register

Generate a generic configuration with `repopilot init --mcp-client generic`, or
use this equivalent entry in any compatible client:

```json
{
  "mcpServers": {
    "repopilot": {
      "command": "repopilot",
      "args": ["mcp", "--root", "."]
    }
  }
}
```

Client-specific bootstrap examples are also available through
`repopilot init --mcp-client claude` and `--mcp-client cursor`. They are thin
adapters over the same `repopilot mcp --root .` server command.

## Tool Contract

Tools advertise `outputSchema`, return `structuredContent`, and retain text
content as a fallback for older clients. They are annotated as read-only,
non-destructive, idempotent, and closed-world.

| Tool | Purpose | Additional inputs |
|---|---|---|
| `repopilot_review_change` | Changed/full review with findings, signals, blast radius, and gate result | `base`, `head`, `config`, `baseline`, `scope`, `profile`, `fail_on_review`, `detail`, `offset`, `limit` |
| `repopilot_scan` | Repository or changed-scope JSON scan | `config`, `profile`, `scope`, `base`, `offset`, `limit` |
| `repopilot_context` | Budgeted AI-ready Markdown context | `config`, `profile`, `focus`, `budget`, `analysis_handle` |
| `repopilot_explain_file` | File-role evidence and ordered rule decision trace | `rule`, `signal` |
| `repopilot_explain_finding` | Replay a file-scoped finding by stable ID and optional occurrence locator; returns a stored-only fallback when safe replay is unavailable | `finding_id`, `source`, `analysis_handle`, `evidence_path`, `line_start` |
| `repopilot_explain_review_signal` | Explain one review signal with provenance, gate state, impact, verification, and limitations | `signal_id`, `analysis_handle` |

Every `tools/call` result includes `workspaceRevision`. Successful scan and
review results additionally include `analysisHandle`; the server retains the
eight most recent full analysis reports in memory. Pass a handle to
`repopilot_explain_finding` to select that report rather than the latest result,
or to `repopilot_context` to require that the generated context still belongs
to the same workspace revision. Unknown, expired, or stale handles fail
explicitly and report the current revision. Handles are session-local and are
not persisted to disk.

`repopilot_explain_file` returns additive JSON fields for explicit scope,
role evidence, applicability checks, every ordered override, severity
transitions, and the final default-profile visibility decision. It resolves
executable-package manifest context from the MCP root so `cli-executable`
classification matches normal scanning. Its rule base severity comes from
detector-specific signal defaults when known, then falls back to the rule
registry when a known `rule` is supplied.

`repopilot_explain_finding` consumes a stable file-scoped finding ID from
either `repopilot://last-scan` or `repopilot://last-review`. It uses the stored
`provenance.knowledge_decision` inputs to replay the rule against the current
workspace and returns the emitted finding, stored decision, full file/decision
trace, and a `matched` or `drifted` comparison. A drift result means the source
file or current bundled knowledge no longer reproduces the recorded action,
severity, or reason. Repository, workspace, Git-diff, and framework-project
findings are rejected explicitly because correct replay requires rebuilding
their wider analysis context. The finding-level severity remains authoritative
when detector-local policy differs from the Knowledge Engine output.

A finding ID is stable baseline identity, not guaranteed occurrence identity.
When the selected report contains several entries with the same ID,
`repopilot_explain_finding` returns:

```json
{
  "status": "ambiguous",
  "finding_id": "…",
  "candidates": [
    {
      "evidence_path": "src/config.rs",
      "line_start": 10
    },
    {
      "evidence_path": "src/config.rs",
      "line_start": 20
    }
  ]
}
```

Pass one candidate back as `evidence_path` plus `line_start` to select the exact
occurrence. The stable finding ID and baseline matching contract remain
unchanged.

Every tool accepts a workspace-relative `path` where applicable. Review defaults
to `scope=changed`, `profile=default`, `fail_on_review=none`, and
`detail=compact`. Compact output caps detailed findings and each signal tier at
20 total review signals; `detail=full` returns the complete JSON report. Scan
and review accept `filters.min_severity`, `filters.min_confidence`,
`filters.min_priority`, and `filters.rules`.

Scan and review accept zero-based `offset` plus `limit` (1-1000) for the
top-level `findings` array. A paginated result includes MCP-level `pagination`
metadata with `offset`, `limit`, `total`, `returned`, and nullable
`next_offset`; the JSON report itself remains schema-compatible. Review stores
the full report behind its handle even when the immediate client response uses
`detail=compact`.

Serialized tool results are bounded to 1 MiB by default. Configure the bound
with `repopilot mcp --max-response-bytes N` (`N >= 1024`). A result that still
exceeds the bound is replaced with a small in-band error carrying
`responseTruncated`, `responseLimitBytes`, `workspaceRevision`, and the analysis
handle when one was created. Use filters, pagination, compact review detail, or
a smaller context budget to stay below the limit.

For agent-assisted remediation, treat default-profile tool output as the product
view, not the full recall set. Use `profile=strict` when auditing false positives,
validating that a downgrade remains recoverable, or checking whether hidden
suggestions still exist. Changed-scope tools are optimized for edited files and
cache reuse; run a full scan when repository-wide architecture/framework findings
or aggregate counts must be authoritative.

Tool failures use MCP `isError: true`. Malformed input produces JSON-RPC error
`-32700` instead of being skipped.

## `repopilot_scan` Persistent Cache

`repopilot_scan` may reuse a local persistent cache for repeated scans of an
unchanged repository state. The cache is read-only from the MCP client's
perspective: it only stores scan reports produced by local RepoPilot analysis and
never uploads source, telemetry, or results.

Cache files are stored in Git-owned metadata, resolved with:

```bash
git rev-parse --git-path repopilot/cache/mcp-scan
```

RepoPilot disables the disk cache if that Git metadata path cannot be resolved
safely. Full scan reports are not written under the repository working tree
(`.repopilot/cache/mcp-scan` is not used), so users cannot accidentally commit
cached evidence snippets.

A cached `repopilot_scan` result is invalidated by RepoPilot version or cache
schema changes, scan arguments (`profile`, `scope`, `base`, filters), the
resolved commit for a changed-scope `base` ref, committed/staged/unstaged or
untracked changes anywhere in the Git worktree, explicit or discovered
`repopilot.toml` content, `.repopilot/feedback.yml`, `.repopilotignore`, parent
`.gitignore` and `.ignore` files, `.git/info/exclude`, and the effective global
Git ignore file. If any cache input is uncertain, RepoPilot prefers a fresh scan
over a cache hit.

The cache can be deleted safely. RepoPilot recreates it on demand. Retention is
best effort: after a successful store, RepoPilot keeps at most 32 valid scan
reports per repository cache directory, removes the oldest valid entries first,
ignores cleanup errors, and leaves unrelated files in that Git-owned cache
directory untouched.

## Resources

The server exposes:

- `repopilot://rules`: the rule catalog;
- `repopilot://repository-summary`: workspace/config/baseline/feedback and
  session-result availability without triggering a scan;
- `repopilot://analyses`: newest-first summaries for retained scan/review handles;
- `repopilot://last-scan`: the last successful scan in this session;
- `repopilot://last-review`: the last successful review in this session.

Last-result resources appear after the corresponding tool has run.
Workspace-dependent MCP tools are evaluated on every call so edits, manifest
changes, feedback, ignore files, configuration, and review state cannot be
hidden behind an arguments-only result cache. `repopilot_scan` keeps its separate
persistent cache described above; that cache validates Git/config/input
fingerprints and prefers a fresh scan whenever an input is uncertain.
Successful scan and review calls always replace `repopilot://last-scan` and
`repopilot://last-review` with the exact result returned to the client.

## Prompts

- `review-change`: prepare an agent to inspect the current change.
- `fix-top-risk`: prepare a constrained remediation pass over the highest risk.

## Lifecycle

Clients must call `initialize`, send `notifications/initialized`, then use
`tools/*`, `resources/*`, or `prompts/*`. Tool calls run through one background
worker built with standard-library channels, so the stdio loop can receive
`notifications/cancelled` while analysis is running. Calls that include
`_meta.progressToken` receive start/completion `notifications/progress`.
No async runtime is used.

HTTP transport, hosted MCP, sampling, and source upload are outside the current
local stdio MCP scope.

## Manual Smoke Test

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"resources/list"}' \
  | repopilot mcp --root .
```
