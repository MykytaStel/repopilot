from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from changeproof_benchmark_artifact import (  # noqa: E402
    render_report,
    validate_artifact_file,
)
from changeproof_benchmark_proof import (  # noqa: E402
    evaluate_normalized_proof,
    normalized_proof_sha256,
)
from changeproof_benchmark_contract import BenchmarkCase, BenchmarkManifestError, fixture_source_sha256  # noqa: E402


def write_fixture(root: Path) -> None:
    before = root / "sample" / "safe" / "before"
    after = root / "sample" / "safe" / "after"
    rationale = root / "sample" / "safe" / "patch"
    before.mkdir(parents=True)
    after.mkdir()
    rationale.mkdir(parents=True)
    (before / "app.py").write_text("value = 1\n", encoding="utf-8")
    (after / "app.py").write_text("value = 2\n", encoding="utf-8")
    (rationale / "rationale.md").write_text("A bounded fixture oracle.", encoding="utf-8")


def write_manifest(root: Path, claims_state: str = "unknown", expected_verdict: str = "REVIEW") -> Path:
    manifest = root / "changeproof.toml"
    manifest.write_text(
        f'''schema_version = 1
corpus = "artifact-test"
protocol = "changeproof-benchmark-v1"

[[case]]
id = "sample"
fixture = "sample"
variant = "safe"
split = "evaluation"
profile = "default"
mutation_kind = "negative-control"
expected_verdict = "{expected_verdict}"
claims_state = "{claims_state}"
reason_codes_state = "unknown"
contracts_state = "unknown"
capabilities_state = "unknown"
oracle = "mutation-rationale-v1"
oracle_state = "known"
coverage_state = "unknown"
obligations_state = "unknown"
''',
        encoding="utf-8",
    )
    return manifest


def normalized_proof(verdict: str = "REVIEW") -> dict[str, object]:
    return {
        "verdict": verdict,
        "reasons": [{"code": "maybe-sensitive", "count": 1}],
        "coverage": {"scope": "changed", "requested_files": 1, "analyzed_files": 1, "excluded_files": 0, "unsupported_files": 0},
        "obligations": {"applicable": 0, "satisfied": 0, "failed": 0, "unavailable": 0, "unselected": 0, "stale": 0},
        "contracts": [],
        "capabilities": [],
    }


def benchmark_case(claims_state: str = "unknown", expected_verdict: str = "REVIEW") -> BenchmarkCase:
    return BenchmarkCase(
        case_id="sample", fixture="sample", variant="safe", split="evaluation", profile="default",
        mutation_kind="negative-control", expected_verdict=expected_verdict, claims_state=claims_state,
        claims=() if claims_state == "known-empty" else None, reason_codes_state="unknown", reason_codes=None,
        contracts_state="unknown", contracts=None, capabilities_state="unknown", capabilities=None,
        coverage_state="unknown", coverage=None, obligations_state="unknown", obligations=None,
        oracle="mutation-rationale-v1", oracle_state="known",
    )


def write_artifact(root: Path, manifest: Path, claims_state: str = "unknown", expected_verdict: str = "REVIEW", observed_verdict: str | None = None) -> Path:
    proof = normalized_proof(observed_verdict or expected_verdict)
    evaluation = evaluate_normalized_proof(benchmark_case(claims_state, expected_verdict), proof)
    run = {
        "phase": "cold", "status": "collected", "returncode": 0, "wall_ms": 1.0,
        "resource": {"status": "available", "peak_rss_kb": 10, "source": "posix-time-v1"},
        "proof": proof, "proof_semantic_sha256": normalized_proof_sha256(proof), "evaluation": evaluation,
    }
    artifact = {
        "schema_version": 1, "protocol": "changeproof-benchmark-v1", "corpus": "artifact-test",
        "manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
        "scanner": {"mode": "explicit", "version": "0.22.0", "report_schema_version": "0.26", "workspace_version": "0.22.0", "workspace_commit": None, "workspace_dirty": None, "version_mismatch_allowed": False},
        "cases": [{
            "id": "sample", "fixture": "sample", "variant": "safe",
            "fixture_source_sha256": fixture_source_sha256(benchmark_case(claims_state, expected_verdict), root / "review-zoo"),
            "oracle": {"status": "measured", "scheme": "mutation-rationale-v1", "sha256": hashlib.sha256((root / "review-zoo/sample/safe/patch/rationale.md").read_bytes()).hexdigest()},
            "runs": [dict(run, phase=phase) for phase in ("cold", "warm", "warm")],
            "proof_semantic_hashes": [normalized_proof_sha256(proof)], "deterministic": True,
        }],
    }
    path = root / "result.json"
    path.write_text(json.dumps(artifact), encoding="utf-8")
    return path


class ChangeProofBenchmarkArtifactTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.fixture_root = self.root / "review-zoo"
        write_fixture(self.fixture_root)
        self.manifest = write_manifest(self.root)
        self.artifact = write_artifact(self.root, self.manifest)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_validation_rejects_manifest_hash_drift(self) -> None:
        data = json.loads(self.artifact.read_text(encoding="utf-8"))
        data["manifest_sha256"] = "0" * 64
        self.artifact.write_text(json.dumps(data), encoding="utf-8")

        with self.assertRaisesRegex(BenchmarkManifestError, "manifest_sha256"):
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)

    def test_report_keeps_unknown_claim_metrics_unavailable(self) -> None:
        report = render_report(validate_artifact_file(self.artifact, self.manifest, self.fixture_root))

        self.assertIn("Claims | unavailable", report)

    def test_validation_rejects_tampered_proof_hash(self) -> None:
        data = json.loads(self.artifact.read_text(encoding="utf-8"))
        data["cases"][0]["runs"][0]["proof_semantic_sha256"] = "f" * 64
        self.artifact.write_text(json.dumps(data), encoding="utf-8")

        with self.assertRaisesRegex(BenchmarkManifestError, "proof_semantic_sha256"):
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)

    def test_observed_verified_against_review_is_reported_as_false_certainty(self) -> None:
        manifest = write_manifest(self.root, claims_state="unsupported", expected_verdict="REVIEW")
        artifact = write_artifact(self.root, manifest, claims_state="unsupported", expected_verdict="REVIEW", observed_verdict="VERIFIED")

        validated = validate_artifact_file(artifact, manifest, self.fixture_root)

        self.assertEqual(validated["summary"]["false_certainty_cases"], 1)

    def test_validation_rejects_raw_source_and_fixture_drift(self) -> None:
        data = json.loads(self.artifact.read_text(encoding="utf-8"))
        proof = data["cases"][0]["runs"][0]["proof"]
        proof["raw_source"] = "/outside/token"
        data["cases"][0]["runs"][0]["proof_semantic_sha256"] = hashlib.sha256(json.dumps(proof, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        self.artifact.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(BenchmarkManifestError, "proof is invalid"):
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)
        data = json.loads(write_artifact(self.root, self.manifest).read_text(encoding="utf-8"))
        (self.fixture_root / "sample/safe/after/app.py").write_text("value = 3\n", encoding="utf-8")
        self.artifact.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(BenchmarkManifestError, "fixture source identity"):
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)

    def test_validation_rejects_bad_provenance_and_reports_transient_overclaim(self) -> None:
        data = json.loads(self.artifact.read_text(encoding="utf-8"))
        data["scanner"]["workspace_dirty"] = []
        with self.assertRaisesRegex(BenchmarkManifestError, "workspace_dirty"):
            self.artifact.write_text(json.dumps(data), encoding="utf-8")
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)
        data = json.loads(write_artifact(self.root, self.manifest).read_text(encoding="utf-8"))
        data["cases"][0]["runs"][1]["evaluation"]["decision"]["observed"] = "VERIFIED"
        data["cases"][0]["runs"][1]["proof"]["verdict"] = "VERIFIED"
        data["cases"][0]["runs"][1]["proof_semantic_sha256"] = normalized_proof_sha256(data["cases"][0]["runs"][1]["proof"])
        data["cases"][0]["runs"][1]["evaluation"] = evaluate_normalized_proof(benchmark_case(), data["cases"][0]["runs"][1]["proof"])
        data["cases"][0]["proof_semantic_hashes"] = sorted({run["proof_semantic_sha256"] for run in data["cases"][0]["runs"]})
        data["cases"][0]["deterministic"] = False
        self.artifact.write_text(json.dumps(data), encoding="utf-8")
        self.assertEqual(validate_artifact_file(self.artifact, self.manifest, self.fixture_root)["summary"]["false_certainty_cases"], 1)

    def test_validation_accepts_producer_unavailable_resource_and_rejects_scalar_injection(self) -> None:
        data = json.loads(self.artifact.read_text(encoding="utf-8"))
        for run in data["cases"][0]["runs"]:
            run["resource"] = {"status": "unavailable", "reason": "portable peak RSS sampler is unavailable on this platform", "source": "unavailable"}
        self.artifact.write_text(json.dumps(data), encoding="utf-8")
        validate_artifact_file(self.artifact, self.manifest, self.fixture_root)
        data["scanner"]["report_schema_version"] = 'const SECRET = "x";\n/private'
        self.artifact.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(BenchmarkManifestError, "report_schema_version"):
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)

    def test_validation_rejects_boolean_in_evaluation_scalar(self) -> None:
        data = json.loads(self.artifact.read_text(encoding="utf-8"))
        data["cases"][0]["runs"][0]["evaluation"]["decision"]["observed"] = True
        self.artifact.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(BenchmarkManifestError, "evaluation does not match"):
            validate_artifact_file(self.artifact, self.manifest, self.fixture_root)


if __name__ == "__main__":
    unittest.main()
