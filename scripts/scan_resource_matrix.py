#!/usr/bin/env python3
"""Run the synthetic full/changed scan RSS matrix."""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import subprocess
import tempfile
import tomllib
from pathlib import Path
from typing import Any

from differential_resource import execute_timed_command


SCENARIOS = ("full_cold", "full_warm", "changed_cold", "changed_warm")


class ScanResourcePolicyError(ValueError):
    """Raised when the scan resource policy or matrix is invalid."""


def _positive(value: object, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or value <= 0:
        raise ScanResourcePolicyError(f"{label} must be positive")
    return float(value)


def _validate_policy(policy: object) -> dict[str, Any]:
    if not isinstance(policy, dict) or policy.get("schema_version") != 1:
        raise ScanResourcePolicyError("scan resource policy schema_version must be 1")
    for key in ("policy_id", "workload", "source", "unit"):
        if not isinstance(policy.get(key), str) or not policy[key].strip():
            raise ScanResourcePolicyError(f"scan resource policy {key} must be a non-empty string")
    if policy["source"] != "posix-time-v1" or policy["unit"] != "KiB":
        raise ScanResourcePolicyError("scan resource policy must use posix-time-v1 KiB samples")
    if policy.get("unavailable") != "fail":
        raise ScanResourcePolicyError('scan resource policy unavailable must be "fail"')
    scenarios = policy.get("required_scenarios")
    if not isinstance(scenarios, list) or set(scenarios) != set(SCENARIOS) or len(scenarios) != len(SCENARIOS):
        raise ScanResourcePolicyError("scan resource policy must list all four matrix scenarios")
    ceilings = policy.get("ceiling_kb_by_scenario")
    if not isinstance(ceilings, dict) or set(ceilings) != set(SCENARIOS):
        raise ScanResourcePolicyError("scan resource policy must define four scenario ceilings")
    normalized = dict(policy)
    normalized["required_scenarios"] = list(SCENARIOS)
    normalized["ceiling_kb_by_scenario"] = {
        scenario: _positive(ceilings[scenario], f"{scenario} ceiling") for scenario in SCENARIOS
    }
    return normalized


def load_scan_resource_policy(path: Path) -> dict[str, Any]:
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise ScanResourcePolicyError(f"cannot read scan resource policy {path}: {error}") from error
    return _validate_policy(raw)


def _violation(kind: str, scenario: str, message: str, **values: object) -> dict[str, object]:
    item: dict[str, object] = {"scenario": scenario, "kind": kind, "message": message}
    item.update(values)
    return item


def evaluate_matrix(samples: dict[str, list[dict[str, object]]], policy: dict[str, Any]) -> dict[str, object]:
    normalized = _validate_policy(policy)
    if set(samples) != set(SCENARIOS):
        raise ScanResourcePolicyError("scenario set mismatch")
    violations: list[dict[str, object]] = []
    summaries: dict[str, dict[str, object]] = {}
    for scenario in SCENARIOS:
        values: list[float] = []
        for sample in samples[scenario]:
            if sample.get("resource_status") != "available":
                kind = "unavailable" if sample.get("resource_status") == "unavailable" else "invalid_status"
                violations.append(
                    _violation(kind, scenario, str(sample.get("resource_reason", "RSS sample unavailable")))
                )
                continue
            if sample.get("resource_source") != normalized["source"]:
                violations.append(_violation("source_mismatch", scenario, "RSS source does not match policy"))
                continue
            value = sample.get("child_max_rss_kb")
            if isinstance(value, bool) or not isinstance(value, (int, float)) or value <= 0:
                violations.append(_violation("invalid_sample", scenario, "RSS sample must be positive"))
                continue
            values.append(float(value))
        ceiling = normalized["ceiling_kb_by_scenario"][scenario]
        if not values:
            violations.append(_violation("missing_scenario", scenario, "no valid RSS samples"))
            summaries[scenario] = {
                "status": "fail", "samples": 0, "median_kb": None, "max_kb": None, "ceiling_kb": ceiling
            }
            continue
        median = round(statistics.median(values), 3)
        maximum = round(max(values), 3)
        summaries[scenario] = {
            "status": "pass", "samples": len(values), "median_kb": median, "max_kb": maximum, "ceiling_kb": ceiling
        }
        if median > ceiling:
            violations.append(_violation("median_over_ceiling", scenario, "median exceeds ceiling", observed_kb=median, ceiling_kb=ceiling))
        if maximum > ceiling:
            violations.append(_violation("max_over_ceiling", scenario, "maximum exceeds ceiling", observed_kb=maximum, ceiling_kb=ceiling))
    violations.sort(key=lambda item: (str(item["scenario"]), str(item["kind"]), str(item["message"])))
    return {
        "status": "fail" if violations else "pass",
        "policy_id": normalized["policy_id"],
        "workload": normalized["workload"],
        "source": normalized["source"],
        "unit": normalized["unit"],
        "scenarios": summaries,
        "violations": violations,
    }


def render_matrix(result: dict[str, object]) -> str:
    lines = [f"Scan resource matrix: {result['status']}", f"Policy: {result['policy_id']} ({result['workload']})"]
    for scenario in SCENARIOS:
        summary = result["scenarios"][scenario]
        lines.append(
            f"{scenario}: median={summary['median_kb']} KiB; max={summary['max_kb']} KiB; "
            f"ceiling={summary['ceiling_kb']} KiB; samples={summary['samples']}"
        )
    for item in result["violations"]:
        lines.append(f"violation: {item['scenario']} {item['kind']}: {item['message']}")
    return "\n".join(lines) + "\n"


def _write_file(root: Path, relative: str, contents: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")


def _build_corpus(root: Path, files_per_language: int) -> None:
    _write_file(root, "go.mod", "module benchmod\n\ngo 1.22\n")
    for index in range(files_per_language):
        previous = max(0, index - 1)
        _write_file(root, f"src/file_{index}.rs", f"use crate::file_{previous};\n\npub fn compute_{index}(input: i64) -> i64 {{\n    (0..input).sum()\n}}\n")
        _write_file(root, f"web/file_{index}.ts", f'import {{ compute }} from "./file_{previous}";\nexport function run_{index}(input: number): number {{ return input + compute; }}\n')
        _write_file(root, f"py/file_{index}.py", f"from .file_{previous} import compute\n\ndef run_{index}(value):\n    return value + compute\n")
        _write_file(root, f"gopkg/file_{index}.go", f"package gopkg\n\nfunc Run{index}(value int) int {{ return value }}\n")


def _git_baseline(root: Path) -> None:
    for args in (("git", "init", "-q"), ("git", "config", "user.email", "benchmark@repopilot.local"), ("git", "config", "user.name", "RepoPilot Benchmark"), ("git", "add", "."), ("git", "commit", "-qm", "benchmark baseline")):
        subprocess.run(args, cwd=root, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)


def _scan(scanner: Path, root: Path, output: Path, changed: bool) -> dict[str, object]:
    output.parent.mkdir(parents=True, exist_ok=True)
    args = [str(scanner), "scan", ".", "--format", "json", "--output", str(output), "--no-progress"]
    if changed:
        args.append("--changed")
    result = execute_timed_command(tuple(args), root, 300)
    if result["status"] != "passed":
        raise RuntimeError(f"scan command failed: {result.get('resource_reason', result['status'])}")
    resource = result["resource"]
    sample = {
        "resource_status": resource.get("status"),
        "resource_source": resource.get("source"),
        "child_max_rss_kb": resource.get("peak_rss_kb"),
        "wall_ms": result["wall_ms"],
    }
    if resource.get("reason"):
        sample["resource_reason"] = resource["reason"]
    return sample


def collect_matrix(scanner: Path, files_per_language: int) -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="repopilot-scan-resource-") as tmp:
        root = Path(tmp)
        results = root / "results"
        full_root = root / "full"
        changed_root = root / "changed"
        _build_corpus(full_root, files_per_language)
        full_samples = {"full_cold": [_scan(scanner, full_root, results / "full-cold.json", False)], "full_warm": []}
        for index in range(3):
            full_samples["full_warm"].append(_scan(scanner, full_root, results / f"full-warm-{index}.json", False))
        _build_corpus(changed_root, files_per_language)
        _git_baseline(changed_root)
        with (changed_root / "src/file_0.rs").open("a", encoding="utf-8") as handle:
            handle.write("\n// benchmark edit\n")
        changed_samples = {"changed_cold": [_scan(scanner, changed_root, results / "changed-cold.json", True)], "changed_warm": []}
        for index in range(3):
            changed_samples["changed_warm"].append(_scan(scanner, changed_root, results / f"changed-warm-{index}.json", True))
        return {**full_samples, **changed_samples}


def run_matrix(scanner: Path, policy_path: Path, output: Path, files_per_language: int) -> dict[str, object]:
    policy = load_scan_resource_policy(policy_path)
    samples = collect_matrix(scanner, files_per_language)
    result = evaluate_matrix(samples, policy)
    result["host_profile"] = f"{platform.system().lower()}-{platform.machine().lower()}"
    result["files_per_language"] = files_per_language
    result["scanner"] = str(scanner)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scanner", type=Path, default=Path("target/release/repopilot"))
    parser.add_argument("--policy", type=Path, default=Path("tests/benchmarks/scan-resource.toml"))
    parser.add_argument("--output", type=Path, default=Path("/tmp/repopilot-scan-resource-matrix.json"))
    parser.add_argument("--files-per-language", type=int, default=250)
    args = parser.parse_args(argv)
    if args.files_per_language <= 0:
        parser.error("--files-per-language must be positive")
    try:
        result = run_matrix(args.scanner.resolve(), args.policy, args.output, args.files_per_language)
    except (OSError, RuntimeError, ScanResourcePolicyError) as error:
        print(f"scan resource matrix failed: {error}")
        return 1
    print(render_matrix(result), end="")
    return 0 if result["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
