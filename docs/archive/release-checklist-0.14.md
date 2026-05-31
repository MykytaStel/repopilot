# RepoPilot 0.14.0 Release Checklist

RepoPilot 0.14.0 is the AI-guardrail release for the pre-1.0 line.

Main release promise:

```text
parse once -> AST-precise findings -> local MCP tools for AI agents -> still nothing leaves your machine
```

## Release Scope

- Keep the primary CLI families: `scan`, `review`, `baseline`, `compare`, `doctor`, `ai`, `inspect`, `init`, `cache`, and the new `mcp`.
- Keep runtime commands local-first: no telemetry, source upload, hosted scanner, plugin runtime, external rule execution, or LLM API calls.
- The `repopilot mcp` server is local-first too: stdio JSON-RPC only, no network egress, no AI service calls, and no source upload. Its tools run the same on-disk analysis as the CLI.
- Use `repopilot::api::*` as the supported Rust embedding facade; treat direct internal modules as unsupported implementation detail.
- Keep rule growth limited to regression fixes only; this release upgrades detection precision (AST), not rule coverage.

## Engine Contract Gates

Verify every rendered finding has:

- stable `id`;
- non-empty `rule_id`;
- non-empty title and description;
- non-empty recommendation;
- at least one evidence item;
- risk score, priority, formula version, and risk signals;
- provenance with detector, lifecycle, signal source, and scope;
- schema-safe serialization in JSON and SARIF.

Rule metadata gates:

- every registered rule has non-empty title, description, recommendation;
- every registered rule has lifecycle, signal source, and default confidence;
- runtime-risk and long-function rules report `ast` signal source (line scanner remains a parse-failure fallback only);
- `repopilot inspect rules`, `inspect rule`, and `inspect eval-rules` pass locally;
- `scripts/smoke-product.sh` verifies the self-scan JSON report has zero finding contract violations and zero default-visible noisy findings.

## MCP Gates

```bash
cargo test --test mcp_cli
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | cargo run -- mcp
```

Verify:

- `initialize` returns `serverInfo.name = "repopilot"` and a `tools` capability;
- `tools/list` advertises `repopilot_review_change`, `repopilot_scan`, `repopilot_context`, and `repopilot_explain_file`, each with an object input schema;
- `tools/call` returns local results and reports tool failures in-band (`isError: true`) rather than as transport errors;
- the server performs no network egress and calls no AI service.

## Smoke commands

```bash
cargo run -- scan . --format json --output /tmp/repopilot-014-scan.json --receipt /tmp/repopilot-014-receipt.json
cargo run -- scan . --format sarif --output /tmp/repopilot-014.sarif
cargo run -- baseline create . --output /tmp/repopilot-014-baseline.json
cargo run -- scan . --baseline /tmp/repopilot-014-baseline.json --fail-on new-high
cargo run -- review . --format json --output /tmp/repopilot-014-review.json
cargo run -- ai context . --budget 2k --output /tmp/repopilot-014-ai-context.md
cargo run -- inspect rules --format json --output /tmp/repopilot-014-rules.json
cargo run -- inspect eval-rules --format json --output /tmp/repopilot-014-rule-eval.json
```

## Required Local Checks

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
npm run test:npm
npm pack --dry-run
cargo package --list
cargo publish --dry-run
./scripts/verify-release.sh
cargo build --release
./scripts/smoke-product.sh --binary ./target/release/repopilot --repo .
```

## Version Consistency

Before tagging, verify:

```bash
grep '^version = "0.14.0"' Cargo.toml
grep '"version": "0.14.0"' package.json
grep '## \[0.14.0\]' CHANGELOG.md
rg -n '0\.13\.0|release-checklist-0\.13' Cargo.toml package.json action.yml README.md docs .github scripts install.sh Formula examples
```

The final search should only report historical changelog entries, old release
checklists, old release announcements, and examples intentionally pinned to old
versions.
