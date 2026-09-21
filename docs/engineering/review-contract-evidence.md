# Review Contract Evidence

<!-- @generated from tests/fixtures/review-zoo/**/expected.json and tests/review_zoo.rs — do not edit by hand. -->
<!-- Regenerate with `python3 scripts/review_contract_scorecard.py --write`. -->

This scorecard reports **synthetic fixture evidence** for the contract deltas emitted by the review pipeline. It is a protocol and coverage check: the Rust `review_zoo` test runs the real CLI against every pair and matches these expectations. It does not establish real-repository precision or recall.

## Coverage

- Fixture pairs: 15 of 15 scenario pairs have both safe and unsafe variants.
- Runtime controls: 15 safe variants are required to emit zero review signals and zero contract deltas.
- Contract-positive unsafe fixtures: 2 of 15 unsafe variants; 4 contract expectations total.
- All contract evidence is `limited` confidence; static paths do not prove runtime reachability, authorization behavior, or test execution coverage.
- These counts describe committed fixtures and do not establish real-repository precision or recall.

## Contract-family matrix

| Contract family | Changes covered | Positive fixtures | Expectations | Scenario pairs |
|---|---|---:|---:|---:|
| `security-boundary` | `boundary-changed` | 2 | 2 | 2 |
| `test-coverage` | `test-changed`, `test-missing` | 2 | 2 | 2 |

## Fixture matrix

| Review family | Scenario | Unsafe contract evidence | Safe control |
|---|---|---|---|
| `behavioral` | `network-call` | none | zero contract deltas |
| `boundary` | `access-control` | `security-boundary/boundary-changed`, `test-coverage/test-missing` | zero contract deltas |
| `boundary` | `access-control-with-test` | `security-boundary/boundary-changed`, `test-coverage/test-changed` | zero contract deltas |
| `taint` | `array-destructuring` | none | zero contract deltas |
| `taint` | `destructuring-provenance` | none | zero contract deltas |
| `taint` | `exact-static-index` | none | zero contract deltas |
| `taint` | `fastify-request` | none | zero contract deltas |
| `taint` | `field-sensitive-local` | none | zero contract deltas |
| `taint` | `hono-request` | none | zero contract deltas |
| `taint` | `nestjs-controller` | none | zero contract deltas |
| `taint` | `nestjs-pipes` | none | zero contract deltas |
| `taint` | `nextjs-app-router` | none | zero contract deltas |
| `taint` | `property-aware-destructuring` | none | zero contract deltas |
| `taint` | `sql-injection` | none | zero contract deltas |
| `taint` | `static-subscript` | none | zero contract deltas |

## Reproduction

```bash
python3 scripts/review_contract_scorecard.py --check
cargo test --test review_zoo
```

The scorecard validates fixture metadata; `cargo test --test review_zoo` is the execution evidence that the current binary satisfies it.
