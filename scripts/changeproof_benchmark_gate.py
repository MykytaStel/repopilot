"""Conservative release gate over validated matched ChangeProof observations."""

from __future__ import annotations

from changeproof_benchmark_compare import compare_validated


def evaluate_gate(baseline: dict, candidate: dict) -> dict:
    comparison = compare_validated(baseline, candidate)
    failures = []
    summary = candidate["summary"]
    count = summary["case_count"]

    def require(condition: bool, code: str, details: object) -> None:
        if not condition:
            failures.append({"code": code, "details": details})

    require(count > 0 and summary["deterministic_cases"] == count,
            "proof-nondeterministic", summary["deterministic_cases"])
    require(summary["false_certainty_cases"] == 0,
            "false-certainty", summary["false_certainty_cases"])
    require(not comparison["regressions"], "proof-regression", comparison["regressions"])
    for name, counts in sorted(summary["dimensions"].items()):
        require(counts["failed"] == 0, f"controlled-{name}-failure", counts)

    old = baseline["summary"]["signals"]
    new = summary["signals"]
    for label, signals in (("baseline", old), ("candidate", new)):
        require(signals["measured_cases"] == count
                and signals["unavailable_cases"] == 0
                and signals["nondeterministic_cases"] == 0,
                f"{label}-signal-evidence-incomplete", signals["measured_cases"])

    def pairs(signals: dict, field: str) -> set[tuple[str, str]]:
        return {(item["id"], signal) for item in signals[field] for signal in item["signals"]}

    for field in ("misses", "safe_alerts"):
        added = sorted(pairs(new, field) - pairs(old, field))
        require(not added, f"new-{field}", added)

    # Aggregate per-case medians, not a median of percentage changes.
    # This is a screening budget, not a statistically significant speed claim.
    resources = {}
    for phase in ("cold", "warm"):
        samples = [row["resources"][phase] for row in comparison["cases"]]
        old_ms = sum(row["baseline_wall_ms_median"] for row in samples)
        new_ms = sum(row["candidate_wall_ms_median"] for row in samples)
        delta = (new_ms - old_ms) / old_ms * 100 if old_ms > 0 else None
        rss_pairs = [(row["baseline_peak_rss_kb"], row["candidate_peak_rss_kb"]) for row in samples]
        complete = bool(rss_pairs) and all(a is not None and b is not None for a, b in rss_pairs)
        old_rss = max(a for a, _ in rss_pairs) if complete else None
        new_rss = max(b for _, b in rss_pairs) if complete else None
        rss_delta = (new_rss - old_rss) / old_rss * 100 if complete and old_rss else None
        resources[phase] = {
            "baseline_total_median_ms": old_ms,
            "candidate_total_median_ms": new_ms,
            "wall_delta_pct": delta,
            "baseline_max_rss_kib": old_rss,
            "candidate_max_rss_kib": new_rss,
            "rss_delta_pct": rss_delta,
        }
        require(delta is not None and delta <= 10, f"{phase}-wall-budget", resources[phase])
        require(rss_delta is not None and rss_delta <= 10, f"{phase}-rss-budget", resources[phase])

    return {
        "schema_version": 1,
        "status": "pass" if not failures else "fail",
        "case_count": count,
        "failures": failures,
        "resources": resources,
        "comparison": comparison,
        "limitations": [
            "Controlled development corpus only; unavailable oracle dimensions are not treated as passes.",
            "No independent production precision, recall or confidence interval is established.",
            "Resource budgets assume matched hardware, load and build conditions; rerun noisy observations.",
            "Explicit binary hashes identify executable bytes, not their source commit or build provenance.",
        ],
    }
