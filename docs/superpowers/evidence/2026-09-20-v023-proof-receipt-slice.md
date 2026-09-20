# v0.23 Proof Receipt Slice Evidence

- Date: 2026-09-20
- Worktree: `feat/v023-proof-receipts`
- Base: `origin/main` at `61db9a8f63a50e93790a5d5fc78c9799afb287c1`
- Current committed head before the fix pass: `4e049656`
- The fix pass remains uncommitted in this isolated worktree; no release commit,
  tag, or publication was created.
- Package version: `0.22.0`
- Release/tag/publication: none; no `v0.23` tag was created

## Delivered behavior

- `ChangeProof` remains the only verdict source.
- Review JSON exposes additive `proof_receipt`.
- Review SARIF exposes the same value as `runs[0].properties.proofReceipt`.
- The receipt records schema/analyzer/report versions, workspace revision,
  configuration/policy hash, proof/evidence projections, normalized reason
  codes, next action, and unavailable inputs.
- Replay states are `matched`, `stale`, `unsupported`, `invalid`, and
  `unavailable`; replay never upgrades a verdict.
- Replay diagnostics carry a bounded code and reason; reasons are capped at
  256 characters. Serialized replay validates input size and schema before
  typed decoding, so malformed, future, and oversized payloads stay explicit.
- Receipt provenance is captured once per analysis session. Configuration
  fingerprints include check IDs, roles, paths, selected checks, intent policy,
  and critical paths; unavailable scanner configuration remains disclosed.
- Revision hashing does not follow symlinks outside the workspace and bounds
  regular-file reads. Receipt proof collections are canonicalized before both
  hashing and serialization.
- Existing `change_proof`, `evidence`, readiness fields, exit codes, MCP
  stored projections, and SARIF legacy properties remain present.

## Verification

| Check | Result |
| --- | --- |
| `cargo test --lib review::proof::receipt_tests -- --nocapture` | 13 passed |
| `cargo test --lib scan::session::tests -- --nocapture` | 7 passed |
| `cargo test --test review_proof_receipt -- --nocapture` | 1 passed |
| `cargo test --test review_projection_parity -- --nocapture` | 1 passed |
| `cargo test --test mcp_change_proof -- --nocapture` | 1 passed |
| `cargo test --test review_readiness -- --nocapture` | 16 passed |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | passed |
| `cargo test --all` | passed; 987 unit tests plus all integration and doc tests |
| `python3 scripts/release-contract.py check` | passed; release contract 0.22.0 |
| self-scan with P1 gate | exit 0; 0 findings |

The self-scan emitted one known informational diagnostic:
`analysis.unresolved-local-import-limited` reported 56 unresolved internal
imports with ambiguous or unsupported resolution semantics and did not treat
them as broken code.

## Test boundaries

- Replay tests cover revision mismatch, configuration mismatch including policy
  paths, future schema, malformed/oversized serialized input, tampered payload,
  missing context, bounded diagnostics, JSON round-trip, and byte-stable proof
  collection ordering.
- Workspace revision tests cover tracked edits, cache exclusion, Git and
  non-Git symlinks, and external target changes without external reads.
- Projection tests cover JSON/SARIF equality and preserve the existing MCP
  stored projection parity.
- No human-usefulness, demand, independent labeling, zoo precision/recall,
  platform install, dependency-advisory, or publication evidence was claimed.
- This slice does not make the package release-ready and does not authorize a
  version bump or tag.
