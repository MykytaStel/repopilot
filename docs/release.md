# Release process

RepoPilot releases are prepared in the repository, verified locally and in CI, tagged from `main`, and published by the release workflow when the required secrets are configured.

## 1. Prepare the release

Update:

- `Cargo.toml` version
- `Cargo.lock` package version, if needed
- `package.json` version
- `CHANGELOG.md`
- `README.md` if user-facing behavior changed
- release checklist for the target version, for example `docs/release-checklist-0.10.md`

Use the current date for the release entry in `CHANGELOG.md`.

## 2. Verify locally

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo audit
cargo deny check advisories licenses
cargo package --list
cargo publish --dry-run
./scripts/smoke-product.sh --binary ./target/release/repopilot --repo . --tmp-dir /tmp/repopilot-release-smoke
mapfile -d '' shell_scripts < <({ [[ -f install.sh ]] && printf '%s\0' install.sh; [[ -d scripts ]] && find scripts -maxdepth 1 -type f -name '*.sh' -print0; })
((${#shell_scripts[@]} == 0)) || bash -n "${shell_scripts[@]}"
((${#shell_scripts[@]} == 0)) || shellcheck "${shell_scripts[@]}"
mapfile -d '' workflows < <(find .github/workflows -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -print0)
((${#workflows[@]} == 0)) || actionlint "${workflows[@]}"
npm test
npm pack --dry-run
```

Review the package contents from `cargo package --list` before publishing.
When release archives already exist in `dist/`, verify npm platform package
generation with:

```bash
node scripts/build-npm-platform-packages.js --dist dist --out /tmp/repopilot-npm-platform-packages
```

Install `cargo-audit`, `cargo-deny`, `shellcheck`, and `actionlint` before running the local release checks.
For a version-specific gate list, use the current release checklist in `docs/release-checklist-*.md`.
The product smoke suite validates the adoption flow, including receipt generation.

## npm Trusted Publishing setup

Before the first npm publish, configure npm Publishing Access for `repopilot` and
each `@repopilot/*` platform package:

```text
Publisher: GitHub Actions
Repository owner: MykytaStel
Repository name: repopilot
Workflow filename: publish-npm.yml
Environment name: npm
```

Keep package publishing access set to "Require two-factor authentication and
disallow tokens". The `publish-npm.yml` job uses the `npm` GitHub Environment and
OIDC, so no `NPM_TOKEN` secret is required.

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

The tag workflow builds GitHub Release artifacts, creates or updates the GitHub Release once, runs `cargo publish --dry-run`, publishes crates.io with `CRATES_IO_TOKEN`, and updates the Homebrew tap with `HOMEBREW_TAP_TOKEN`. The separate `publish-npm.yml` workflow downloads the GitHub Release artifacts, generates checksum-verified `@repopilot/*` platform packages, publishes those packages first, then publishes the root `repopilot` package through npm Trusted Publishing / GitHub OIDC. Optional secret-backed channels are skipped when their secret is absent.

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
