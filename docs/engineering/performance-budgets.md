# Performance Budgets

RepoPilot scan performance is part of the product contract. A local-first safety
pass must stay fast enough for pre-commit, AI-agent loops, and CI adoption.

## Initial Budgets

These budgets start with RepoPilot self-scan fixtures and should be tightened as
larger fixture repositories are added:

| Scenario | Budget |
|---|---:|
| Warm full scan accounted engine time | <= 600ms |
| Report finalization | <= 100ms |
| Changed scan cache lookup and reuse | No material regression after cache warmup |

A benchmark update should be explicit when a legitimate scan-engine improvement
changes expected work. Otherwise, avoid merging changes that regress an existing
budget by more than 10%.

## Measurement

Use:

```bash
repopilot scan . --profile strict --timing --format json --output /tmp/repopilot-self-scan.json
```

Track `scan_timings.accounted_engine_us()` from the timing breakdown and
`scan_timings.report_finalization_us` from the JSON report. Keep wall-clock
numbers as supporting context only; the budget should focus on accounted engine
timing so CI host variance does not create noisy failures.

## Benchmark

A Criterion benchmark builds a synthetic 480-file multi-language repository
(Rust, TypeScript, Python, Go) in a temp directory and measures full-scan wall
time:

```bash
cargo bench --bench scan_bench
```

The benchmark lives in `benches/scan_bench.rs`. Use it to record a baseline
before scan-engine changes — notably the shared parsed-AST work — and to prove
the expected speedup afterwards. Criterion stores results under
`target/criterion` and reports the delta versus the previous run, so benchmark
`main` first, then the change branch.

## v0.20 Benchmark Matrix

The following matrix extends the initial budgets for the 0.20 release. The
`self-scan` rows use accounted engine time (see Measurement above). The
synthetic small/medium/large rows below use wall-clock time from
`cargo bench --bench scan_bench` (Criterion, in-process function calls) —
accounted engine time is not yet plumbed through these fixtures, so budgets for
these rows are wall-clock and carry more headroom accordingly.

Synthetic sizes follow `SyntheticSize` in `tests/support/synthetic_repo.rs`:
small ~60 files, medium ~1000 files, large ~3000 files (Rust/TypeScript/Python/Go).

| Scenario | Size | Budget | Notes |
|---|---|---:|---|
| Warm full strict scan | self-scan | <= 600ms | existing 0.19 budget, accounted engine time |
| Report finalization | self-scan | <= 100ms | existing 0.19 budget, accounted engine time |
| Cold changed review | small synthetic | <= 150ms | baseline ~90ms (86-96ms observed); wall-clock, `cargo bench` |
| Warm changed review | small synthetic | <= 200ms | baseline ~124ms (108-132ms observed); at this scale, parsed-cache load/lookup overhead can exceed reparse savings — see note below |
| Warm full strict scan | medium synthetic | <= 700ms | baseline ~442ms (424-460ms observed); ~1000 files; wall-clock |
| Warm full strict scan | large synthetic | <= 1800ms | baseline ~1231ms (1212-1257ms observed); ~3000 files; wall-clock |
| Cold changed review | medium synthetic | <= 500ms | baseline ~303ms (265-339ms observed); ~1000 files; wall-clock |
| Warm changed review | medium synthetic | <= 350ms | baseline ~192ms (156-225ms observed); after cache warm; wall-clock |

Baselines recorded 2026-07-06 on a local Apple Silicon macOS dev machine
(`cargo bench --bench scan_bench`, release profile) — see "Measurement
environment" below; run-to-run variance up to ~30% was observed on this
machine between separate `cargo bench` invocations, which the budgets above
absorb. The release-only smoke gate (`scripts/check-scan-performance.js`,
`npm run scan:performance`) independently measures the medium full-scan and
medium changed-review scenarios by spawning the release binary; its budgets
(`MEDIUM_SCAN_BUDGET_MS`, `MEDIUM_CHANGED_COLD_BUDGET_MS`,
`MEDIUM_CHANGED_WARM_BUDGET_MS`) are separate, generous wall-clock ceilings
tuned for that measurement method and are not expected to match the Criterion
numbers above exactly.

### Small-scale cache overhead

