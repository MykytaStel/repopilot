# Distribution

This document describes how RepoPilot is currently distributed and what future channels are planned.

## Current: crates.io

RepoPilot is published on crates.io:

```bash
cargo install repopilot
```

Upgrade an existing install:

```bash
cargo install repopilot --force
```

Crate page: <https://crates.io/crates/repopilot>

## Current: npm

RepoPilot is also packaged as an npm CLI wrapper around the native Rust binary:

```bash
npm install -g repopilot
```

The npm package exposes the `repopilot` command through the package `bin` entry
and depends on platform-specific optional native packages:

- `@repopilot/darwin-arm64`
- `@repopilot/darwin-x64`
- `@repopilot/linux-arm64-gnu`
- `@repopilot/linux-x64-gnu`
- `@repopilot/win32-x64-msvc`

npm selects the matching optional package during install. RepoPilot does not run
a `postinstall` downloader and does not download GitHub Release assets during npm
installation. Normal `repopilot` CLI execution remains local-first and does not
upload source code or call external services.

Environment overrides:

- `REPOPILOT_BINARY_PATH=/path/to/repopilot` uses an existing binary when optional dependencies are omitted or unsupported.

Package page: <https://www.npmjs.com/package/repopilot>

## Current: source build

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
```

The binary is written to:

```text
./target/release/repopilot
```

Install directly from GitHub:

```bash
cargo install --git https://github.com/MykytaStel/repopilot.git
```

## Current: GitHub Releases

The release workflow builds binaries for the following targets on every `v*` tag:

| Target | Platform |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux x86-64 |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x86-64 |

Binaries and `.sha256` checksum files are attached to GitHub Releases.

## Runtime and Artifact Security Model

- Runtime scans read files from disk and write reports to stdout or an explicit `--output` path.
- Runtime commands do not call AI providers, telemetry endpoints, or RepoPilot servers.
- AI workflow commands (`ai context`, `ai plan`, `ai prompt`) only format local scan findings as Markdown.
- The curl installer downloads a GitHub Release artifact and its `.sha256` checksum, then fails closed if the checksum file cannot be downloaded, no SHA256 tool is available, or verification fails.
- npm installation uses platform-specific optional packages and does not run a downloader script.
- Platform npm packages are generated from GitHub Release archives only after their SHA256 checksum files are verified.
- `REPOPILOT_BINARY_PATH` can point the npm wrapper at a user-managed binary when optional dependencies are omitted or unsupported.

Before v1, the preferred hardening path is GitHub artifact attestations for release assets in addition to npm Trusted Publishing provenance.

## Publishing Policy

The release workflow runs `cargo publish --dry-run` for release tags. Publishing is automated when the corresponding channel is configured:

- `CRATES_IO_TOKEN` publishes the crate to crates.io.
- `publish-npm.yml` publishes the platform npm packages first, then the root npm package, through npm Trusted Publishing / GitHub OIDC, without an npm token secret.
- `HOMEBREW_TAP_TOKEN` updates the Homebrew tap formula.

If an optional publishing secret is absent, that channel is skipped without failing the release workflow. The GitHub Release binaries are still built and attached.

Before publishing:

- confirm `Cargo.toml` metadata;
- confirm `package.json` metadata and version;
- review `cargo package --list`;
- run `npm pack --dry-run`;
- run `cargo publish --dry-run`;
- tag from the exact commit that should be published.

See [docs/release.md](release.md) for the full release process.

For RepoPilot 0.12.0, publish the GitHub Release assets first, then the platform
npm packages, then the root npm package. The root `repopilot` package declares
optional dependencies on the platform packages at the same version, so publishing
the root package before the platform packages can create a broken install window.

After publishing, verify:

```bash
npm view repopilot version
for pkg in \
  @repopilot/darwin-arm64 \
  @repopilot/darwin-x64 \
  @repopilot/linux-arm64-gnu \
  @repopilot/linux-x64-gnu \
  @repopilot/win32-x64-msvc; do
  npm view "$pkg" version
done
git ls-remote --tags origin v0.12.0
```

## Homebrew (via tap)

A formula template lives at [`Formula/repopilot.rb`](../Formula/repopilot.rb) in the repo.

RepoPilot uses a custom Homebrew tap, not `homebrew/core`. Homebrew recommends GitHub tap repositories with the `homebrew-` prefix so users can install them with the short `brew tap user/name` form; see [How to Create and Maintain a Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap).

One-time setup:

1. Create a new GitHub repo **`MykytaStel/homebrew-repopilot`**.
2. Add an empty `Formula/` directory to that repo and commit it.
3. Create a fine-grained GitHub PAT with contents write access to `MykytaStel/homebrew-repopilot`.
4. Add that token to the main `MykytaStel/repopilot` repository as the `HOMEBREW_TAP_TOKEN` Actions secret.

On each release tag, the `update-homebrew-tap` job downloads the GitHub Release artifacts, extracts their SHA256 checksums, replaces the placeholders in `Formula/repopilot.rb`, writes `tap/Formula/repopilot.rb`, and pushes the formula update to the tap.

Users install with:

```bash
brew tap mykytastel/repopilot
brew install repopilot
```

After a release, verify the tap with:

```bash
brew tap mykytastel/repopilot
brew install repopilot
repopilot --version
brew test repopilot
```

If the tap update fails, check that the tap repo exists, the `Formula/` directory exists, `HOMEBREW_TAP_TOKEN` has write permission, and the macOS/Linux `.tar.gz` release artifacts exist.

## Quick install (curl | bash)

An `install.sh` script is provided in the repo root for one-line installs:

```bash
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | bash
```

The script detects OS and architecture, downloads the correct pre-built binary from GitHub
Releases, verifies the SHA256 checksum, and installs to `~/.local/bin`. The checksum
file and a local SHA256 tool (`sha256sum` or `shasum`) are required; installation
aborts for safety if verification cannot be completed.
