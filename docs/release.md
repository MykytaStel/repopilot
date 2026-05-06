# Release process

RepoPilot releases are prepared in the repository, verified locally and in CI, tagged from `main`, and published to crates.io manually by the maintainer.

## 1. Prepare the release

Update:

- `Cargo.toml` version
- `Cargo.lock` package version, if needed
- `CHANGELOG.md`
- `README.md` if user-facing behavior changed

Use the current date for the release entry in `CHANGELOG.md`.

## 2. Verify locally

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo package --list
cargo publish --dry-run
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

The tag workflow builds GitHub Release artifacts and runs `cargo publish --dry-run`. It does not publish to crates.io.

## 7. Publish to crates.io

After the tag workflow passes and the maintainer has reviewed the release:

```bash
cargo publish
```

Do not publish from a dirty worktree. Publish from the exact tagged commit.

## 8. Create GitHub Release

Title:

```text
RepoPilot vX.Y.Z — short release name
```

Include:

- highlights;
- install command;
- upgrade command;
- example usage;
- link to [CHANGELOG.md](../CHANGELOG.md).

Example release body:

````md
## Highlights

- ...

## Install

```bash
cargo install repopilot
```

## Upgrade

```bash
cargo install repopilot --force
```

## Example

```bash
repopilot review . --base origin/main
```

See [CHANGELOG.md](https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md) for details.
````
