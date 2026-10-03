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
| Google Antigravity | Generic stdio MCP and manual snapshot/review | Native MCP loading and correct workspace root; migrated hooks require their own session proof |
| Cursor | Project hooks | Valid JSON replies, one follow-up, observed hook loading |
| Copilot coding agent | Setup steps, MCP, PR Action | Setup-job execution and a real PR review; no stop-hook claim |
| Other MCP clients | Generic stdio entry and AGENTS.md recipe | Actual tool discovery in that client; no automatic-stop claim |

Native CLI support covers macOS, Linux, and Windows release assets. The POSIX
hook recipes are tested on Unix. Record the exact client/platform version and
whether evidence is configuration, scripted protocol, or an actual client
session. Never promote one level to another.

One snapshot belongs to one Git working tree. Parallel agent sessions use
separate worktrees until session-scoped baselines are implemented and tested.

## Observed owner setup (2026-10-03, macOS ARM64)

The public Homebrew CLI is 0.24.0. The Claude/Codex plugin and Gemini extension
are installed, enabled, and their hook scripts match the tagged source.

| Client version | Observation | Remaining boundary |
|---|---|---|
| Claude Code 2.1.216 | Authenticated; plugin loaded; native SessionStart baseline, successful MCP context, and injected skipped-test Stop feedback observed | Controlled fixture; the bounded first run hit its five-turn limit after feedback, then a resumed turn acknowledged the signal and exited successfully |
| Codex CLI 0.144.6, model gpt-5.5 | Both RepoPilot hooks explicitly trusted; native SessionStart wrote the baseline; MCP context completed; native Stop returned the injected skipped-test signals to the model once | Controlled owner fixture, not an external user sample; the configured gpt-6.1-sol model was rejected by this CLI/account |
| Gemini CLI 0.62.0 | Version-pinned extension installed/enabled; Google login attempted | Provider rejected Gemini Code Assist for individuals and directed migration to Antigravity; native Gemini session remains unobserved |
| Antigravity CLI 1.2.15 | Official checksum-verified CLI installed; native MCP context returned the requested fixture data and a successful model response; exact context-tool permission granted | Automatic stop unverified; imported legacy hooks disabled because their workspace root was fixed at import time |
| Cursor | Three real-binary scripted protocol tests passed | Client not installed; native session unobserved |
| Copilot cloud agent | Repository setup workflow installs pinned 0.24.0 and snapshots; MCP recipe documented | Workflow execution, repository MCP settings, and an actual agent PR are separate checks |

Installation or a scripted protocol pass does not close a native session claim.
Update this table after observing the missing client step, with its exact
version and platform. Never commit account configuration or raw transcripts.

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

For 0.24.0 publication verification, use **Node 24.21.0 and npm 11.19.0**,
the observed publisher runtime, on a clean checkout of the tag. Node 26 produced
different gzip bytes for identical package contents during owner verification.
The verifier rejects a different runtime before reading public channels; never
weaken the digest comparison or replace an immutable published package to fix
this local reproduction mismatch. Publisher and verification workflows pin
the same Node version. The manual verifier selects recorded runtimes by tag
from `scripts/publication_runtime.py`, including 0.22's older publisher.
Unknown tags fail before channel reads; record their observed publisher runtime
when preparing a release. Recheck this contract when updating the runtime.

After the stable channels pass, run `publish-mcp-registry.yml` for the same tag
and verify the exact `io.github.MykytaStel/repopilot` name/version.

The Claude plugin directory lists the plugin from
[MykytaStel/repopilot-plugin](https://github.com/MykytaStel/repopilot-plugin),
a mirror where `integrations/claude-code/repopilot` is the repository root. The
directory blocks hook and MCP commands with computed paths only for a plugin in
a subfolder, and our hook scripts compute the user's repository paths. Run
`scripts/sync-plugin-repo.sh` after every merge that changes the plugin folder,
and at each release; the directory rescans new mirror commits. Never commit to
the mirror directly: the sync is a fast-forward of `git subtree split`. Registry
publication is a separate operation, not implied by npm/crates.io success.
Update RP24-016, its publication artifact, the generated release scorecard,
the public release status, and version-pinned installation instructions.

Release notes distinguish verified channels and client sessions from scripted
tests and pending manual steps. Catalog submissions and announcements use
reviewable prepared copy and the owner's chosen accounts; publishing a release
does not authorize messaging people or posting to communities.
