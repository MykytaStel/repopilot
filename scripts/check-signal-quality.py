
#!/usr/bin/env python3
"""RepoPilot signal quality checker.

This is a product-level guard, not a per-release helper.

Examples:
    cargo run -- scan . --format json --output /tmp/repopilot-self-scan.json
    python3 scripts/check-signal-quality.py --scan-json /tmp/repopilot-self-scan.json
    python3 scripts/check-signal-quality.py --scan-json /tmp/repopilot-self-scan.json --warn-only
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

NOISY_RULE_IDS = {
    "code-quality.long-function",
    "code-quality.complex-file",
    "testing.source-without-test",
    "testing.missing-test-structure",
    "code-marker.todo",
    "code-marker.fixme",
    "code-marker.hack",
}

NOISY_RULE_PREFIXES = (
    "testing.",
)

DEFAULT_MAX_CONTRACT_VIOLATIONS = 0
DEFAULT_MAX_VISIBLE_NOISY = 0


def load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise SystemExit(f"scan report not found: {path}")
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid scan report JSON: {exc}") from exc

    if not isinstance(data, dict):
        raise SystemExit("scan report must be a JSON object")
    return data


def get_findings(report: dict[str, Any]) -> list[dict[str, Any]]:
    direct = report.get("findings")
    if isinstance(direct, list):
        return [item for item in direct if isinstance(item, dict)]

    nested = report.get("report")
    if isinstance(nested, dict) and isinstance(nested.get("findings"), list):
        return [item for item in nested["findings"] if isinstance(item, dict)]

    return []


def get_rule_id(finding: dict[str, Any]) -> str:
    for key in ("rule_id", "ruleId", "rule", "id"):
        value = finding.get(key)
        if isinstance(value, str) and value:
            return value
    return "unknown"


def get_priority(finding: dict[str, Any]) -> str:
    for key in ("priority", "risk_priority", "riskPriority"):
        value = finding.get(key)
        if isinstance(value, str) and value:
            return value.lower()

    risk = finding.get("risk")
    if isinstance(risk, dict):
        value = risk.get("priority")
        if isinstance(value, str) and value:
            return value.lower()

    return "unknown"


def is_visible(finding: dict[str, Any]) -> bool:
    for key in ("visible", "is_visible", "default_visible", "defaultVisible"):
        value = finding.get(key)
        if isinstance(value, bool):
            return value

    visibility = finding.get("visibility")
    if isinstance(visibility, str):
        return visibility.lower() in {
            "visible",
            "shown",
            "default",
            "default-visible",
            "default_visible",
        }

    hidden = finding.get("hidden")
    if isinstance(hidden, bool):
        return not hidden

    # Older reports usually only serialized visible findings.
    return True


def is_noisy_rule(rule_id: str) -> bool:
    return rule_id in NOISY_RULE_IDS or rule_id.startswith(NOISY_RULE_PREFIXES)


def get_contract_violations(report: dict[str, Any]) -> int:
    signal_quality = report.get("signal_quality") or report.get("signalQuality")
    if isinstance(signal_quality, dict):
        for key in (
            "contract_violations",
            "contractViolations",
            "contract_violation_count",
            "contractViolationCount",
        ):
            value = signal_quality.get(key)
            if isinstance(value, int):
                return value

    diagnostics = report.get("diagnostics")
    if isinstance(diagnostics, list):
        return sum(
            1
            for item in diagnostics
            if isinstance(item, dict)
            and "contract" in str(item.get("kind") or item.get("code") or item.get("message") or "").lower()
        )

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check RepoPilot scan signal quality.")
    parser.add_argument("--scan-json", required=True, type=Path)
    parser.add_argument("--warn-only", action="store_true")
    parser.add_argument("--max-contract-violations", type=int, default=DEFAULT_MAX_CONTRACT_VIOLATIONS)
    parser.add_argument("--max-visible-noisy", type=int, default=DEFAULT_MAX_VISIBLE_NOISY)
    parser.add_argument("--output-json", type=Path)
    args = parser.parse_args()

    report = load_json(args.scan_json)
    findings = get_findings(report)
    visible_findings = [finding for finding in findings if is_visible(finding)]
    noisy_visible = [finding for finding in visible_findings if is_noisy_rule(get_rule_id(finding))]
    contract_violations = get_contract_violations(report)

    visible_by_rule = Counter(get_rule_id(finding) for finding in visible_findings)
    visible_by_priority = Counter(get_priority(finding) for finding in visible_findings)
    noisy_by_rule = Counter(get_rule_id(finding) for finding in noisy_visible)

    failures: list[str] = []
    if contract_violations > args.max_contract_violations:
        failures.append(
            f"contract violations {contract_violations} > {args.max_contract_violations}"
        )
    if len(noisy_visible) > args.max_visible_noisy:
        failures.append(
            f"visible noisy findings {len(noisy_visible)} > {args.max_visible_noisy}"
        )

    summary = {
        "scan_json": str(args.scan_json),
        "findings_total": len(findings),
        "findings_visible": len(visible_findings),
        "visible_by_priority": dict(sorted(visible_by_priority.items())),
        "top_visible_rules": dict(visible_by_rule.most_common(15)),
        "contract_violations": contract_violations,
        "visible_noisy_findings": len(noisy_visible),
        "visible_noisy_by_rule": dict(sorted(noisy_by_rule.items())),
        "status": "failed" if failures else "passed",
        "failures": failures,
        "mode": "warn-only" if args.warn_only else "gate",
    }

    rendered = json.dumps(summary, indent=2, sort_keys=True)
    print(rendered)
    if args.output_json:
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(rendered + "\n", encoding="utf-8")

    if failures and not args.warn_only:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
