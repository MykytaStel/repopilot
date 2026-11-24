"""Matched comparison of validated controlled ChangeProof benchmark artifacts."""

from __future__ import annotations

from statistics import median

from changeproof_benchmark_contract import BenchmarkManifestError


def compare_validated(baseline: dict, candidate: dict) -> dict:
    """Compare identical fixture/manifest inputs, not independent populations."""
    left, right = baseline["artifact"], candidate["artifact"]
    for field in ("protocol", "corpus", "manifest_sha256"):
        if left[field] != right[field]:
            raise BenchmarkManifestError(f"comparison requires matching {field}")
    for artifact in (left, right):
        if not artifact["scanner"].get("binary_sha256"):
            raise BenchmarkManifestError("comparison requires binary_sha256 on both artifacts; recollect")
    if left["scanner"]["binary_sha256"] == right["scanner"]["binary_sha256"]:
        raise BenchmarkManifestError("comparison requires distinct scanner binaries")
    old = {case["id"]: case for case in left["cases"]}
    new = {case["id"]: case for case in right["cases"]}
    if old.keys() != new.keys():
        raise BenchmarkManifestError("comparison requires identical case IDs")
    rows = []
    for case_id in sorted(old):
        a, b = old[case_id], new[case_id]
        for field in ("fixture", "variant", "fixture_source_sha256", "oracle"):
            if a[field] != b[field]:
                raise BenchmarkManifestError(f"case {case_id}: mismatched {field}")
        comparable = a["deterministic"] and b["deterministic"]
        transitions = {}
        for dimension in sorted(a["runs"][0]["evaluation"]):
            before = a["runs"][0]["evaluation"][dimension]["status"] if a["deterministic"] else "unavailable"
            after = b["runs"][0]["evaluation"][dimension]["status"] if b["deterministic"] else "unavailable"
            transitions[dimension] = {"baseline": before, "candidate": after}
        rows.append({
            "id": case_id,
            "semantic_equal": (
                a["proof_semantic_hashes"] == b["proof_semantic_hashes"] if comparable else None
            ),
            "baseline_deterministic": a["deterministic"],
            "candidate_deterministic": b["deterministic"],
            "dimensions": transitions,
            "resources": {phase: _compare_phase(a["runs"], b["runs"], phase) for phase in ("cold", "warm")},
        })
    return {
        "schema_version": 1,
        "corpus": left["corpus"],
        "manifest_sha256": left["manifest_sha256"],
        "baseline_scanner": left["scanner"],
        "candidate_scanner": right["scanner"],
        "baseline_signal_coverage": baseline["summary"].get("signals"),
        "candidate_signal_coverage": candidate["summary"].get("signals"),
        "baseline_false_certainty": baseline["summary"]["false_certainty_cases"],
        "candidate_false_certainty": candidate["summary"]["false_certainty_cases"],
        "case_count": len(rows),
        "semantic_equal": sum(row["semantic_equal"] is True for row in rows),
        "semantic_changed": sum(row["semantic_equal"] is False for row in rows),
        "semantic_unavailable": sum(row["semantic_equal"] is None for row in rows),
        "regressions": [
            {"id": row["id"], "dimension": name, **transition}
            for row in rows for name, transition in row["dimensions"].items()
            if transition["baseline"] == "pass" and transition["candidate"] != "pass"
        ],
        "improvements": [
            {"id": row["id"], "dimension": name, **transition}
            for row in rows for name, transition in row["dimensions"].items()
            if transition["baseline"] == "fail" and transition["candidate"] == "pass"
        ],
        "cases": rows,
        "limitations": [
            "Controlled fixtures; not independent production precision or recall.",
            "Timing is descriptive: host, load, and build conditions must be matched externally.",
            "Semantic equality covers the canonical proof projection, not all review fields.",
        ],
    }


def _compare_phase(left: list, right: list, phase: str) -> dict:
    a = [run for run in left if run["phase"] == phase]
    b = [run for run in right if run["phase"] == phase]
    old = median(run["wall_ms"] for run in a)
    new = median(run["wall_ms"] for run in b)
    def rss(runs):
        samples = [run["resource"]["peak_rss_kb"] for run in runs if run["resource"]["status"] == "available"]
        return max(samples) if len(samples) == len(runs) else None
    return {
        "baseline_wall_ms_median": old,
        "candidate_wall_ms_median": new,
        "wall_ms_delta_pct": round((new - old) / old * 100, 3) if old > 0 else None,
        "baseline_peak_rss_kb": rss(a),
        "candidate_peak_rss_kb": rss(b),
        "samples_per_binary": len(a),
    }
