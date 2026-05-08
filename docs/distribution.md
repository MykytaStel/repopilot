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

The release workflow runs `cargo publish --dry-run` for release tags. npm publishing is automated when `NPM_TOKEN` is configured. crates.io publishing remains manual.

Before publishing:

- confirm `Cargo.toml` metadata;
- confirm `package.json` metadata and version;
- review `cargo package --list`;
- run `npm pack --dry-run`;
- run `cargo publish --dry-run`;
- publish from the exact tagged commit.

See [docs/release.md](release.md) for the full release process.

## Homebrew (via tap)

A formula template lives at [`Formula/repopilot.rb`](../Formula/repopilot.rb) in the repo.

To publish it:

1. Create a new GitHub repo **`MykytaStel/homebrew-repopilot`**.
2. Copy `Formula/repopilot.rb` into `Formula/repopilot.rb` in that repo.
3. On each release, update `version` and replace every `PLACEHOLDER_SHA256` with the real
   value from `repopilot-checksums.txt` attached to the GitHub Release.
4. Users install with:

```bash
brew tap mykytastel/repopilot
brew install repopilot
```

## Quick install (curl | sh)

An `install.sh` script is provided in the repo root for one-line installs:

```bash
curl -fsSL https://raw.githubusercontent.com/MykytaStel/repopilot/main/install.sh | sh
```

The script detects OS and architecture, downloads the correct pre-built binary from GitHub
Releases, verifies the SHA256 checksum, and installs to `~/.local/bin`.
