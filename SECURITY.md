# Security Policy

## Reporting a Vulnerability

If you discover a security issue in RepoPilot, please do not open a public issue.

Please report it privately through GitHub Security Advisories.

Include:

- affected version;
- steps to reproduce;
- expected impact;
- suggested fix if known.

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
