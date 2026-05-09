# Release process

RepoPilot releases are prepared in the repository, verified locally and in CI, tagged from `main`, and published by the release workflow when the required secrets are configured.

## 1. Prepare the release

Update:

- `Cargo.toml` version
- `Cargo.lock` package version, if needed
- `package.json` version
- `CHANGELOG.md`
- `README.md` if user-facing behavior changed

Use the current date for the release entry in `CHANGELOG.md`.

## 2. Verify locally

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo package --list
cargo publish --dry-run
npm run test:npm
npm pack --dry-run
```

Review the package contents from `cargo package --list` before publishing.

## 3. Create release branch

```bash
git checkout -b release/vX.Y.Z
```

## 4. Commit

```bash
git add .
git commit -m "chore: prepare vX.Y.Z release"
```

## 5. Merge after CI passes

Open a pull request, wait for CI, review the package metadata and docs, then merge into `main`.

## 6. Tag

```bash
git checkout main
git pull
git tag vX.Y.Z
git push origin vX.Y.Z
```

The tag workflow builds GitHub Release artifacts, creates or updates the GitHub Release once, runs `cargo publish --dry-run`, publishes crates.io with `CRATES_IO_TOKEN`, and updates the Homebrew tap with `HOMEBREW_TAP_TOKEN`. The separate `publish-npm.yml` workflow publishes npm through npm Trusted Publishing / GitHub OIDC when the GitHub Release is published. Optional secret-backed channels are skipped when their secret is absent.

## 7. Verify publishing

After the tag workflow passes, verify all configured distribution channels:

```bash
cargo install repopilot --force
npm install -g repopilot@X.Y.Z
brew tap mykytastel/repopilot
brew install repopilot
repopilot --version
brew test repopilot
```

If a publishing secret is intentionally absent, publish that channel manually from the exact tagged commit and a clean worktree.

For Homebrew failures, check the custom tap first: `MykytaStel/homebrew-repopilot` must exist, contain a `Formula/` directory, and be writable by `HOMEBREW_TAP_TOKEN`.

## 8. Review GitHub Release

Title:

```text
RepoPilot vX.Y.Z — short release name
```

Include:

- highlights;
- install commands for npm, cargo, and quick install;
- upgrade command;
- example usage;
- React Native example when relevant;
- link to [CHANGELOG.md](../CHANGELOG.md).

Example release body:

````md
## Highlights

- ...

## Install

```bash
npm install -g repopilot
cargo install repopilot
```

## Upgrade

```bash
npm update -g repopilot
cargo install repopilot --force
```

## Example

```bash
repopilot review . --base origin/main
```

See [CHANGELOG.md](https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md) for details.
````
