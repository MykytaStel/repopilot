"""Strict committed-input contract for ChangeProof Benchmark v1."""

from __future__ import annotations

import hashlib
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path


SCHEMA_VERSION = 1
PROTOCOL = "changeproof-benchmark-v1"
CASE_ID = re.compile(r"^[a-z0-9][a-z0-9._-]+$")
FIXTURE_PATH = re.compile(r"^[a-zA-Z0-9][a-zA-Z0-9._/-]*$")
STATES = {"known", "known-empty", "unknown", "not-applicable", "unsupported", "unavailable"}
TOP_LEVEL_FIELDS = {"schema_version", "corpus", "protocol", "case"}
CASE_FIELDS = {
    "id", "fixture", "variant", "split", "profile", "mutation_kind", "expected_verdict",
    "claims_state", "claims", "reason_codes_state", "reason_codes", "contracts_state", "contracts",
    "capabilities_state", "capabilities", "coverage_state", "coverage", "obligations_state", "obligations",
    "oracle", "oracle_state",
}
VARIANTS = {"safe", "unsafe"}
SPLITS = {"evaluation"}
PROFILES = {"default", "strict"}
MUTATION_KINDS = {"negative-control", "authorization", "behavioral", "taint"}
VERDICTS = {"REVIEW", "VERIFIED"}
ORACLES = {"mutation-rationale-v1"}


class BenchmarkManifestError(ValueError):
    """Raised when benchmark corpus input is malformed or unsafe."""


@dataclass(frozen=True)
class BenchmarkCase:
    case_id: str
    fixture: str
    variant: str
    split: str
    profile: str
    mutation_kind: str
    expected_verdict: str
    claims_state: str
    claims: tuple[str, ...] | None
    reason_codes_state: str
    reason_codes: tuple[str, ...] | None
    contracts_state: str
    contracts: tuple[str, ...] | None
    capabilities_state: str
    capabilities: tuple[str, ...] | None
    coverage_state: str
    coverage: dict[str, object] | None
    obligations_state: str
    obligations: dict[str, int] | None
    oracle: str
    oracle_state: str


def validate_manifest(path: Path, fixture_root: Path) -> tuple[str, tuple[BenchmarkCase, ...]]:
    """Load a deterministic, lexically confined v1 benchmark corpus."""

    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        raise BenchmarkManifestError(f"cannot read manifest {path}: {error}") from error
    _validate_document(document)
    root = fixture_root.resolve()
    cases = tuple(_parse_case(raw, root, index) for index, raw in enumerate(document["case"], 1))
    _assert_unique_ids(cases)
    return document["corpus"].strip(), tuple(sorted(cases, key=lambda case: case.case_id))