The small-synthetic warm changed review (~124ms) is consistently *slower* than
its cold counterpart (~90ms), reproduced across repeated runs. At ~60 files,
loading and looking up the parsed-facts cache costs more than reparsing the
handful of unchanged files it would save; the benefit only appears at the
medium/large scale, where warm changed review is clearly faster than cold
(~192ms vs ~303ms). This is expected engine behavior, not a benchmark defect.

### Entries marked "to be baselined"

Historical note: the rows above were baselined in this PR (#270). Future
scenarios added to this matrix should follow the same process — a stable
synthetic fixture and a recorded measurement on the pinned environment before a
hard budget is set. The PR that introduces each fixture must record the
baseline and propose a budget. Until then the regression threshold (no more
than 10% unexplained regression) applies relative to the first recorded
measurement.

### Peak memory

Measure peak RSS using `/usr/bin/time -v` (Linux) or `/usr/bin/time -l` (macOS)
as a practical proxy. Record the result alongside the benchmark run. A hard
memory budget should be set only after baseline data from the pinned environment
is available.

### Thread determinism

Output determinism must be verified for 1-thread and multi-thread runs. The
determinism test compares the full JSON report (excluding wall-clock timing)
across `RAYON_NUM_THREADS=1` and default thread counts. `tests/perf_matrix.rs`
enforces this as a `cargo test --all` CI gate across 1/2/4 `RAYON_NUM_THREADS`
on the small and medium synthetic repos (`tests/support/synthetic_repo.rs`),
alongside the pre-existing tiny-fixture determinism tests in
`tests/parallel_scan.rs` and `tests/scan_changed_cache_cli.rs`.

### Cache behavior

Benchmark runs should record cache hit, miss, and invalidation rates for warm
scenarios. The cache benchmark covers:

- unchanged workspace (expect full hit);
- single file changed (expect targeted miss);
- new file added (expect miss for new file, hit for others);
- file removed (expect invalidation for removed file and its dependents).

`tests/perf_matrix.rs` enforces these four scenarios as a `cargo test --all` CI
gate against the parsed-cache v2 (#268) `cache_telemetry` fields on a small git
fixture. Wall-clock timing for the matrix above stays release-only
(`scripts/check-scan-performance.js`) since Criterion-scale timing is too noisy
for a per-PR gate.

### Measurement environment

Pin the benchmark environment to a specific machine profile or CI runner type.
Record the profile alongside results so regressions are comparable. Local
developer machines are acceptable for relative comparisons; absolute budgets
should be validated on the pinned environment.

### Rules for budget changes

- A benchmark update must be explicit and justified when a legitimate engine
  improvement changes expected work.
- Do not silently increase a budget. Any budget increase requires a PR
  description explaining the reason.
- Do not claim a speedup before it is measured on the pinned environment with the
  full benchmark matrix.
- The 10% regression threshold applies to accounted engine time. Wall-clock
  regressions are supporting context only.

### Separating accounted engine time from wall-clock time

Accounted engine time (`scan_timings.accounted_engine_us()`) includes parsing,
rule execution, graph construction, and report finalization. It excludes process
startup, argument parsing, I/O to the output file, and terminal rendering.
Wall-clock time is useful for user experience but is affected by system load,
I/O scheduling, and terminal speed. Budget enforcement uses accounted engine time
only.

### Baselining a change

Before measuring a change branch:

```bash
git switch main
cargo bench --bench scan_bench       # record baseline
git switch <change-branch>
cargo bench --bench scan_bench       # compare against baseline
```

Criterion stores results under `target/criterion` and reports the delta. The
baseline measurement must be on the same machine profile as the change
measurement.

## Fixture Direction

Small, medium, and large synthetic repositories now exist
(`tests/support/synthetic_repo.rs`, shared by `benches/scan_bench.rs`,
`tests/perf_matrix.rs`, and `scripts/check-scan-performance.js`). Determinism
and cache behavior are hard CI gates today via `tests/perf_matrix.rs`
(`cargo test --all`). Wall-clock timing regressions remain release-only
(`scripts/check-scan-performance.js`, `verify-release.sh`) until fixture timing
is proven stable enough on the project CI runners to justify a hard per-PR
timing gate.
