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

The following matrix extends the initial budgets for the 0.20 release. All
values use accounted engine time unless noted otherwise.

| Scenario | Size | Budget | Notes |
|---|---|---:|---|
| Warm full strict scan | self-scan | <= 600ms | existing 0.19 budget |
| Report finalization | self-scan | <= 100ms | existing 0.19 budget |
| Cold changed review | small synthetic | to be baselined | first measurement needed |
| Warm changed review | small synthetic | to be baselined | after cache warm |
| Warm full strict scan | medium synthetic | to be baselined | ~1000 files |
| Warm full strict scan | large synthetic | to be baselined | ~3000 files |
| Cold changed review | medium synthetic | to be baselined | ~1000 files |
| Warm changed review | medium synthetic | to be baselined | after cache warm |

### Entries marked "to be baselined"

These scenarios require a stable synthetic fixture and a recorded measurement on
the pinned environment before a hard budget can be set. The PR that introduces
each fixture must record the baseline and propose a budget. Until then the
regression threshold (no more than 10% unexplained regression) applies relative
to the first recorded measurement.

### Peak memory

Measure peak RSS using `/usr/bin/time -v` (Linux) or `/usr/bin/time -l` (macOS)
as a practical proxy. Record the result alongside the benchmark run. A hard
memory budget should be set only after baseline data from the pinned environment
is available.

### Thread determinism

Output determinism must be verified for 1-thread and multi-thread runs. The
determinism test compares the full JSON report (excluding wall-clock timing)
across `RAYON_NUM_THREADS=1` and default thread counts.

### Cache behavior

Benchmark runs should record cache hit, miss, and invalidation rates for warm
scenarios. The cache benchmark covers:

- unchanged workspace (expect full hit);
- single file changed (expect targeted miss);
- new file added (expect miss for new file, hit for others);
- file removed (expect invalidation for removed file and its dependents).

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

Add small, medium, and large synthetic repositories before enforcing this as a
hard CI gate. The gate should fail on major regressions only after fixture timing
is stable on the project CI runners. The `scan_bench` synthetic repository is the
first such fixture.
