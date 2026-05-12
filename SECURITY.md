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

RepoPilot runtime commands analyze files on disk and write reports to stdout or explicit output paths. They do not upload source code, call AI services, or send telemetry.

The npm package installer downloads a matching GitHub Release binary during `postinstall` and verifies the release `.sha256` checksum. Set `REPOPILOT_SKIP_DOWNLOAD=1` to skip this download, or `REPOPILOT_BINARY_PATH` to use a user-managed binary.
