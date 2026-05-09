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

The npm package downloads the matching GitHub Release binary during `postinstall`, verifies the `.sha256` checksum, and exposes the `repopilot` command through the package `bin` entry.

Environment overrides:

- `REPOPILOT_BINARY_PATH=/path/to/repopilot` uses an existing binary.
- `REPOPILOT_SKIP_DOWNLOAD=1` skips the postinstall download.

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

## Publishing Policy

The release workflow runs `cargo publish --dry-run` for release tags. Publishing is automated when the corresponding secrets are configured:

- `CRATES_IO_TOKEN` publishes the crate to crates.io.
- `NPM_TOKEN` publishes the npm package.
- `HOMEBREW_TAP_TOKEN` updates the Homebrew tap formula.

Before publishing:

- confirm `Cargo.toml` metadata;
- confirm `package.json` metadata and version;
- review `cargo package --list`;
- run `npm pack --dry-run`;
- run `cargo publish --dry-run`;
- tag from the exact commit that should be published.

See [docs/release.md](release.md) for the full release process.

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
Releases, verifies the SHA256 checksum, and installs to `~/.local/bin`.
