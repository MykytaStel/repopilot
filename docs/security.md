# Security

RepoPilot is designed as a local-first Git change review and repository audit
tool.

## Runtime trust model

RepoPilot commands analyze files on disk and write reports to stdout or explicit
output paths. Runtime commands do not upload source code, call AI providers, send
telemetry, or contact RepoPilot servers.

The AI handoff command is a formatting command:

```bash
repopilot ai context .
```

It generates local Markdown from local scan findings. It does not call an LLM API.

MCP tool paths and finding replay are confined to the configured MCP root.
RepoPilot canonicalizes existing paths, checks the nearest existing ancestor for
missing outputs, and rejects lexical traversal and symlink escapes before an
analysis command runs.

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

The root npm package is a JavaScript wrapper around platform-specific optional
native packages under the `@repopilot/*` scope. It does not run a `postinstall`
script and does not download binaries during npm installation.

The platform packages are generated from GitHub Release archives after verifying
the matching `.sha256` checksum file. npm then installs the package matching the
user's OS, CPU, and, on Linux, glibc runtime.

Supported overrides:

```bash
REPOPILOT_BINARY_PATH=/path/to/repopilot
```

If optional dependencies are omitted or blocked by policy, set
`REPOPILOT_BINARY_PATH` or use the Cargo, Homebrew, or GitHub Releases channel.

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

## Secret handling

RepoPilot treats real provider-looking tokens and high-entropy values assigned to
secret-like names as security findings. Examples include OpenAI-style `sk-*`
tokens, Google/Gemini `AIza*` keys, GitHub tokens, AWS access keys, JWT-like
tokens, and other high-entropy literals.

Use explicit placeholders in code samples and setup UI strings:

```bash
export OPENAI_API_KEY=<OPENAI_API_KEY>
export GEMINI_API_KEY=${GEMINI_API_KEY}
export ANTHROPIC_API_KEY=your-anthropic-api-key
```

Placeholders such as `your-openai-api-key`, `replace-with-*`, `example-*`,
`<OPENAI_API_KEY>`, and `${OPENAI_API_KEY}` are not treated as leaked secrets.
If a real credential was committed, rotate it and consider the old value
compromised even if it is later removed from the working tree.

Secret-candidate and private-key evidence is centrally replaced with
`[sensitive evidence redacted]` in console, Markdown, HTML, AI-context Markdown,
review output, and rendered verification steps. Structured JSON and SARIF keep
their additive schema contract; built-in secret detectors mask credential values
before findings reach those formats.

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
- npm platform package generation and publishing;
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