def fixture_source_sha256(case: BenchmarkCase, fixture_root: Path) -> str:
    """Hash exactly the regular before/after files materialized into a temp repo."""

    root = _fixture_dir(case, fixture_root)
    digest = hashlib.sha256()
    for section in ("before", "after"):
        directory = root / section
        if section == "before" and not directory.is_dir():
            raise BenchmarkManifestError(f"case {case.case_id}: fixture before directory is missing")
        if directory.exists():
            _validate_tree(directory, case.case_id)
            for path in sorted(item for item in directory.rglob("*") if item.is_file()):
                relative = path.relative_to(root).as_posix()
                digest.update(relative.encode("utf-8") + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def rationale_sha256(case: BenchmarkCase, fixture_root: Path) -> str:
    """Read the independently committed rationale only for a known oracle."""

    rationale = _fixture_dir(case, fixture_root) / "patch" / "rationale.md"
    try:
        content = rationale.read_bytes()
    except OSError as error:
        raise BenchmarkManifestError(f"case {case.case_id}: oracle rationale is unavailable") from error
    if not content.strip():
        raise BenchmarkManifestError(f"case {case.case_id}: oracle rationale must not be empty")
    return hashlib.sha256(content).hexdigest()


def fixture_directory(case: BenchmarkCase, fixture_root: Path) -> Path:
    """Return a revalidated case directory for safe materialization."""

    return _fixture_dir(case, fixture_root)


def _validate_document(document: dict[str, object]) -> None:
    _reject_unknown(document, TOP_LEVEL_FIELDS, "top-level")
    if type(document.get("schema_version")) is not int or document["schema_version"] != SCHEMA_VERSION:
        raise BenchmarkManifestError(f"schema_version must be {SCHEMA_VERSION}")
    if document.get("protocol") != PROTOCOL:
        raise BenchmarkManifestError(f"protocol must be {PROTOCOL}")
    if not isinstance(document.get("corpus"), str) or not document["corpus"].strip():
        raise BenchmarkManifestError("corpus must be a non-empty string")
    if not isinstance(document.get("case"), list) or not document["case"]:
        raise BenchmarkManifestError("manifest must contain at least one [[case]]")


def _parse_case(raw: object, fixture_root: Path, index: int) -> BenchmarkCase:
    if not isinstance(raw, dict):
        raise BenchmarkManifestError(f"case {index}: entry must be a table")
    _reject_unknown(raw, CASE_FIELDS, f"case {index}")
    case_id = _string(raw, "id", index)
    if not CASE_ID.fullmatch(case_id):
        raise BenchmarkManifestError(f"case {index}: invalid id {case_id!r}")
    fixture = _relative_fixture(_string(raw, "fixture", index), case_id)
    variant = _enum(raw, "variant", index, VARIANTS)
    split = _enum(raw, "split", index, SPLITS)
    profile = _enum(raw, "profile", index, PROFILES)
    mutation_kind = _enum(raw, "mutation_kind", index, MUTATION_KINDS)
    expected_verdict = _enum(raw, "expected_verdict", index, VERDICTS)
    oracle = _enum(raw, "oracle", index, ORACLES)
    case = BenchmarkCase(
        case_id, fixture, variant, split, profile, mutation_kind, expected_verdict,
        _state(raw, "claims", case_id), _expectations(raw, "claims", case_id),
        _state(raw, "reason_codes", case_id), _expectations(raw, "reason_codes", case_id),
        _state(raw, "contracts", case_id), _expectations(raw, "contracts", case_id),
        _state(raw, "capabilities", case_id), _expectations(raw, "capabilities", case_id),
        _state(raw, "coverage", case_id), _mapping_expectation(raw, "coverage", case_id),
        _state(raw, "obligations", case_id), _mapping_expectation(raw, "obligations", case_id),
        oracle, _state(raw, "oracle", case_id),
    )
    _fixture_dir(case, fixture_root)
    return case


def _fixture_dir(case: BenchmarkCase, fixture_root: Path) -> Path:
    root = fixture_root.resolve()
    candidate = root
    for part in (*Path(case.fixture).parts, case.variant):
        candidate /= part
        if candidate.is_symlink():
            raise BenchmarkManifestError(f"case {case.case_id}: fixture path contains a symlink: {candidate.relative_to(root)}")
    if not candidate.is_dir():
        raise BenchmarkManifestError(f"case {case.case_id}: fixture directory missing or symlinked: {case.fixture}/{case.variant}")
    resolved = candidate.resolve()
    try:
        resolved.relative_to(root)
    except ValueError as error:
        raise BenchmarkManifestError(f"case {case.case_id}: fixture escapes fixture root") from error
    _validate_tree(resolved, case.case_id)
    return resolved


def _validate_tree(directory: Path, case_id: str) -> None:
    for path in directory.rglob("*"):
        if path.is_symlink():
            raise BenchmarkManifestError(f"case {case_id}: fixture symlinks are not allowed: {path.relative_to(directory)}")
        if ".git" in path.relative_to(directory).parts:
            raise BenchmarkManifestError(f"case {case_id}: fixture .git metadata is not allowed")
        if not path.is_dir() and not path.is_file():
            raise BenchmarkManifestError(f"case {case_id}: fixture entry is not a regular file: {path.relative_to(directory)}")


def _relative_fixture(value: str, case_id: str) -> str:
    path = Path(value)
    if path.is_absolute() or not FIXTURE_PATH.fullmatch(value) or any(part in {"", ".", ".."} for part in path.parts):
        raise BenchmarkManifestError(f"case {case_id}: fixture must be a relative descendant path")
    return path.as_posix()


def _reject_unknown(value: dict[str, object], allowed: set[str], context: str) -> None:
    unknown = set(value) - allowed
    if unknown:
        raise BenchmarkManifestError(f"{context}: unknown fields: {', '.join(sorted(unknown))}")


def _string(raw: dict[str, object], field: str, index: int) -> str:
    value = raw.get(field)
    if not isinstance(value, str) or not value.strip():
        raise BenchmarkManifestError(f"case {index}: {field} must be a non-empty string")
    return value.strip()


def _enum(raw: dict[str, object], field: str, index: int, choices: set[str]) -> str:
    value = _string(raw, field, index)
    if value not in choices:
        raise BenchmarkManifestError(f"case {index}: {field} must be one of {', '.join(sorted(choices))}")
    return value


def _state(raw: dict[str, object], prefix: str, case_id: str) -> str:
    value = raw.get(f"{prefix}_state")
    if not isinstance(value, str) or value not in STATES:
        raise BenchmarkManifestError(f"case {case_id}: {prefix}_state must be one of {', '.join(sorted(STATES))}")
    return value


def _expectations(raw: dict[str, object], prefix: str, case_id: str) -> tuple[str, ...] | None:
    state, value = _state(raw, prefix, case_id), raw.get(prefix)
    if state == "known":
        if not isinstance(value, list) or not value or not all(isinstance(item, str) and item for item in value):
            raise BenchmarkManifestError(f"case {case_id}: {prefix} must be a non-empty string array when known")
        if len(value) != len(set(value)):
            raise BenchmarkManifestError(f"case {case_id}: {prefix} cannot contain duplicates")
        return tuple(sorted(value))
    if state == "known-empty":
        if value != []:
            raise BenchmarkManifestError(f"case {case_id}: {prefix} must be [] when known-empty")
        return ()
    if value is not None:
        raise BenchmarkManifestError(f"case {case_id}: {prefix} must be omitted when state is {state}")
    return None


def _mapping_expectation(raw: dict[str, object], prefix: str, case_id: str) -> dict[str, object] | None:
    state, value = _state(raw, prefix, case_id), raw.get(prefix)
    if state == "known":
        if not isinstance(value, dict) or not value:
            raise BenchmarkManifestError(f"case {case_id}: {prefix} must be a non-empty table when known")
        return dict(sorted(value.items()))
    if state == "known-empty":
        if value != {}:
            raise BenchmarkManifestError(f"case {case_id}: {prefix} must be {{}} when known-empty")
        return {}
    if value is not None:
        raise BenchmarkManifestError(f"case {case_id}: {prefix} must be omitted when state is {state}")
    return None


def _assert_unique_ids(cases: tuple[BenchmarkCase, ...]) -> None:
    ids = [case.case_id for case in cases]
    if len(ids) != len(set(ids)):
        raise BenchmarkManifestError("duplicate case id")
