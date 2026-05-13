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

The curl installer downloads the selected GitHub Release archive and its matching `.sha256` file, verifies the SHA256 checksum with `sha256sum` or `shasum`, and fails closed if the checksum file, verification tool, or checksum match is unavailable. Installation is aborted for safety instead of using an unverified archive.

The npm package installer downloads a matching GitHub Release binary during `postinstall` and verifies the release `.sha256` checksum. Set `REPOPILOT_SKIP_DOWNLOAD=1` to skip this download, or `REPOPILOT_BINARY_PATH` to use a user-managed binary.
