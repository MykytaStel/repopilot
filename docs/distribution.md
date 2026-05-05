# Distribution

This document describes how RepoPilot is currently distributed and what future channels are planned.

## Current: source build

RepoPilot is not yet published to crates.io or distributed as pre-built binaries.

**Clone and build:**

```bash
git clone https://github.com/MykytaStel/repopilot.git
cd repopilot
cargo build --release
# binary: ./target/release/repopilot
```

**Install directly with cargo:**

```bash
cargo install --git https://github.com/MykytaStel/repopilot.git
```

## Prepared: crates.io

Once the CLI surface stabilizes at `0.1.0`, RepoPilot will be published to crates.io:

```bash
cargo install repopilot
```

Before publishing:

- Confirm `Cargo.toml` metadata fields (`description`, `license`, `repository`, `keywords`, `categories`).
- Run `cargo publish --dry-run` to catch packaging issues.

## Planned: GitHub Releases (pre-built binaries)

The release workflow builds binaries for the following targets on every `v*` tag:

| Target | Platform |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux x86-64 |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x86-64 |

Binaries and `.sha256` checksum files are attached to each GitHub Release. Users can download and run without installing Rust.

Tags containing a hyphen, such as `v0.1.0-test`, are treated as test/prerelease tags for workflow validation and do not publish to crates.io.

## Planned: Homebrew

A Homebrew formula will be added after GitHub Release artifacts are available. Users will be able to install with:

```bash
brew install repopilot
```
