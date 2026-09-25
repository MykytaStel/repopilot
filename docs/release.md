# Release Process

RepoPilot releases are prepared in a short pull request, verified locally and in
CI, tagged from `main`, and published by the tag workflow.

## Product Contract

RepoPilot is a review-first local CLI. A release should improve at least one of:

- review precision or lower false-positive noise;
- stable CLI and machine-readable contracts;
- predictable CI and release behavior;
- clear first-run documentation;
- local-first trust and artifact security.

More commands, rules, formats, or distribution channels are not release goals by
themselves.

Official channels are crates.io, npm, Homebrew, and GitHub Releases.

RepoPilot `0.x` does not declare a minimum supported Rust version. CI and
release workflows pin an explicit Rust toolchain for reproducibility; that pin
is a build input, not a compatibility promise for older compilers.

## Prepare

Update the version consistently in Cargo, npm, the Action, and reusable
workflow. Add a dated `CHANGELOG.md` section for the full technical record, add
curated GitHub Release notes at `docs/releases/vX.Y.Z.md`, and leave
`[Unreleased]` ready for the next change.

Curated release notes should stay short and user-facing:

1. `title` and `description` front matter;
2. three to five highlights;
3. breaking changes or compatibility notes;
4. version-pinned Cargo and npm upgrade commands.

Do not add an H1 or a full changelog link to the curated note. The release
script uses the front-matter title as the GitHub Release title, renders the
description as the first body paragraph, and appends the tagged `CHANGELOG.md`
link automatically.

Run the release contract:

```bash
python3 scripts/release-contract.py check
```

It verifies:

- all release versions and action pins agree;
- the matching changelog section exists;
- the curated GitHub Release note exists, has the required structure, and can
  produce the release title and body;
- local Markdown links resolve;
- current MCP tool names and report version/schema claims match source-owned
  registries;
- stale npm installer claims are absent;
- third-party GitHub Actions are pinned to commit SHAs;
- the tag workflow directly calls reusable npm publishing;
- removed editor packaging does not return;
- `cargo package --list` contains only Cargo metadata, README, LICENSE, and
  `src/**`.

## Verify

Run the complete local gate:

```bash
./scripts/verify-release.sh
```

The gate runs formatting, clippy, all Rust tests, release-contract checks,
dependency policy and security checks, shell and workflow validation, Cargo and
npm package dry-runs, review and scan performance checks, rule evaluation,
self-scan signal quality, and the install-like product smoke suite.

Required product workflows include:

```text
version and help
init
review
scan in console, JSON, Markdown, and SARIF
baseline create and new-high gate
AI context
```

The release should not be blocked only because no new rules were added. Stable
behavior, low noise, and safe packaging take priority.

## Pull Request And Tag

Create a short release branch and PR:

```bash
git switch -c release/vX.Y.Z
git commit -am "chore: prepare vX.Y.Z"
git push -u origin release/vX.Y.Z
```

Merge only after required CI passes. Then tag the exact reviewed `main` commit:

```bash
git switch main
git pull --ff-only
git tag vX.Y.Z
git push origin vX.Y.Z
```

The release workflow validates the tag against repository metadata before any
build. It renders the GitHub Release title and body from
`docs/releases/vX.Y.Z.md`, appends the tagged full technical changelog link,
builds and attests platform archives, publishes crates.io, calls npm publishing,
and updates the Homebrew tap. The npm workflow publishes checksum-verified
platform packages before the root package through Trusted Publishing. Missing
Homebrew credentials fail the release before packaging. crates.io publishes
through Trusted Publishing (OIDC) from the `release` environment; the crate
rejects API-token publishes.

### Rehearse with a release candidate

Before a stable tag, rehearse the whole pipeline with a release candidate
(`vX.Y.Z-rc.N`). Cut it from a short-lived branch so `main` never carries a
prerelease version:

```bash
git switch -c release/vX.Y.Z-rc.N origin/main
# bump every version pin to X.Y.Z-rc.N (Cargo, Cargo.lock, package.json and its
# five platform pins, action.yml, the reusable workflow), then:
python3 scripts/release-contract.py check --tag vX.Y.Z-rc.N
git commit -am "chore: rehearse vX.Y.Z-rc.N"
git tag vX.Y.Z-rc.N
git push origin vX.Y.Z-rc.N
```

A release candidate needs technical entries under `[Unreleased]` in
`CHANGELOG.md`; curated notes are optional and synthesized when absent. The tag
workflow then:

- creates a GitHub prerelease that is never marked latest;
- publishes npm packages under the `next` dist-tag, leaving `latest` on the
  stable release;
- publishes a crates.io prerelease version, which `cargo install` does not pick
  by default;
- skips the Homebrew tap;
- verifies every channel and fails if npm `latest` moved.

The release contract rejects workflow changes that would let a release
candidate move a stable channel.

### Recover a partial publication

If the tag's Release run publishes some channels but not others, rerun only the
missing channel from `main`; both workflows check out the tag, skip a version
that is already published with a matching digest, and refuse a mismatch:

```bash
gh workflow run publish-crates.yml -f tag=vX.Y.Z
gh workflow run publish-npm.yml -f tag=vX.Y.Z
```

Both run in protected environments (`release` for crates.io, `npm` for npm)
and wait for a reviewer's approval. Then confirm every channel serves the exact
tagged artifacts:

```bash
gh workflow run verify-publication.yml -f tag=vX.Y.Z
```

The same check runs locally against a checkout of the tag:

```bash
VERSION=vX.Y.Z SOURCE_DIR=/path/to/tag/checkout scripts/verify-publication.sh
```

## Release Checklist

- `HOMEBREW_TAP_TOKEN` is the only required release secret. Check its expiry at
  <https://github.com/settings/tokens> before tagging and rotate it with
  `gh secret set HOMEBREW_TAP_TOKEN` when it is close to expiring.
- Trusted Publishing registrations must name the calling workflow; see
  [distribution](distribution.md).
- Approve the `release` (crates.io) and `npm` environment deployments when the
  tag workflow requests them.

## Verify Public Channels

After the workflows finish:

```bash
git ls-remote --tags origin vX.Y.Z
npm view repopilot version
cargo search repopilot --limit 1
brew update
brew upgrade repopilot
repopilot --version
```

The final workflow job verifies each `@repopilot/*` native package, GitHub
Release assets and checksums, crates.io, npm, and the Homebrew formula. Also
install one public CLI channel and run:

```bash
repopilot review . --base origin/main
repopilot scan . --format json --output /tmp/repopilot-X.Y.Z.json
```
