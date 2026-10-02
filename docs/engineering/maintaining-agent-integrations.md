# Maintaining Agent Integrations

The public installation and explanation path is
[Guard your agent runs](../agent-guardrail.md). This page defines maintenance;
release progress belongs in the current evidence ledger, and product priorities
belong in [the roadmap](../roadmap.md).

## Compatibility contract

| Client | Shipped integration | Evidence required for a compatibility claim |
|---|---|---|
| Claude Code | Marketplace plugin, SessionStart/Stop, MCP, skill | Manifest validation, scripted hooks, observed client loading/trust |
| Codex | Same plugin, Codex manifest and hooks | Plugin discovery, explicit hook trust, scripted hooks, observed session |
| Gemini CLI | Extension, SessionStart/AfterAgent, MCP | Extension discovery/validation, hook consent, scripted hooks, observed session |
| Cursor | Project hooks | Valid JSON replies, one follow-up, observed hook loading |
| Copilot coding agent | Setup steps, MCP, PR Action | Setup-job execution and a real PR review; no stop-hook claim |
| Other MCP clients | Generic stdio entry and AGENTS.md recipe | Actual tool discovery in that client; no automatic-stop claim |

Native CLI support covers macOS, Linux, and Windows release assets. The POSIX
hook recipes are tested on Unix. Record the exact client/platform version and
whether evidence is configuration, scripted protocol, or an actual client
session. Never promote one level to another.

One snapshot belongs to one Git working tree. Parallel agent sessions use
separate worktrees until session-scoped baselines are implemented and tested.

## Triage an installation report

Ask for OS, agent version, CLI version, installation channel, and the exact
failing command. Check the resolved executable (`command -v repopilot` on Unix,
`Get-Command repopilot` in PowerShell), then plugin/extension enabled state,
hook trust, Git working directory, and snapshot existence. A plugin install
does not install the CLI or automatically establish hook trust.

Reproduce with a clean small Git repository. Add one regression for the failure
and one successful-path guard, then fix the smallest responsible integration.
Request only the relevant diagnostic; do not ask users to publish credentials,
complete agent settings, private source, or full transcripts.

## Regular maintenance

- Review new installation failures and false alarms first. Keep one small
  reproducible issue per failure, including the affected agent/platform.
- For dependency or client-protocol changes, run the focused hooks and MCP
  tests. Then run the repository handoff gate before opening a PR.
- After an upstream hook/schema change, validate the affected manifest and
  perform one new client loading/trust check. Pin the observed version in the
  release evidence instead of claiming compatibility with every future client.
- Record adoption counters periodically with `scripts/adoption_stats.py`.
  Preserve unavailable values and interpret mirror downloads/code mentions
  separately from observed use. RepoPilot adds no source telemetry.

## Release maintenance

Follow [the release process](../release.md): full local verification, required
hosted CI, tag the reviewed main commit, approve the existing protected
deployments, and verify public artifacts by digest. Recover only a failed
channel; never republish a different artifact under the same version.

After the stable channels pass, run `publish-mcp-registry.yml` for the same tag
and verify the exact `io.github.MykytaStel/repopilot` name/version. Registry
publication is a separate operation, not implied by npm/crates.io success.
Update RP24-016, its publication artifact, the generated release scorecard,
the public release status, and version-pinned installation instructions.

Release notes distinguish verified channels and client sessions from scripted
tests and pending manual steps. Catalog submissions and announcements use
reviewable prepared copy and the owner's chosen accounts; publishing a release
does not authorize messaging people or posting to communities.
