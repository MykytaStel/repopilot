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
version = "0.2.0"
```

Run `cargo build` to regenerate `Cargo.lock` with the new version.

### 2. Update `CHANGELOG.md`

- Rename the `[Unreleased]` heading to `[0.2.0] - YYYY-MM-DD`.
- Add a new empty `[Unreleased]` section at the top.
- Update the comparison links at the bottom of the file.

### 3. Open a release PR

Title: `chore: release v0.2.0`

Include only the `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` changes. CI must pass before merge.

### 4. Tag after merge

```bash
git tag v0.2.0
git push origin v0.2.0
```

Tag only commits that are on `main`. A tag on an unmerged commit creates confusion.

### 5. Create a GitHub Release

- Go to **Releases → Draft a new release**.
- Select the tag created above.
- Paste the CHANGELOG entry for this version as the release description.
- Attach pre-built binaries when the release workflow is in place (see `docs/distribution.md`).

## Notes

- A release workflow that produces pre-built binaries on tag push will be added in a future PR.
- Do not skip CI (`--no-verify`) or amend published commits.
