#!/usr/bin/env python3
"""Scaffold a language frontend module tree.

Stamps `src/languages/<id>/` with descriptor and table stubs whose TODOs
mirror docs/engineering/add-a-language.md. It never touches existing files,
the registry, or the knowledge pack — the guard tests and the compiler walk
you through the wiring on purpose.

Usage: python3 scripts/new-language.py <id> "<Display Label>"
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

MOD_TEMPLATE = """\
use super::conventions::PathConventions;
use super::{{GrammarBinding, LanguageFrontend}};
use crate::audits::context::LanguageKind;

// TODO(add-a-language): declare table modules as you wire them.
// mod imports;
// mod review;
// mod risk;

pub(super) static {upper}: LanguageFrontend = LanguageFrontend {{
    id: "{id}",
    label: "{label}",
    // TODO(add-a-language): use the real LanguageKind variant and route it
    // in `frontend_for_kind` (the exhaustive match will not compile until
    // you do). Register the frontend in `ALL_FRONTENDS` as well.
    kind: LanguageKind::Unknown,
    knowledge_ids: &["{id}"],
    // TODO(add-a-language): bind detection labels to a ParseLanguage variant
    // once a tree-sitter grammar is wired in `src/analysis/parse.rs`.
    grammars: &[],
    imports: None,
    taint: None,
    review: None,
    risk: None,
    conventions: &{upper}_CONVENTIONS,
}};

static {upper}_CONVENTIONS: PathConventions = PathConventions {{
    // TODO(add-a-language): language test-file naming, e.g.
    // |name| name.ends_with("_test.{id}")
    test_file_name: |_| false,
    test_prefix_marks_test: true,
    test_support: None,
    entrypoint_content: None,
}};
"""


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    lang_id, label = sys.argv[1], sys.argv[2]
    if not re.fullmatch(r"[a-z][a-z0-9-]*", lang_id):
        print(f"id must be a kebab-case slug, got: {lang_id}", file=sys.stderr)
        return 2

    module = REPO_ROOT / "src" / "languages" / lang_id.replace("-", "_")
    if module.exists():
        print(f"refusing to overwrite existing module: {module}", file=sys.stderr)
        return 1

    module.mkdir(parents=True)
    upper = lang_id.replace("-", "_").upper()
    (module / "mod.rs").write_text(
        MOD_TEMPLATE.format(id=lang_id, label=label, upper=upper),
        encoding="utf-8",
    )

    print(f"scaffolded {module}/mod.rs")
    print("next steps (docs/engineering/add-a-language.md):")
    print(f"  1. declare `mod {lang_id.replace('-', '_')};` in src/languages/mod.rs")
    print(f"  2. register &{lang_id.replace('-', '_')}::{upper} in ALL_FRONTENDS")
    print("  3. route its LanguageKind in frontend_for_kind")
    print("  4. follow the checklist; the guard tests enforce the rest")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
