#!/usr/bin/env python3
"""Validate release metadata and render curated GitHub release notes."""

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
ACTION_USE = re.compile(r"^\s*-?\s*uses:\s*([^@\s]+)@([^\s#]+)", re.MULTILINE)
PINNED_SHA = re.compile(r"^[0-9a-f]{40}$")
RELEASE_FRONT_MATTER = re.compile(
    r"\A---\n(?P<meta>.*?)\n---\n(?P<body>.*)\Z",
    re.DOTALL,
)
RELEASE_SECTION = re.compile(r"^## (?P<title>[^\n]+)$", re.MULTILINE)
RELEASE_REQUIRED_SECTIONS = ("Highlights", "Compatibility", "Upgrade")


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
        raise ContractError(f"CHANGELOG.md release {version} has no technical entries")
    return body


def release_notes(version: str) -> str:
    title, description, body = release_document(version)
    if not title:
        raise ContractError(f"Release title for {version} is empty")
    return f"{description}\n\n{body}"


def release_title(version: str) -> str:
    title, _description, _body = release_document(version)
    return f"RepoPilot v{version}: {title}"


def release_document(version: str) -> tuple[str, str, str]:
    curated = ROOT / "docs" / "releases" / f"v{version}.md"
    if not curated.exists():
        raise ContractError(f"Missing curated GitHub release notes for {version}")

    text = read_text(curated).strip()
    if not text:
        raise ContractError(f"Release notes for {version} are empty")
    title, description, body = parse_release_document(version, text)
    validate_release_note_body(version, body)
    return title, description, body


def parse_release_document(version: str, text: str) -> tuple[str, str, str]:
    match = RELEASE_FRONT_MATTER.match(text)
    if not match:
        raise ContractError(
            f"Release notes for {version} must start with title/description front matter"
        )

    metadata: dict[str, str] = {}
    for line in match.group("meta").splitlines():
        if not line.strip():
            continue
        key, separator, value = line.partition(":")
        if not separator:
            raise ContractError(f"Invalid release metadata line for {version}: {line}")
        metadata[key.strip()] = value.strip().strip("\"'")

    title = metadata.get("title", "")
    description = metadata.get("description", "")
    body = match.group("body").strip()
    if not title:
        raise ContractError(f"Release title for {version} is empty")
    if not description:
        raise ContractError(f"Release description for {version} is empty")
    if not body:
        raise ContractError(f"Release notes for {version} have no body")
    return title, description, body


def validate_release_note_body(version: str, body: str) -> None:
    if re.search(r"^# ", body, re.MULTILINE):
        raise ContractError(f"Release notes for {version} must not include an H1")

    sections = list(RELEASE_SECTION.finditer(body))
    titles = [section.group("title").strip() for section in sections]
    if titles[: len(RELEASE_REQUIRED_SECTIONS)] != list(RELEASE_REQUIRED_SECTIONS):
        expected = ", ".join(f"## {section}" for section in RELEASE_REQUIRED_SECTIONS)
        raise ContractError(f"Release notes for {version} must start with {expected}")

    highlights = release_section_body(body, sections, "Highlights")
    highlight_count = len(re.findall(r"^- \S", highlights, re.MULTILINE))
    if not 3 <= highlight_count <= 5:
        raise ContractError(
            f"Release notes for {version} must have 3-5 highlight bullets"
        )

    upgrade = release_section_body(body, sections, "Upgrade")
    required_commands = [
        f"cargo install repopilot --version {version} --force",
        f"npm install -g repopilot@{version}",
    ]
    missing = [command for command in required_commands if command not in upgrade]
    if missing:
        raise ContractError(
            f"Release notes for {version} missing pinned upgrade command: {missing[0]}"
        )


def release_section_body(
    body: str, sections: list[re.Match[str]], title: str
) -> str:
    for index, section in enumerate(sections):
        if section.group("title").strip() != title:
            continue
        end = sections[index + 1].start() if index + 1 < len(sections) else len(body)
        return body[section.end() : end].strip()
    raise ContractError(f"Release notes section is missing: {title}")


def check_versions(version: str) -> None:
    package = json_file("package.json")

    versions = {
        "Cargo.lock": cargo_lock_version(),
        "package.json": package["version"],
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


def workflow_files() -> list[Path]:
    return [ROOT / "action.yml", *sorted((ROOT / ".github/workflows").glob("*.y*ml"))]


def check_action_pins() -> None:
    failures: list[str] = []
    for workflow in workflow_files():
        for action, ref in ACTION_USE.findall(read_text(workflow)):
            if action.startswith("./") or action == "MykytaStel/repopilot":
                continue
            if not PINNED_SHA.fullmatch(ref):
                failures.append(f"{workflow.relative_to(ROOT)} -> {action}@{ref}")
    if failures:
        raise ContractError(
            "Third-party GitHub Actions must use commit SHAs:\n  "
            + "\n  ".join(failures)
        )


def check_release_orchestration() -> None:
    release = read_text(ROOT / ".github/workflows/release.yml")
    npm = read_text(ROOT / ".github/workflows/publish-npm.yml")
    if "uses: ./.github/workflows/publish-npm.yml" not in release:
        raise ContractError("Release workflow does not call the npm publishing workflow")
    if "workflow_call:" not in npm or "workflow_dispatch:" not in npm:
        raise ContractError(
            "npm publishing must support workflow_call and manual recovery"
        )
    if re.search(r"(?i)\b(vsix|vscode|marketplace)\b", release):
        raise ContractError("Release workflow still contains a VS Code/VSIX surface")


def check_removed_vscode_surface() -> None:
    stale_paths = [
        ROOT / "editors/vscode/package.json",
        ROOT / "editors/vscode/package-lock.json",
        ROOT / "editors/vscode/src/extension.ts",
        ROOT / "scripts/build-vscode-platform-packages.js",
    ]
    existing = [path.relative_to(ROOT) for path in stale_paths if path.exists()]
    if existing:
        raise ContractError(
            "Removed VS Code surface still exists:\n  "
            + "\n  ".join(map(str, existing))
        )


def check_contract(tag: str | None) -> None:
    version = expected_version(tag)
    check_versions(version)
    release_section(version)
    release_notes(version)
    check_markdown_links()
    check_npm_claims()
    check_cargo_package()
    check_action_pins()
    check_release_orchestration()
    check_removed_vscode_surface()
    print(f"Release contract passed for {version}")


def write_notes(tag: str, output: Path) -> None:
    version = version_from_tag(tag)
    body = release_notes(version)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        body
        + "\n\n"
        + f"Full technical changelog: https://github.com/MykytaStel/repopilot/blob/{tag}/CHANGELOG.md\n",
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

    title = subparsers.add_parser("title")
    title.add_argument("--tag", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "check":
            check_contract(args.tag)
        elif args.command == "notes":
            write_notes(args.tag, args.output)
        else:
            print(release_title(version_from_tag(args.tag)))
    except (ContractError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"release-contract: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
