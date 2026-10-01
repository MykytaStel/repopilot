"""Prevalence report for the integrity corpus (Wilson 95% intervals)."""

from __future__ import annotations

import math

KINDS = (
    "skip-added",
    "focus-added",
    "test-removed",
    "test-substituted",
    "assertion-removed",
    "assertion-trivialized",
    "expectation-rewritten",
    "suppression-added",
    "gate-relaxed",
)
VERDICTS = ("weakened", "justified", "none")
# Which labeled kinds each RepoPilot signal is meant to catch.
SIGNAL_KINDS = {
    "integrity.test-focused": ("focus-added",),
    "integrity.test-skipped": ("skip-added",),
    "integrity.test-removed": ("test-removed", "test-substituted"),
    "integrity.assertions-removed": ("assertion-removed", "assertion-trivialized"),
    "integrity.suppression-added": ("suppression-added",),
}


def wilson(hits: int, n: int, z: float = 1.96) -> tuple[float, float]:
    if n == 0:
        return (0.0, 0.0)
    p = hits / n
    denom = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / denom
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / denom
    return (max(0.0, centre - half), min(1.0, centre + half))


def fmt(hits: int, n: int) -> str:
    lo, hi = wilson(hits, n)
    pct = 100 * hits / n if n else 0.0
    return f"{hits}/{n} ({pct:.1f}%, 95% CI {100 * lo:.1f}–{100 * hi:.1f}%)"


def render(manifest: list[dict], labels: dict, results: dict[str, dict] | None = None) -> str:
    by_id = {entry["id"]: entry for entry in labels.get("label", [])}
    meta = labels.get("meta", {})
    lines = [
        "# Integrity corpus — prevalence",
        "",
        f"Labeler: {meta.get('labeler', 'unrecorded')}. Status: {meta.get('status', 'exploratory')}.",
        "Denominator: labeled merged PRs that modify or remove at least one test file.",
        "",
        "| Kind | agent | human |",
        "|---|---|---|",
    ]
    groups = {g: [e for e in manifest if e["group"] == g and e["id"] in by_id] for g in ("agent", "human")}
    for kind in KINDS:
        cells = []
        for group in ("agent", "human"):
            entries = groups[group]
            hits = sum(1 for e in entries if kind in by_id[e["id"]].get("kinds", []))
            cells.append(fmt(hits, len(entries)))
        lines.append(f"| `{kind}` | {cells[0]} | {cells[1]} |")
    lines += ["", "| Verdict | agent | human |", "|---|---|---|"]
    for verdict in VERDICTS:
        cells = []
        for group in ("agent", "human"):
            entries = groups[group]
            hits = sum(1 for e in entries if by_id[e["id"]].get("verdict") == verdict)
            cells.append(fmt(hits, len(entries)))
        lines.append(f"| {verdict} | {cells[0]} | {cells[1]} |")
    unlabeled = [e["id"] for e in manifest if e["id"] not in by_id]
    lines += ["", f"Unlabeled: {len(unlabeled)} of {len(manifest)}."]
    if results:
        lines += catches(manifest, by_id, results)
    return "\n".join(lines)


def catches(manifest: list[dict], by_id: dict, results: dict[str, dict]) -> list[str]:
    """PR-level agreement between labels and RepoPilot signals."""
    scored = [e for e in manifest if e["id"] in by_id and e["id"] in results]
    binaries = sorted({results[e["id"]].get("binary", "?") for e in scored})
    lines = [
        "",
        "## RepoPilot catches",
        "",
        f"PRs with both labels and results: {len(scored)}. Binary: {', '.join(binaries)}.",
        "A PR counts once per signal kind, whatever the number of occurrences.",
        "",
        "| Signal | caught | missed | false alarm | precision | recall |",
        "|---|---|---|---|---|---|",
    ]
    for signal, kinds in SIGNAL_KINDS.items():
        tp = fn = fp = 0
        for entry in scored:
            labeled = any(kind in by_id[entry["id"]].get("kinds", []) for kind in kinds)
            fired = any(s["kind"] == signal for s in results[entry["id"]]["signals"])
            tp += labeled and fired
            fn += labeled and not fired
            fp += fired and not labeled
        precision = fmt(tp, tp + fp) if tp + fp else "n/a"
        recall = fmt(tp, tp + fn) if tp + fn else "n/a"
        lines.append(f"| `{signal}` | {tp} | {fn} | {fp} | {precision} | {recall} |")
    return lines
