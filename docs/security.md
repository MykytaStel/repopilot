# Security

RepoPilot is designed as a local-first repository audit tool.

## Runtime trust model

RepoPilot commands analyze files on disk and write reports to stdout or explicit
output paths. Runtime commands do not upload source code, call AI providers, send
telemetry, or contact RepoPilot servers.

AI workflow commands are formatting commands:

```bash
repopilot ai context .
repopilot ai plan .
repopilot ai prompt .
```

They generate local Markdown from local scan findings. They do not call an LLM API.

## Installation security

### GitHub Release artifacts

Release binaries are published through GitHub Releases. Each archive is published
with a `.sha256` checksum file.

### Curl installer

The curl installer downloads:

1. the release archive;
2. the matching `.sha256` file.

Installation aborts if checksum verification cannot be completed.

### npm package

The npm package uses a `postinstall` script to download the matching native binary
from GitHub Releases. It verifies the `.sha256` checksum before copying the binary
into the package `vendor/` directory.

Supported overrides:

```bash
REPOPILOT_SKIP_DOWNLOAD=1
REPOPILOT_BINARY_PATH=/path/to/repopilot
```

### Homebrew

The Homebrew formula uses versioned GitHub Release artifacts and SHA256 checksums.

## Repository security recommendations

For RepoPilot itself and projects using RepoPilot in CI, prefer:

- protected `main` branch;
- required CI checks;
- CodeQL code scanning;
- no force-push to protected branches;
- minimal GitHub Actions permissions;
- dependency graph, Dependabot alerts, and Dependabot security updates;
- secret scanning;
- secret scanning push protection;
- release tags created from reviewed commits.

RepoPilot keeps `.github/dependabot.yml` for Cargo, npm, and GitHub Actions
updates. Security updates still require Dependabot security updates to be enabled
in the repository settings.

## GitHub Actions permissions

For scan-only workflows, prefer:

```yaml
permissions:
  contents: read
```

For SARIF upload:

```yaml
permissions:
  contents: read
  security-events: write
```

For release publishing, grant write permissions only to jobs that need them.

## Reporting vulnerabilities

Do not open a public issue for suspected vulnerabilities.

Use GitHub Security Advisories or the reporting instructions in `SECURITY.md`.
Include:

- affected version;
- installation channel;
- steps to reproduce;
- expected impact;
- suggested fix, if known.

## Security-sensitive areas

Treat these areas as security-sensitive:

- release workflow;
- npm `postinstall` installer;
- curl installer;
- binary checksums;
- package publishing credentials;
- path traversal behavior;
- secret detection logic;
- SARIF output consumed by CI systems.

## What RepoPilot is not

RepoPilot is not a replacement for:

- language-specific linters;
- type checkers;
- dependency vulnerability scanners;
- secret scanners;
- manual security review.

It is a repository-level audit and AI-remediation context layer that complements
those tools.
