# Release Process

RepoPilot follows [Semantic Versioning](https://semver.org/).

## Versioning policy

| Increment | When to use |
|---|---|
| Patch `0.1.x` | Bug fixes, doc corrections, minor CI changes with no user-visible behavior change |
| Minor `0.x.0` | New findings, new output formats, new flags, non-breaking behavior changes |
| Major `x.0.0` | Breaking CLI changes, output schema changes, removed flags or features |

## Steps to cut a release

### 1. Bump the version

In `Cargo.toml`, update the `version` field:

```toml
[package]
version = "0.1.0"
```

Run `cargo build` to regenerate `Cargo.lock` with the new version.

### 2. Update `CHANGELOG.md`

- Rename the `[Unreleased]` heading to the release version and date.
- Add a new empty `[Unreleased]` section at the top.
- Update the comparison links at the bottom of the file.

### 3. Open a release PR

Title: `chore: release v0.1.0`

Include only the `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` changes. CI must pass before merge.

Before merging, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo publish --dry-run
```

### 4. Tag after merge

```bash
git tag v0.1.0
git push origin v0.1.0
```

Tag only commits that are on `main`. A tag on an unmerged commit creates confusion.

### 5. Confirm the GitHub Release

- The release workflow creates the GitHub Release on `v*` tags.
- Confirm all binary archives and `.sha256` files are attached.
- Confirm crates.io publish ran only for a real release tag, not a prerelease/test tag containing `-`.
- Paste or edit the CHANGELOG entry if generated release notes need cleanup.

## Notes

- Prerelease/test tags such as `v0.1.0-test` build artifacts but do not publish to crates.io.
- Do not skip CI (`--no-verify`) or amend published commits.
