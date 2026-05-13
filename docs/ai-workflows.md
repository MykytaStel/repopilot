# AI-Assisted Workflows

RepoPilot helps prepare local, evidence-backed repository context for AI-assisted
development without uploading source code.

## Core workflow

```bash
repopilot scan .
repopilot ai context .
repopilot ai plan .
repopilot ai prompt .
```

Use this loop:

```text
scan -> understand -> plan -> prompt -> change -> review
```

## Generate context

```bash
repopilot ai context . --budget 4k --output repopilot-context.md
```

Use `context` when you want to paste a compact, structured repository brief into
Claude Code, Cursor, ChatGPT, or another assistant.

Useful options:

```bash
repopilot ai context . --focus security
repopilot ai context . --focus architecture
repopilot ai context . --focus quality
repopilot ai context . --focus framework
repopilot ai context . --budget 2k
repopilot ai context . --no-header
```

## Generate a remediation plan

```bash
repopilot ai plan . --output repopilot-plan.md
```

Use `plan` when you want deterministic next steps before editing code. The plan is
based on local findings, severity, evidence, and recommendations.

## Generate a paste-ready prompt

```bash
repopilot ai prompt . --budget 8k --output repopilot-prompt.md
```

Use `prompt` when you want a coding assistant to make a focused patch.

Recommended prompt constraints:

```text
Do not rewrite unrelated code.
Preserve existing CLI behavior.
Add or update tests for changed behavior.
Keep changes small and reviewable.
Run cargo fmt, clippy, and tests.
```

## Safer AI remediation loop

```bash
repopilot ai prompt . --focus security --output prompt.md
# paste prompt into your coding assistant
cargo test --all
repopilot review . --base origin/main --fail-on new-high
```

## Good focus values

| Focus | Use when |
|---|---|
| `security` | hardcoded secrets, private keys, committed `.env` files |
| `arch` / `architecture` | large files, coupling, circular dependencies |
| `quality` | long functions, complexity, code markers |
| `framework` | React, React Native, Expo, JS framework findings |
| `all` | full repository context |

## Token budgets

| Budget | Use when |
|---|---|
| `2k` | quick focused remediation |
| `4k` | default context for small/medium repos |
| `8k` | broader plan with more evidence |
| `16k` | larger repository context when the assistant supports it |

## What not to do

Avoid asking an assistant to “fix everything” on a large repository without scope.
Prefer one category or one priority group at a time:

```bash
repopilot ai prompt . --focus security --budget 4k
repopilot ai prompt . --focus architecture --budget 4k
```

## CI review after AI changes

Use `review` after AI-generated or AI-assisted edits:

```bash
repopilot review . --base origin/main --baseline .repopilot/baseline.json --fail-on new-high
```

This helps catch new high-risk findings introduced by the change rather than
blocking on accepted legacy debt.
