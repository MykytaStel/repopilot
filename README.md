# RepoPilot

[![Crates.io](https://img.shields.io/crates/v/repopilot.svg)](https://crates.io/crates/repopilot)
[![npm](https://img.shields.io/npm/v/repopilot.svg)](https://www.npmjs.com/package/repopilot)
[![CI](https://github.com/MykytaStel/repopilot/actions/workflows/ci.yaml/badge.svg)](https://github.com/MykytaStel/repopilot/actions)
[![License](https://img.shields.io/crates/l/repopilot.svg)](LICENSE)

**Local, deterministic review for Git changes.**

RepoPilot helps developers and teams inspect a change before merge. It reports
structural evidence about security boundaries, behavior, local imports and
exports, and the files affected through the dependency graph. The same review
can run from a terminal, in CI, or through an agent integration.

The analysis runs on the machine or CI runner that invokes it. It does not send
source to a hosted service or call an embedded language model. Findings point to
code and explain what to check; reviewers still decide whether a change is safe
for their application.

## Install and review

```bash
cargo install repopilot
# or
npm install -g repopilot

repopilot review . --base origin/main
```

For uncommitted work, run `repopilot review .`. The first screen gives one
`PASS`, `REVIEW`, `BLOCK`, or `NOT ASSESSED` decision, its reasons, coverage
limits, and a next action.

A review can surface:

- changes to authentication, request trust, deployment, dependencies, or secret
  configuration;
- added or removed behavior such as network calls, subprocesses, filesystem
  writes, SQL, or error handling;
- changed input-to-sink paths, algorithmic structure, or local import/export
  contracts;
- tests the change stopped running or weakened: committed focus markers such as
  `it.only`, newly skipped tests (`it.skip`, `@pytest.mark.skip`, `t.Skip`,
  `#[ignore]`), removed or substituted test cases, tests that lost assertions,
  new lint, type, or coverage suppressions, and relaxed CI or tool gates
  (`continue-on-error`, `|| true`, lowered coverage thresholds, strict mode
  off), so a green run cannot hide them;
- direct dependents and the wider impact of changed files.

Signals are advisory evidence. For example, a taint-lite signal shows that a
recognized input can reach a recognized sink in the changed source. Confirm the
impact in the context of the application and its configured checks.

## Example: review an image-processing change

The Wagtail example removes an authorization check and passes request data to a
subprocess in a one-file change. RepoPilot reports both the boundary change and
the input-to-process flow.

<p align="center">
  <img src="https://raw.githubusercontent.com/MykytaStel/repopilot/main/docs/demos/03-agent-review.gif" alt="RepoPilot review showing a removed authorization check and request input reaching a subprocess" width="800">
</p>

Replay the example on the pinned test repository:

```bash
python3 scripts/zoo.py clone --only wagtail
scripts/demo-agent-edit.sh .zoo/wagtail
repopilot review .zoo/wagtail
```

> A reported flow is a path to investigate. RepoPilot does not prove that it is
> exploitable or that the application is safe.

## Use in a team

Gate a branch review on high-confidence signals:

```bash
repopilot review . --base origin/main --fail-on-review definitely
```

A review is `VERIFIED` only when checks configured for the repository are
explicitly selected with `--verify` and pass on the reviewed revision. Generate
suggestions for a first setup with
`repopilot init --suggestions-output repopilot-suggestions.toml`; the file is
separate from active configuration. See [configuration](docs/configuration.md).

For a broader repository view, run `repopilot scan .`. Use
`repopilot baseline create .` to adopt existing findings before gating new work.

## Agent and CI integrations

The same review can check work from a human or a coding agent. Record a starting
point and review the changes made since it:

```bash
repopilot snapshot
# Work happens here.
repopilot review --since-snapshot
```

When the working tree is already dirty, the marker also records a baseline
commit of those uncommitted files, so the later review covers only what changed
after the snapshot. It shows what changed, not who changed it.
See [common workflows](docs/commands.md#review-work-since-a-marker).

In Claude Code, the RepoPilot plugin runs that loop for every session and
stops Claude from finishing while a test it skipped, focused, removed, or
weakened is unexplained:

```text
/plugin marketplace add MykytaStel/repopilot
/plugin install repopilot@repopilot
```

See [Guard your agent runs](docs/agent-guardrail.md).

RepoPilot also provides a local stdio MCP server and a GitHub Action. The MCP
server gives an agent access to the local scan and review tools. The Action runs
RepoPilot on the Actions runner and can publish SARIF or a pull request summary.
See [Guard your agent runs](docs/agent-guardrail.md), [MCP server](docs/mcp.md),
and [GitHub integration](docs/integrations/github-code-scanning.md).

## More capabilities

- `repopilot ai context .` creates a local handoff with repository facts,
  findings, and a prioritized plan. It makes no model calls.
- `repopilot init` creates configuration or integration files for review.
- Reports are available as console, Markdown, JSON, HTML, and SARIF.

## Documentation

- [Install](docs/install.md) · [Common workflows](docs/commands.md) ·
  [CLI reference](docs/cli.md)
- [Current architecture](docs/architecture.md) ·
  [Reports and schemas](docs/reports.md) · [Security model](docs/security.md)
- [Language support](docs/language-support.md) ·
  [Rules reference](docs/rules-reference.md) · [Roadmap](docs/roadmap.md)
- [Latest release notes](docs/releases/v0.23.0.md) ·
  [Maintainer documentation](docs/engineering/README.md)

Contributing and development setup: [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT OR Apache-2.0.
