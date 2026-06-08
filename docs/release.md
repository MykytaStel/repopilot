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
workflow. Add a dated `CHANGELOG.md` section and leave
`[Unreleased]` ready for the next change.

Run the release contract:

```bash
python3 scripts/release-contract.py check
```

It verifies:

- all release versions and action pins agree;
- the matching changelog section exists and can produce release notes;
- local Markdown links resolve;
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
npm package dry-runs, rule evaluation, self-scan signal quality, and the
install-like product smoke suite.

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
build. It extracts the GitHub Release body from the matching changelog section,
builds and attests platform archives, publishes crates.io, calls npm publishing,
and updates the Homebrew tap. The npm workflow publishes checksum-verified
platform packages before the root package through Trusted Publishing. Missing
Cargo or Homebrew credentials fail the release before packaging.

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
