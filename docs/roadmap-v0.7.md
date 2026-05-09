# v0.7 Roadmap

Status: v0.7 scope is implemented and this document is kept as the release planning record.

## Delivered

- Per-workspace framework scans with findings scoped to each package root.
- Deeper React Native dependency health checks for navigation, storage, animation, and native module compatibility.
- First-party GitHub Action wrapper.
- Vibe-coding positioning in README/release messaging without adding new CLI commands.

## Deferred from v0.7

The Vibe Check item below was delivered in v0.8.0; the remaining items are still future work.

- `repopilot vibe <path>` as a deterministic wrapper over existing scan data. Delivered in v0.8.0.
- `repopilot harden <path>` for prioritized remediation plans.
- `repopilot prompt` and AI-ready context export.
- Optional per-platform npm binary packages for faster installs and fewer postinstall downloads.

## Post-0.7 Direction

- Reuse `ScanSummary`, existing findings, severity levels, output writing, and the scan pipeline for the first Vibe Check implementation.
- Start with console and Markdown output only; defer JSON until the report shape is stable.
- Keep AI/LLM calls out of the MVP. AI-ready output should be generated locally from deterministic findings.

## Acceptance Signals

- New rules include evidence and tests.
- JSON additions remain backward-compatible.
- Release artifacts are available through crates.io, npm, GitHub Releases, and quick install script.
