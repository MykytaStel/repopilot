"""Path rules for the integrity corpus: which changed files are test or gate files."""

from __future__ import annotations

import re

JS_TS = r"\.[cm]?[jt]sx?$"
TEST_PATH = [
    re.compile(r"\.(test|spec)" + JS_TS),
    re.compile(r"(^|/)(__tests__|tests?)/.*" + JS_TS),
    re.compile(r"(^|/)test_[^/]*\.py$"),
    re.compile(r"_test\.py$"),
    re.compile(r"(^|/)tests?/.*\.py$"),
    re.compile(r"(^|/)conftest\.py$"),
    re.compile(r"_test\.go$"),
    re.compile(r"(^|/)tests/.*\.rs$"),
]
RUST_INLINE_TEST = re.compile(r"#\[(tokio::)?test\]|#\[cfg\(test\)\]|mod tests\b")
GATE_PATH = [
    re.compile(r"(^|/)\.github/workflows/[^/]+\.ya?ml$"),
    re.compile(r"(^|/)\.gitlab-ci\.ya?ml$"),
    re.compile(r"(^|/)package\.json$"),
    re.compile(r"(^|/)(jest|vitest|vite|karma)\.config\.[cm]?[jt]s$"),
    re.compile(r"(^|/)(pytest\.ini|pyproject\.toml|setup\.cfg|tox\.ini|mypy\.ini|\.coveragerc|codecov\.ya?ml)$"),
    re.compile(r"(^|/)tsconfig[^/]*\.json$"),
    re.compile(r"(^|/)\.eslintrc[^/]*$|(^|/)eslint\.config\.[cm]?[jt]s$"),
    re.compile(r"(^|/)(Cargo\.toml|clippy\.toml|\.golangci\.ya?ml|ruff\.toml|\.ruff\.toml)$"),
]


def is_test_file(file: dict) -> bool:
    name = file["filename"]
    if any(p.search(name) for p in TEST_PATH):
        return True
    return name.endswith(".rs") and bool(RUST_INLINE_TEST.search(file.get("patch") or ""))


def is_gate_file(name: str) -> bool:
    return any(p.search(name) for p in GATE_PATH)
