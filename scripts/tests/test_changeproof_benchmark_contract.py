from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

from changeproof_benchmark_contract import (  # noqa: E402
    BenchmarkManifestError,
    validate_manifest,
)


def case(case_id: str, fixture: str, claims_state: str) -> str:
    claims = 'claims = []\n' if claims_state == "known-empty" else ""
    return f'''[[case]]
id = "{case_id}"
fixture = "{fixture}"
variant = "safe"
split = "evaluation"
profile = "default"
mutation_kind = "negative-control"
expected_verdict = "REVIEW"
claims_state = "{claims_state}"
reason_codes_state = "unknown"
contracts_state = "unknown"
capabilities_state = "unknown"
oracle = "mutation-rationale-v1"
oracle_state = "known"
coverage_state = "unknown"
obligations_state = "unknown"
{claims}'''


def write_manifest(root: Path, cases: list[str]) -> Path:
    manifest = root / "changeproof.toml"
    manifest.write_text(
        'schema_version = 1\ncorpus = "test-corpus"\nprotocol = "changeproof-benchmark-v1"\n\n'
        + "\n".join(cases),
        encoding="utf-8",
    )
    return manifest


class ChangeProofBenchmarkContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.fixture_root = self.root / "review-zoo"
        for fixture in ("safe-case", "unknown-case"):
            path = self.fixture_root / fixture / "safe" / "patch"
            path.mkdir(parents=True)
            (path / "rationale.md").write_text("Independent fixture rationale.", encoding="utf-8")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_manifest_rejects_case_path_escaping_fixture_root(self) -> None:
        manifest = write_manifest(self.root, [case("escape", "../outside", "unknown")])

        with self.assertRaisesRegex(BenchmarkManifestError, "relative descendant"):
            validate_manifest(manifest, self.fixture_root)

    def test_manifest_keeps_unknown_distinct_from_known_empty(self) -> None:
        manifest = write_manifest(
            self.root,
            [
                case("unknown-claims", "unknown-case", "unknown"),
                case("empty-claims", "safe-case", "known-empty"),
            ],
        )

        corpus, cases = validate_manifest(manifest, self.fixture_root)

        self.assertEqual(corpus, "test-corpus")
        self.assertEqual([case.case_id for case in cases], ["empty-claims", "unknown-claims"])
        self.assertEqual(cases[0].claims_state, "known-empty")
        self.assertEqual(cases[0].claims, ())
        self.assertEqual(cases[1].claims_state, "unknown")
        self.assertIsNone(cases[1].claims)

    def test_manifest_rejects_empty_claims_for_unknown_state(self) -> None:
        manifest = write_manifest(self.root, [case("unknown-claims", "unknown-case", "unknown")])
        manifest.write_text(
            manifest.read_text(encoding="utf-8").replace(
                'reason_codes_state = "unknown"',
                'claims = []\nreason_codes_state = "unknown"',
            ),
            encoding="utf-8",
        )

        with self.assertRaisesRegex(BenchmarkManifestError, "claims must be omitted when state is unknown"):
            validate_manifest(manifest, self.fixture_root)

    def test_manifest_rejects_unknown_case_fields(self) -> None:
        manifest = write_manifest(self.root, [case("safe-case", "safe-case", "known-empty")])
        manifest.write_text(
            manifest.read_text(encoding="utf-8").replace(
                'oracle = "mutation-rationale-v1"',
                'unexpected = "value"\noracle = "mutation-rationale-v1"',
            ),
            encoding="utf-8",
        )

        with self.assertRaisesRegex(BenchmarkManifestError, "unknown fields: unexpected"):
            validate_manifest(manifest, self.fixture_root)

    def test_manifest_rejects_absolute_fixture_and_nested_symlink(self) -> None:
        manifest = write_manifest(self.root, [case("safe-case", "/tmp/inside", "unknown")])
        with self.assertRaisesRegex(BenchmarkManifestError, "relative descendant"):
            validate_manifest(manifest, self.fixture_root)
        manifest = write_manifest(self.root, [case("safe-case", "safe-case", "unknown")])
        before = self.fixture_root / "safe-case" / "safe" / "before"
        before.mkdir()
        (before / "outside.py").symlink_to(self.root / "outside.py")
        with self.assertRaisesRegex(BenchmarkManifestError, "symlinks are not allowed"):
            validate_manifest(manifest, self.fixture_root)

    def test_manifest_rejects_symlinked_fixture_ancestor(self) -> None:
        real = self.fixture_root / "real-case" / "safe" / "patch"
        real.mkdir(parents=True)
        (real / "rationale.md").write_text("oracle", encoding="utf-8")
        (self.fixture_root / "alias-case").symlink_to(self.fixture_root / "real-case", target_is_directory=True)
        manifest = write_manifest(self.root, [case("alias-case", "alias-case", "unknown")])
        with self.assertRaisesRegex(BenchmarkManifestError, "path contains a symlink"):
            validate_manifest(manifest, self.fixture_root)

    def test_manifest_rejects_boolean_schema_version(self) -> None:
        manifest = write_manifest(self.root, [case("safe-case", "safe-case", "unknown")])
        manifest.write_text(manifest.read_text(encoding="utf-8").replace("schema_version = 1", "schema_version = true"), encoding="utf-8")
        with self.assertRaisesRegex(BenchmarkManifestError, "schema_version"):
            validate_manifest(manifest, self.fixture_root)


if __name__ == "__main__":
    unittest.main()
