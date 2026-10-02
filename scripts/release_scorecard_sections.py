"""Sections of the v0.24 release scorecard, each rendered from one committed source."""

from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

CLOSED = ("verified", "accepted")


def ledger_lines(path: Path) -> list[str]:
    """Row counts by status, the P0/P1 exit criterion, and every row still open."""
    rows = []
    for line in path.read_text().splitlines():
        if not re.match(r"\| RP24-\d{3} \|", line):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        rows.append({"id": cells[0], "class": cells[1], "status": cells[2], "phase": cells[3], "text": cells[4]})
    counts = {status: sum(1 for row in rows if row["status"] == status) for status in ("verified", "accepted", "in-progress", "open")}
    blocking = [row for row in rows if row["class"].split()[0] in ("P0", "P1") and row["status"] not in CLOSED]
    lines = [
        f"Source: [v0.24 evidence ledger]({path.name}), {len(rows)} rows.",
        "",
        "| Verified | Accepted | In progress | Open |",
        "|---:|---:|---:|---:|",
        f"| {counts['verified']} | {counts['accepted']} | {counts['in-progress']} | {counts['open']} |",
        "",
        f"Exit criterion, no open P0/P1 row: **{'met' if not blocking else 'not met'}**"
        + ("." if not blocking else f" ({', '.join(row['id'] for row in blocking)})."),
    ]
    still_open = [row for row in rows if row["status"] not in CLOSED]
    if still_open:
        lines += ["", "| Row | Class | Phase | Status | Subject |", "|---|---|---|---|---|"]
        for row in still_open:
            subject = row["text"].split(". ")[0]
            if len(subject) > 140:
                subject = subject[:140].rsplit(" ", 1)[0] + " …"
            lines.append(f"| {row['id']} | {row['class']} | {row['phase']} | {row['status']} | {subject} |")
    return lines


def calibration_lines(path: Path) -> list[str]:
    """The risk-v4 before/after priority table, copied from the calibration record."""
    text = path.read_text()
    result = text[text.index("## Result") :]
    table = []
    for line in result.splitlines()[1:]:
        if line.startswith("|"):
            table.append(line)
        elif table:
            break
    return [
        f"Source: [risk-v4 calibration]({path.name}). Finding occurrences by severity and",
        "priority across the 28-entry calibration corpus, before (`risk-v3`) and after (`risk-v4`).",
        "",
        *table,
    ]


def fixture_lines(directory: Path) -> list[str]:
    """Every integrity golden fixture with the signals it expects and forbids."""
    lines = [
        "Source: `tests/fixtures/review/integrity/*/expected.json`, run by `tests/review_golden_fixtures.rs`.",
        "",
        "| Fixture | Expectation | Must report | Must not report |",
        "|---|---|---|---|",
    ]
    for fixture in sorted(path for path in directory.iterdir() if path.is_dir()):
        expected = json.loads((fixture / "expected.json").read_text())
        must = ", ".join(f"`{item['kind']}` ({item.get('bucket', 'any')})" for item in expected.get("expect", []) if "kind" in item) or "—"
        forbid = ", ".join(f"`{item.get('kind') or item.get('family')}`" for item in expected.get("forbid", [])) or "—"
        label = expected.get("label", "").replace("_", " ")
        lines.append(f"| `{fixture.name}` | {label} | {must} | {forbid} |")
    return lines


def zoo_lines(zoo: Path) -> list[str]:
    """Pinned repositories with their committed default-visible scan totals."""
    manifest = tomllib.loads((zoo / "manifest.toml").read_text()).get("repo", [])
    lines = [
        "Source: `tests/zoo/manifest.toml` and `tests/zoo/snapshots/*.json`. `python3 scripts/zoo.py scan`",
        "must reproduce these snapshots; it matched all of them on the v0.24 integrity code (RP24-017).",
        "",
        "| Repository | Language | Pinned commit | Default-visible findings |",
        "|---|---|---|---:|",
    ]
    total = 0
    for repo in manifest:
        snapshot = json.loads((zoo / "snapshots" / f"{repo['name']}.json").read_text())
        visible = snapshot["default"]["visible_total"]
        total += visible
        lines.append(f"| {repo['name']} | {repo['language']} | `{repo['sha'][:12]}` | {visible} |")
    lines.append(f"| **Total** | | | **{total}** |")
    return lines


