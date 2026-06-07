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
| `repopilot_explain_file` | File classification and applicable rules | `rule`, `signal` |

Every tool accepts a workspace-relative `path` where applicable. Review defaults
to `scope=changed`, `profile=default`, `fail_on_review=none`, and
`detail=compact`. Compact output caps detailed findings and each signal tier at
20 total review signals; `detail=full` returns the complete JSON report. Scan
and review accept `filters.min_severity`, `filters.min_confidence`,
`filters.min_priority`, and `filters.rules`.

Tool failures use MCP `isError: true`. Malformed input produces JSON-RPC error
`-32700` instead of being skipped.

## Resources

The server exposes:

- `repopilot://rules`: the rule catalog;
- `repopilot://repository-summary`: workspace/config/baseline/feedback and
  session-result availability without triggering a scan;
- `repopilot://last-scan`: the last successful scan in this session;
- `repopilot://last-review`: the last successful review in this session.

Last-result resources appear after the corresponding tool has run. Identical
tool calls are served from the session cache.

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
