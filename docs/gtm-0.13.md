# RepoPilot 0.13 Go-To-Market

## Positioning

RepoPilot is the local-first repository risk context layer before AI agents, PRs,
and releases. It is not a replacement for CodeQL, Semgrep, Snyk, or Sonar. Use
those tools for deep SAST, dependency, and platform-specific assurance; use
RepoPilot to explain local repo structure, baseline accepted debt, and generate
AI-ready remediation context without uploading source.

## Demos

1. AI coding safety loop: run `repopilot scan .`, inspect visible P1/P2 risks,
   then generate `repopilot ai context . --format markdown` for local agent work.
2. Baseline-aware CI gate: create `.repopilot/baseline.json`, fail on new high
   risk, and show hidden strict-only suggestions as backlog rather than noise.
3. React Native/Expo release check: scan mobile release configuration, native
   project presence, Hermes/new architecture drift, and risky runtime exits.

## Adoption Assets

- README quickstart with default actionable scan behavior.
- 90-second terminal demo: default scan, strict scan, baseline gate, AI context.
- Sample Markdown and HTML reports showing raw versus visible metrics.
- Copy-paste GitHub Action workflow pinned to `MykytaStel/repopilot@v0.13.0`.
- “Why not just a linter/SAST?” comparison focused on local repo context,
  baseline-aware debt, and AI remediation handoff.

## Selling Proof Points

- Local-first: no telemetry, source upload, hosted scanner, plugin execution, or
  automatic LLM calls.
- Trust surface: schema `0.17`, finding provenance, rule lifecycle, rule quality
  gates, raw-vs-visible signal quality, and fixture-backed rule evaluation.
- Workflow fit: runs before AI agents, in PR review, and in CI without replacing
  existing SAST or dependency scanners.

## Current Market References

- GitHub CodeQL CLI documents SARIF output for sharing static analysis results
  with other systems: https://docs.github.com/en/code-security/reference/code-scanning/codeql/codeql-cli
- Semgrep documents `semgrep scan` for local scans and `semgrep ci` for
  organization/CI scans: https://semgrep.dev/docs/getting-started/cli
- Snyk Code positions itself as developer-first SAST:
  https://docs.snyk.io/scan-using-snyk/snyk-code
- SonarQube Cloud documents AI Code Assurance for AI-generated code standards:
  https://docs.sonarsource.com/sonarqube-cloud/ai-capabilities/ai-code-assurance
- GitHub Copilot coding agent, Claude Code, and OpenAI Codex all position coding
  agents as tools that can inspect repositories and perform development tasks:
  https://docs.github.com/en/copilot/using-github-copilot/coding-agent/about-assigning-tasks-to-copilot,
  https://docs.claude.com/en/docs/agents-and-tools/claude-code/overview,
  https://openai.com/codex