def compatibility_lines(path: Path) -> list[str]:
    """Released producers read by the current reader, and released readers of schema 0.26."""
    manifest = json.loads(path.read_text())
    lines = [
        "Source: `tests/fixtures/reports/releases/manifest.json`, checked by",
        "`tests/released_report_compatibility.rs`; details in the",
        "[v0.24 compatibility matrix](v0.24-compatibility-matrix.md).",
        "",
        "| Released producer | Tag commit | Report schema | Current reader |",
        "|---|---|---:|---|",
    ]
    for producer in manifest["producers"]:
        lines.append(f"| {producer['package_version']} | `{producer['tag_commit'][:8]}` | {producer['report_schema']} | accepts |")
    lines += ["", "| Released reader | Input schema | Outcome |", "|---|---:|---|"]
    for reader in manifest["readers"]:
        lines.append(f"| {reader['package_version']} | {reader['input_schema']} | {reader['outcome']} |")
    return lines


def _number(pattern: str, text: str) -> float:
    match = re.search(pattern, text)
    if not match:
        raise ValueError(f"budget not found: {pattern}")
    return float(match.group(1).replace("_", ""))


def performance_lines(scripts: Path, record: dict | None) -> list[str]:
    """Budgets read from the gate scripts, and the last recorded measurement."""
    gate = (scripts / "check-review-performance.js").read_text()
    large = (scripts / "review-performance-large-diff.js").read_text()
    integrity = (scripts / "review-performance-integrity.js").read_text()
    budgets = {
        "ratio": _number(r"ratio > ([\d.]+)", gate),
        "finalization": _number(r"reportFinalizationUs > ([\d_]+)", gate),
        "large": _number(r"budgetMs: ([\d_]+)", large),
        "integrity": _number(r"budgetMs: ([\d_]+)", integrity),
        "share": _number(r"maxIntegrityShare: ([\d.]+)", integrity),
    }
    measured = (record or {}).get("results", {})
    large_run = measured.get("large_diff_review", {})
    integrity_run = measured.get("integrity_review", {})

    def cell(value, unit=""):
        return "—" if value is None else f"{value}{unit}"

    share_value = integrity_run.get("integrity_share")
    share = "—" if share_value is None else f"{share_value:.2%}"
    lines = [
        "Budgets come from the gate scripts run by `npm run review:performance` in CI and in",
        "`scripts/verify-release.sh`.",
        "",
        "| Gate | Budget | Recorded |",
        "|---|---|---|",
        f"| Changed vs full review median | ratio ≤ {budgets['ratio']} | {cell(measured.get('changed_to_full_ratio'))} |",
        f"| Report finalization | ≤ {budgets['finalization']:.0f} µs | {cell(measured.get('report_finalization_us'), ' µs')} |",
        f"| 123-file real review median | ≤ {budgets['large']:.0f} ms | {cell(large_run.get('wall_median_ms'), ' ms')} |",
        f"| 20-file integrity review median | ≤ {budgets['integrity']:.0f} ms | {cell(integrity_run.get('wall_median_ms'), ' ms')} |",
        f"| Integrity share of that median | ≤ {budgets['share']:.0%} | {share} |",
        "",
    ]
    if record:
        verdict = "passed" if record["gate_passed"] else "failed"
        lines.append(f"Recorded on {record['measured_on']} ({record['platform']}); the gate {verdict} on that run.")
    else:
        lines.append("No measurement recorded yet; run `python3 scripts/release_scorecard.py --measure`.")
    return lines


def publication_lines(record: dict | None) -> list[str]:
    """Channel-by-channel publication proof, recorded after the tag."""
    if not record:
        return [
            "Pending until the `v0.24.0` tag. After publishing, `scripts/verify-publication.sh`",
            "checks every channel by digest; record its result in `docs/engineering/v0.24-publication.json`",
            "and regenerate this page.",
        ]
    lines = [f"Verified on {record['verified_on']} for `{record['tag']}`.", "", "| Channel | Result |", "|---|---|"]
    lines += [f"| {channel} | {result} |" for channel, result in record["channels"].items()]
    return lines
