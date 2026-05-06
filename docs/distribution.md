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

The release workflow runs `cargo publish --dry-run` for release tags, but crates.io publishing is manual.

Before publishing:

- confirm `Cargo.toml` metadata;
- review `cargo package --list`;
- run `cargo publish --dry-run`;
- publish from the exact tagged commit.

See [docs/release.md](release.md) for the full release process.

## Planned: Homebrew

A Homebrew formula is planned after the release artifact process is stable.
