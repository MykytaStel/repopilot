# MCP server

`repopilot mcp` runs a local [Model Context Protocol](https://modelcontextprotocol.io)
server so AI coding agents (Claude Code, Cursor, and other MCP clients) can call
RepoPilot as a tool — for example, to audit a change the agent just made before
proposing it.

It is the same engine as the CLI, exposed over a different transport:

- **Local-first.** The server runs on your machine over stdio. No source is
  uploaded, no telemetry is sent, and no AI service is called. Every tool runs
  the same on-disk analysis as the corresponding command.
- **Lean and synchronous.** The transport is a small JSON-RPC 2.0 loop over
  stdin/stdout — no async runtime and no extra dependencies. The protocol surface
  is intentionally small enough to audit.

## Register with an agent

The client launches `repopilot mcp` for you; you only register the command once.

Claude Code:

```bash
claude mcp add repopilot -- repopilot mcp
```

Any other MCP client uses the same idea — a stdio server whose command is
`repopilot mcp`. A typical client config entry:

```json
{
  "mcpServers": {
    "repopilot": {
      "command": "repopilot",
      "args": ["mcp"]
    }
  }
}
```

Make sure `repopilot` is on `PATH` (see [install.md](install.md)).

## Tools

All tools take a `path` that defaults to the current working directory and return
their result as text content (JSON, except `repopilot_context` which returns
Markdown). A tool that fails returns an MCP error result (`isError: true`) with a
message, rather than tearing down the connection.

### `repopilot_review_change`

Audit the current Git changes: scans the repository, splits findings into those
touching changed diff lines vs. the rest, and reports blast radius (files that
import the changed files). Mirrors `repopilot review`.

| Argument | Type | Description |
|----------|------|-------------|
| `path` | string | Repository path. Defaults to the current directory. |
| `base` | string | Base Git ref to diff against (e.g. `origin/main`). Optional; defaults to the working tree vs `HEAD`. |
| `head` | string | Head Git ref. Optional; only valid together with `base`. |

Returns the JSON review report (findings, in-diff status, blast radius).

### `repopilot_scan`

Full repository audit across architecture, coupling, code quality, security, and
testing. Mirrors `repopilot scan`.

| Argument | Type | Description |
|----------|------|-------------|
| `path` | string | Path to scan. Defaults to the current directory. |

Returns the JSON scan report (findings, metrics, risk summary).

### `repopilot_context`

A budgeted, AI-ready Markdown brief of the repository (risks, hotspots, structure)
for the agent to reason over before editing. Mirrors `repopilot ai context`.

| Argument | Type | Description |
|----------|------|-------------|
| `path` | string | Repository path. Defaults to the current directory. |
| `focus` | string | Optional focus: `security`, `architecture` (or `arch`), `quality`, `framework`, or `all`. |
| `budget` | integer | Optional approximate token budget. Defaults to the standard budget. |

Returns the brief as Markdown.

### `repopilot_explain_file`

Explains how RepoPilot classifies one file (role, language, test/production
context) and which rules and signals apply, with the resulting severity
decisions. Mirrors `repopilot inspect explain`.

| Argument | Type | Description |
|----------|------|-------------|
| `path` | string | Path to the file to explain. Required. |
| `rule` | string | Optional rule id to focus the explanation on. |
| `signal` | string | Optional signal id to focus on. |

Returns the JSON explanation.

## How it works

The server handles the MCP `initialize`, `tools/list`, and `tools/call` methods.
On `tools/call` it dispatches to one of the tools above, which calls the same
library entry points the CLI uses (`build_review_report`, the product scan,
`ai_context::render`, and the explain builder) and returns the rendered result.

Because stdout carries the JSON-RPC stream, the tools always run with progress
output disabled so nothing corrupts the protocol.

## Manual smoke test

You can drive the server by hand with newline-delimited JSON-RPC messages:

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"repopilot_scan","arguments":{"path":"."}}}' \
  | repopilot mcp
```

Each response is one line of JSON on stdout.
