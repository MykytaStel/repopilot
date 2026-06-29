# MCP Server

`repopilot mcp --root PATH` runs RepoPilot as a local Model Context Protocol
server over stdio. It supports protocol versions `2025-11-25` and `2024-11-05`.
No source, telemetry, or tool result leaves the machine.

All filesystem arguments are resolved under `--root`; paths outside that
workspace are rejected.

## Register

```bash
claude mcp add repopilot -- repopilot mcp --root .
```

Generic client configuration:

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

## Tool Contract

Tools advertise `outputSchema`, return `structuredContent`, and retain text
content as a fallback for older clients. They are annotated as read-only,
non-destructive, idempotent, and closed-world.

| Tool | Purpose | Additional inputs |
|---|---|---|
| `repopilot_review_change` | Changed/full review with findings, signals, blast radius, and gate result | `base`, `head`, `config`, `baseline`, `scope`, `profile`, `fail_on_review`, `detail` |
| `repopilot_scan` | Repository or changed-scope JSON scan | `config`, `profile`, `scope`, `base` |
| `repopilot_context` | Budgeted AI-ready Markdown context | `config`, `profile`, `focus`, `budget` |
| `repopilot_explain_file` | File-role evidence and ordered rule decision trace | `rule`, `signal` |
| `repopilot_explain_finding` | Replay a file-scoped emitted finding by stable ID from the current MCP session | `finding_id`, `source` |

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

Every tool accepts a workspace-relative `path` where applicable. Review defaults
to `scope=changed`, `profile=default`, `fail_on_review=none`, and
`detail=compact`. Compact output caps detailed findings and each signal tier at
20 total review signals; `detail=full` returns the complete JSON report. Scan
and review accept `filters.min_severity`, `filters.min_confidence`,
`filters.min_priority`, and `filters.rules`.

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

HTTP transport, hosted MCP, sampling, and source upload are outside the 0.16
scope.

## Manual Smoke Test

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"resources/list"}' \
  | repopilot mcp --root .
```
