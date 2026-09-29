# RepoPilot Architecture

RepoPilot uses one local analysis engine for command-line and agent workflows.
The GitHub Action installs the CLI on its runner and invokes the same commands.

## Review flow

```text
CLI: review / scan / ai context        MCP: scan / review tools
                \                       /
                 \                     /
                    command adapters
                           |
                    scan session
       discovery -> parsing and file facts -> import graph
                           |
          audits -> findings -> risk and visibility
                           |
       review combines the scan summary with the Git diff,
          changed-file signals, proof and decision records
                           |
       console | Markdown | JSON | HTML | SARIF | MCP response
```

A scan session discovers files, parses supported source, resolves imports where
possible, and runs configured audit rules. Review uses that scan summary with
the selected Git change to build boundary, behavioral, algorithmic, taint-lite,
contract, and impact evidence. Reports use the same finding and decision
records; renderers format those records for the selected output.

The graph is a static import/dependency model. It supports repository audits,
review impact, and AI context. Resolution depth varies by language and
repository configuration; unresolved relationships stay limited or unresolved
rather than being presented as proven edges. See the
[dependency graph reference](engineering/dependency-graph-v2.md) and
[language support](language-support.md).

## Trust boundary

The CLI and stdio MCP server read the selected repository on the machine where
they run. RepoPilot does not upload source or call a hosted language model. The
MCP server confines file access to its configured repository root. A configured
verification command runs only when the user explicitly selects its check ID
with `--verify` or the MCP `verify` parameter.

The GitHub Action runs in the GitHub Actions job. It installs a versioned,
checksum-verified CLI and can write artifacts or a pull request summary when
configured. See the [security model](security.md) for filesystem, verification,
and output details.

## Analysis limits

RepoPilot reports static evidence from supported syntax, paths, manifests, and
Git history. It does not execute the application or replace language compilers,
tests, or dedicated runtime security tools. A recognized input-to-sink path is
not proof of exploitability; a missing signal is not proof that a change is
safe. Review output describes the analyzed scope and its known limitations.

For the processing and rendering contract, see the
[engine pipeline](engineering/engine-pipeline.md). For supported CLI and agent
workflows, see [common workflows](commands.md) and
[Guard your agent runs](agent-guardrail.md).
