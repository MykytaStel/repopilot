//! Renders `docs/language-support.md` from the language frontend registry
//! and the bundled knowledge pack, so the published support matrix can never
//! drift from what is actually wired. The drift check lives in
//! `tests/language_support_doc.rs`; regenerate with
//! `REPOPILOT_BLESS=1 cargo test --test language_support_doc`.

use super::{LanguageFrontend, all_frontends, frontend_for_knowledge_id};
use crate::knowledge::active_knowledge;
use crate::knowledge::model::SupportLevel;
use std::fmt::Write;

const GENERATED_BANNER: &str = "<!-- @generated from the language frontend registry — do not edit by hand. -->\n<!-- Regenerate with `REPOPILOT_BLESS=1 cargo test --test language_support_doc`. -->";

/// Deterministic Markdown for the language support matrix: one capability row
/// per frontend (facts derived from wiring, never declared), then the
/// detect-level languages from the knowledge pack.
pub fn render_language_support_markdown() -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Language and Framework Support\n");
    let _ = writeln!(out, "{GENERATED_BANNER}\n");
    let _ = writeln!(
        out,
        "RepoPilot combines generic repository heuristics with language-aware\n\
analysis owned by *language frontends* (`src/languages/`). The capability\n\
columns below are derived from what each frontend actually wires — a column\n\
cannot be claimed without the code behind it. `Declared` is the support\n\
level the bundled knowledge pack asserts; where it exceeds the wired\n\
capabilities, the gap is tracked by the registry's support-honesty guard\n\
test rather than hidden.\n"
    );

    let _ = writeln!(out, "## Frontend capabilities\n");
    let _ = writeln!(
        out,
        "| Language | Grammar | Imports | Review signals | Taint flows | Runtime risk | Conventions | Declared |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    for frontend in all_frontends() {
        if frontend.id == "generic" {
            continue;
        }
        let _ = writeln!(out, "{}", frontend_row(frontend));
    }
    let _ = writeln!(
        out,
        "\nNotes:\n\n\
- **Rust runtime risk** is covered by the dedicated `rust.panic-risk` rule\n\
  family rather than the shared runtime-risk audit, so its column reads “—”\n\
  here; how it counts toward the capability model is tracked in the\n\
  support-honesty ledger.\n\
- JavaScript and TypeScript (with their React dialects) share one frontend\n\
  family: the same grammar shapes, import extractor, and signal tables.\n"
    );

    let _ = writeln!(out, "## Detected languages without a frontend\n");
    let _ = writeln!(
        out,
        "Files in these languages still contribute to repository size, scan\n\
scope, file roles (via the shared context classifier), and generic findings;\n\
they have no language-specific extractors.\n"
    );
    for (heading, level) in [
        (
            "Context-aware (shared classifier)",
            SupportLevel::ContextAware,
        ),
        ("Import-aware", SupportLevel::ImportAware),
        ("Detect-only", SupportLevel::DetectOnly),
        (
            "Declared rule-aware without wiring",
            SupportLevel::RuleAware,
        ),
    ] {
        let names: Vec<&str> = active_knowledge()
            .languages
            .iter()
            .filter(|profile| {
                profile.support == level && frontend_for_knowledge_id(&profile.id).is_none()
            })
            .map(|profile| profile.name.as_str())
            .collect();
        if names.is_empty() {
            continue;
        }
        let _ = writeln!(out, "- **{heading}:** {}.", names.join(", "));
    }

    let _ = writeln!(
        out,
        "\n## Rule philosophy\n\n\
RepoPilot should not treat every paradigm as a smell. Functional,\n\
object-oriented, procedural, and declarative code can all be valid; rules\n\
use context and confidence instead of assuming one style is always better:\n\n\
```text\nevidence + context + severity + confidence + recommendation\n```\n\n\
## Limitations\n\n\
RepoPilot is not a compiler or type checker. Some rules are text-based or\n\
heuristic; findings are review signals, not absolute truth. Use\n\
language-specific tools alongside it: `cargo clippy`, `tsc`/ESLint, Ruff and\n\
Pyright, `go vet`, or your build's own checks.\n\n\
## Adding a language\n\n\
The whole point of the frontend contract is that support is added by\n\
writing tables, not by editing engines. The checklist lives in\n\
[Add a language](engineering/add-a-language.md)."
    );

    out
}

fn frontend_row(frontend: &LanguageFrontend) -> String {
    let declared = active_knowledge()
        .languages
        .iter()
        .find(|profile| profile.id == frontend.id)
        .map(|profile| support_label(profile.support))
        .unwrap_or("—");

    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {} |",
        frontend.label,
        mark(!frontend.grammars.is_empty()),
        mark(frontend.imports.is_some()),
        mark(frontend.review.is_some()),
        mark(frontend.taint.is_some()),
        mark(frontend.risk.is_some()),
        mark(!std::ptr::eq(
            frontend.conventions,
            &super::conventions::GENERIC_CONVENTIONS
        )),
        declared,
    )
}

fn mark(wired: bool) -> &'static str {
    if wired { "✓" } else { "—" }
}

fn support_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::RuleAware => "rule-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::DetectOnly => "detect-only",
    }
}
