# RepoPilot 0.11.0 Release Announcement

RepoPilot 0.11.0 is the signal quality and CI trust release. It keeps the
local-first model intact while making scan output easier to act on in pull
requests and GitHub Actions.

## Suggested Post

RepoPilot 0.11.0 is ready.

This release focuses on signal quality, CI parity, and release trust:

- lower self-audit noise while keeping P0/P1 risk visible;
- first-party GitHub Action support for `doctor`, priority filters, focused rule
  scans, receipt output, and review gates;
- stable local workflows from `doctor` to `scan` or `review`, then AI-ready
  context and CI gating;
- no telemetry, no source upload, and no hosted scanner dependency.

Local release verification passed end-to-end with:

```bash
./scripts/verify-release.sh
```

Current self-audit proof for the 0.11.0 release candidate:

```text
p0=0
p1=0
p2=85
p3=31
scan engine timing: about 32-51ms on the RepoPilot repository
```

Try it locally:

```bash
repopilot doctor .
repopilot scan . --min-priority p2
repopilot review . --base origin/main --fail-on-priority p1
repopilot ai context .
```

GitHub Actions users can pin the release after publishing:

```yaml
- uses: MykytaStel/repopilot@v0.11.0
  with:
    command: scan
    min-priority: p2
    upload-sarif: "true"
```

Recommended first adopters:

- Rust CLI maintainers who want local, fast repository risk checks;
- React Native and Expo teams that need architecture and platform health signals;
- polyglot repositories that want one CI-friendly report across languages.

See the changelog for the full list of changes:
<https://github.com/MykytaStel/repopilot/blob/main/CHANGELOG.md>

## Release Proof

- `./scripts/verify-release.sh` passed end-to-end locally.
- `cargo publish --dry-run` packaged and verified `repopilot v0.11.0`.
- `npm pack --dry-run` produced `repopilot-0.11.0.tgz`.
- Product smoke tests passed against `target/release/repopilot`.
- Self-audit stayed below the release gate of `p2 <= 135`.

## Post-Publish Checks

Run these after the GitHub Release and npm publish workflows complete:

```bash
git ls-remote --tags origin v0.11.0
npm view repopilot version
for pkg in \
  @repopilot/darwin-arm64 \
  @repopilot/darwin-x64 \
  @repopilot/linux-arm64-gnu \
  @repopilot/linux-x64-gnu \
  @repopilot/win32-x64-msvc; do
  npm view "$pkg" version
done
npm install -g repopilot@0.11.0
repopilot --version
repopilot scan . --format json --output /tmp/repopilot-smoke.json
```
