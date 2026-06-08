# Security Policy

## Supported Versions

Security fixes are provided for the latest published minor release. Older
`0.x` minors may receive a fix only when the same patch can be applied without
maintaining a separate compatibility branch.

| Version | Supported |
|---|---|
| Latest `0.x` minor | Yes |
| Older minors | No |

## Reporting a Vulnerability

If you discover a security issue in RepoPilot, please do not open a public issue.

Please report it privately through GitHub Security Advisories.

Include:

- affected version;
- steps to reproduce;
- expected impact;
- suggested fix if known.

Expect an initial acknowledgement within seven days. While a report remains
open, expect a status update at least every fourteen days. Disclosure timing is
coordinated after a fix and release are available.

## Scope

In scope:

- RepoPilot CLI parsing and local file access;
- Git diff/ref handling and repository-root confinement;
- GitHub Action installation, checksums, outputs, comments, and SARIF upload;
- npm wrapper binary selection and platform packages;
- release archives, checksums, attestations, and publishing workflows;
- MCP stdio request validation and root confinement.

Out of scope:

- findings that only report a false positive without a security boundary
  bypass;
- vulnerabilities in unsupported historical versions;
- third-party package registries, GitHub, Homebrew, or AI clients themselves.

## Local-First Runtime Model

RepoPilot runtime commands analyze files on disk and write reports to stdout or
explicit output paths. They do not upload source code, call AI services, or send
telemetry.

The curl installer downloads a versioned GitHub Release archive and its matching
`.sha256` file. Installation fails closed when the checksum cannot be downloaded
or verified.

The npm package has no install-time downloader. It selects a platform-specific
optional package under the `@repopilot/*` scope. Those native packages are
generated only from checksum-verified GitHub Release archives and are published
before the root package. `REPOPILOT_BINARY_PATH` can select a user-managed
binary when optional packages are unavailable.

Release workflows pin third-party Actions to full commit SHAs and pin installed
release/security tools to reviewed versions. Official releases require Cargo,
npm, Homebrew, and GitHub Release publication to complete successfully.
