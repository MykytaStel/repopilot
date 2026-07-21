#!/usr/bin/env python3
"""Generate a PR description from commits and the CHANGELOG diff.

Fills in the factual sections of `.github/pull_request_template.md`
(Summary, Type of change, What changed, Why, Changelog) from information
already in the branch — commit subjects and whatever this branch added to
`CHANGELOG.md`. It never touches the Contract/Verification checklists: those
require a human (or Claude) to actually check the boxes, and auto-checking
them would defeat their purpose.

Usage:
    python3 scripts/generate-pr-body.py [--base origin/main] [--head HEAD]

Prints the generated Markdown body to stdout.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TEMPLATE = REPO_ROOT / ".github" / "pull_request_template.md"

CONVENTIONAL_TYPES = {
    "feat": "Feature",
    "fix": "Bug fix",
    "refactor": "Refactor",
    "perf": "Refactor",
    "test": "Tests",
    "docs": "Documentation",
    "ci": "CI / repository maintenance",
    "chore": "CI / repository maintenance",
    "build": "CI / repository maintenance",
}
ALL_TYPE_LABELS = [
    "Feature",
    "Refactor",
    "Bug fix",
    "Tests",
    "Documentation",
    "CI / repository maintenance",
]


def run(args: list[str]) -> str:
    return subprocess.run(
        args, cwd=REPO_ROOT, capture_output=True, text=True, check=True
    ).stdout


def commit_subjects(base: str, head: str) -> list[str]:
    log = run(["git", "log", "--no-merges", "--pretty=format:%s", f"{base}..{head}"])
    return [line.strip() for line in log.splitlines() if line.strip()]


def commit_bodies(base: str, head: str) -> list[str]:
    log = run(
        ["git", "log", "--no-merges", "--pretty=format:%B%x00", f"{base}..{head}"]
    )
    return [body.strip() for body in log.split("\x00") if body.strip()]


def infer_types(subjects: list[str]) -> set[str]:
    types: set[str] = set()
    for subject in subjects:
        match = re.match(r"^([a-z]+)(\([^)]*\))?!?:", subject)
        if match:
            types.add(CONVENTIONAL_TYPES.get(match.group(1), None) or "")
    types.discard("")
    return types or {"Feature"}


def strip_conventional_prefix(subject: str) -> str:
    return re.sub(r"^[a-z]+\([^)]*\)?!?:\s*", "", subject).strip()


def summary_line(subjects: list[str]) -> str:
    if not subjects:
        return ""
    if len(subjects) == 1:
        text = strip_conventional_prefix(subjects[0])
        return text[:1].upper() + text[1:] if text else ""
    return "\n".join(f"- {strip_conventional_prefix(s)}" for s in subjects)


def why_paragraph(bodies: list[str], bullets: list[str]) -> str:
    """The first commit-message paragraph after the subject line, if any
    commit carries one. Falls back to the first CHANGELOG bullet with its
    bold lead sentence stripped (this repo's bullets open with a headline
    and follow with the rationale/impact), then to the template's own
    placeholder comment."""
    for body in bodies:
        lines = body.splitlines()
        if len(lines) <= 1:
            continue
        rest = "\n".join(lines[1:]).strip()
        rest = re.sub(r"\n*co-authored-by:.*$", "", rest, flags=re.I | re.M).strip()
        if rest:
            return rest
    if bullets:
        rest = re.sub(r"^\*\*[^*]+\*\*\s*", "", bullets[0], count=1)
        if rest.strip():
            return rest.strip()
    return "<!-- Why is this change needed? What problem does it solve? -->"


def changelog_bullets(base: str, head: str) -> list[str]:
    """Top-level `- **Bold lead.** ...` bullets added to CHANGELOG.md on this
    branch, each flattened to one line (wrapped continuation lines joined)."""
    try:
        diff = run(["git", "diff", f"{base}...{head}", "--", "CHANGELOG.md"])
    except subprocess.CalledProcessError:
        return []

    added_lines = [
        line[1:]
        for line in diff.splitlines()
        if line.startswith("+") and not line.startswith("+++")
    ]

    bullets: list[str] = []
    current: list[str] = []
    for line in added_lines:
        if re.match(r"^-\s+\*\*", line):
            if current:
                bullets.append(" ".join(current))
            current = [line.lstrip("- ").strip()]
        elif line.strip() and current and line.startswith("  "):
            current.append(line.strip())
        elif not line.strip() and current:
            bullets.append(" ".join(current))
            current = []
    if current:
        bullets.append(" ".join(current))
    return bullets


def split_on_top_level_semicolons(text: str) -> list[str]:
    """Splits on `; ` while ignoring semicolons inside `(...)` or
    `` `...` ``, so a parenthetical or code span never gets torn in half."""
    clauses: list[str] = []
    depth = 0
    in_code = False
    start = 0
    i = 0
    while i < len(text):
        char = text[i]
        if char == "`":
            in_code = not in_code
        elif not in_code and char == "(":
            depth += 1
        elif not in_code and char == ")":
            depth = max(0, depth - 1)
        elif not in_code and depth == 0 and char == ";" and text[i + 1 : i + 2] == " ":
            clauses.append(text[start:i])
            start = i + 2
            i += 1
        i += 1
    clauses.append(text[start:])
    return clauses


def split_bullet(bullet: str) -> list[str]:
    """This repo's CHANGELOG bullets open with a `**Bold lead.**` sentence
    and separate clauses with `; ` — split those into short sub-items so
    "What changed" reads as a list, not one paragraph. A bullet without that
    shape (fewer than three top-level `; ` clauses) stays a single item."""
    match = re.match(r"^(\*\*[^*]+\*\*)\s*(.*)$", bullet, flags=re.S)
    lead, rest = match.groups() if match else ("", bullet)
    clauses = split_on_top_level_semicolons(rest) if rest else []
    if len(clauses) < 3:
        return [bullet]
    items = [f"{lead} {clauses[0]}".strip()] if lead else [clauses[0]]
    for clause in clauses[1:]:
        clause = clause.strip()
        clause = re.sub(r"^(and|but)\s+", "", clause, count=1, flags=re.I)
        if clause:
            items.append(clause[:1].upper() + clause[1:])
    return items


def changed_files(base: str, head: str) -> list[str]:
    diff = run(["git", "diff", f"{base}...{head}", "--name-only"])
    return [line.strip() for line in diff.splitlines() if line.strip()]


def render(base: str, head: str) -> str:
    subjects = commit_subjects(base, head)
    bodies = commit_bodies(base, head)
    types = infer_types(subjects)
    bullets = changelog_bullets(base, head)
    files = changed_files(base, head)

    type_checkboxes = "\n".join(
        f"- [{'x' if label in types else ' '}] {label}" for label in ALL_TYPE_LABELS
    )

    if bullets:
        items = [item for bullet in bullets for item in split_bullet(bullet)]
        what_changed = "\n".join(f"- {item}" for item in items)
    else:
        what_changed = "\n".join(f"- {strip_conventional_prefix(s)}" for s in subjects) or "-"

    changelog_checked = "x" if "CHANGELOG.md" in files else " "

    template = TEMPLATE.read_text()
    body = template
    body = body.replace(
        "<!-- What changed in this PR? Keep it short and practical. -->",
        summary_line(subjects) or "<!-- What changed in this PR? Keep it short and practical. -->",
        1,
    )
    body = re.sub(
        r"- \[ \] Feature\n- \[ \] Refactor\n- \[ \] Bug fix\n- \[ \] Tests\n- \[ \] Documentation\n- \[ \] CI / repository maintenance",
        type_checkboxes,
        body,
        count=1,
    )
    body = body.replace(
        "<!-- List the main changes. -->\n\n-\n-\n-",
        f"<!-- List the main changes. -->\n\n{what_changed}",
        1,
    )
    body = body.replace(
        "<!-- Why is this change needed? What problem does it solve? -->",
        why_paragraph(bodies, bullets),
        1,
    )
    body = body.replace(
        "- [ ] `CHANGELOG.md` updated (skip for CI-only or doc changes with no user-visible effect)",
        f"- [{changelog_checked}] `CHANGELOG.md` updated (skip for CI-only or doc changes with no user-visible effect)",
        1,
    )
    return body


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--head", default="HEAD")
    args = parser.parse_args()

    print(render(args.base, args.head))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
