#!/usr/bin/env python3
"""Validate release metadata and extract release notes from CHANGELOG.md."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parent.parent
CHANGELOG = ROOT / "CHANGELOG.md"
MARKDOWN_LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
RELEASE_HEADING = re.compile(
    r"^## \[(?P<version>\d+\.\d+\.\d+)\] - (?P<date>\d{4}-\d{2}-\d{2})$",
    re.MULTILINE,
)


class ContractError(RuntimeError):
    pass


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def json_file(path: str) -> dict:
    return json.loads(read_text(ROOT / path))


def cargo_version() -> str:
    match = re.search(
        r"^\[package\]\s+name\s*=\s*\"repopilot\"\s+version\s*=\s*\"([^\"]+)\"",
        read_text(ROOT / "Cargo.toml"),
        re.MULTILINE,
    )
    if not match:
        raise ContractError("Could not read package version from Cargo.toml")
    return match.group(1)


def cargo_lock_version() -> str:
    match = re.search(
        r"\[\[package\]\]\s+name = \"repopilot\"\s+version = \"([^\"]+)\"",
        read_text(ROOT / "Cargo.lock"),
        re.MULTILINE,
    )
    if not match:
        raise ContractError("Could not read repopilot version from Cargo.lock")
    return match.group(1)


def yaml_default_version(path: str, key: str) -> str:
    text = read_text(ROOT / path)
    match = re.search(
        rf"(?ms)^(?P<indent>[ \t]*){re.escape(key)}:\s*$"
        rf".*?^(?P=indent)[ \t]+default:\s*[\"']?(?P<value>[^\"'\s]+)",
        text,
    )
    if not match:
        raise ContractError(f"Could not read {key} default from {path}")
    return match.group("value")


def version_from_tag(tag: str) -> str:
    version = tag.removeprefix("v")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ContractError(f"Invalid release tag: {tag}")
    return version


def expected_version(tag: str | None) -> str:
    version = cargo_version()
    if tag:
        tag_version = version_from_tag(tag)
        if tag_version != version:
            raise ContractError(
                f"Tag {tag} does not match Cargo.toml version {version}"
            )
    return version


def release_section(version: str) -> str:
    changelog = read_text(CHANGELOG)
    headings = list(RELEASE_HEADING.finditer(changelog))
    match = next(
        (heading for heading in headings if heading.group("version") == version),
        None,
    )
    if match is None:
        raise ContractError(f"CHANGELOG.md has no release section for {version}")

    next_heading = re.search(r"^## ", changelog[match.end() :], re.MULTILINE)
    end = match.end() + next_heading.start() if next_heading else len(changelog)
    body = changelog[match.end() : end].strip()
    if not body or not re.search(r"^### ", body, re.MULTILINE):
        raise ContractError(f"CHANGELOG.md release {version} has no release notes")
    return body


def check_versions(version: str) -> None:
    package = json_file("package.json")
    vscode = json_file("editors/vscode/package.json")
    vscode_lock = json_file("editors/vscode/package-lock.json")

    versions = {
        "Cargo.lock": cargo_lock_version(),
        "package.json": package["version"],
        "editors/vscode/package.json": vscode["version"],
        "editors/vscode/package-lock.json": vscode_lock["version"],
        "editors/vscode/package-lock.json packages['']": vscode_lock["packages"][""][
            "version"
        ],
        "action.yml version input": yaml_default_version("action.yml", "version"),
        "reusable workflow version input": yaml_default_version(
            ".github/workflows/repopilot-pr-review.yml", "version"
        ),
    }

    for dependency, dependency_version in package["optionalDependencies"].items():
        versions[f"package.json optional dependency {dependency}"] = dependency_version

    workflow = read_text(ROOT / ".github/workflows/repopilot-pr-review.yml")
    action_ref = re.search(r"uses:\s+MykytaStel/repopilot@v([^\s]+)", workflow)
    if not action_ref:
        raise ContractError("Reusable workflow does not pin the RepoPilot action")
    versions["reusable workflow action ref"] = action_ref.group(1)

    mismatches = {
        source: found for source, found in versions.items() if found != version
    }
    if mismatches:
        details = ", ".join(f"{source}={found}" for source, found in mismatches.items())
        raise ContractError(f"Version mismatch for {version}: {details}")


def markdown_files() -> list[Path]:
    files = [
        ROOT / "README.md",
        ROOT / "CONTRIBUTING.md",
        ROOT / "SECURITY.md",
        ROOT / "editors/vscode/README.md",
    ]
    files.extend(sorted((ROOT / "docs").rglob("*.md")))
    return files


def check_markdown_links() -> None:
    failures: list[str] = []
    for document in markdown_files():
        text = read_text(document)
        for raw_target in MARKDOWN_LINK.findall(text):
            target = raw_target.strip().split(maxsplit=1)[0].strip("<>")
            if (
                not target
                or target.startswith(("#", "http://", "https://", "mailto:"))
            ):
                continue
            path_text = unquote(target.split("#", 1)[0].split("?", 1)[0])
            resolved = (document.parent / path_text).resolve()
            if not resolved.exists():
                failures.append(
                    f"{document.relative_to(ROOT)} -> {target}"
                )
    if failures:
        raise ContractError("Broken Markdown links:\n  " + "\n  ".join(failures))


def check_npm_claims() -> None:
    stale_patterns = [
        re.compile(r"REPOPILOT_SKIP_DOWNLOAD"),
        re.compile(
            r"npm package installer downloads .* GitHub Release binary",
            re.IGNORECASE,
        ),
        re.compile(r"downloads .* during `?postinstall`?", re.IGNORECASE),
    ]
    failures: list[str] = []
    for document in markdown_files():
        text = read_text(document)
        for pattern in stale_patterns:
            if pattern.search(text):
                failures.append(
                    f"{document.relative_to(ROOT)} matches {pattern.pattern!r}"
                )
    if failures:
        raise ContractError("Stale npm installation claims:\n  " + "\n  ".join(failures))


def check_cargo_package() -> None:
    result = subprocess.run(
        ["cargo", "package", "--list", "--allow-dirty"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    allowed_files = {
        ".cargo_vcs_info.json",
        "Cargo.lock",
        "Cargo.toml",
        "Cargo.toml.orig",
        "LICENSE",
        "README.md",
    }
    unexpected = [
        path
        for path in result.stdout.splitlines()
        if path not in allowed_files and not path.startswith("src/")
    ]
    if unexpected:
        raise ContractError(
            "Cargo package contains files outside the allowlist:\n  "
            + "\n  ".join(unexpected)
        )


def check_contract(tag: str | None) -> None:
    version = expected_version(tag)
    check_versions(version)
    release_section(version)
    check_markdown_links()
    check_npm_claims()
    check_cargo_package()
    print(f"Release contract passed for {version}")


def write_notes(tag: str, output: Path) -> None:
    version = version_from_tag(tag)
    body = release_section(version)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        body
        + "\n\n"
        + f"Full changelog: https://github.com/MykytaStel/repopilot/blob/v{version}/CHANGELOG.md\n",
        encoding="utf-8",
    )
    print(f"Wrote release notes for {version} to {output}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    check = subparsers.add_parser("check")
    check.add_argument("--tag")

    notes = subparsers.add_parser("notes")
    notes.add_argument("--tag", required=True)
    notes.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "check":
            check_contract(args.tag)
        else:
            write_notes(args.tag, args.output)
    except (ContractError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"release-contract: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
