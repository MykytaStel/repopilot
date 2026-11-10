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
    render_language_support_header(&mut out);
    render_frontend_capabilities(&mut out);
    render_bounded_analysis(&mut out);
    render_unwired_languages(&mut out);
    render_rule_philosophy(&mut out);
    out
}

fn render_language_support_header(out: &mut String) {
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
}

fn render_frontend_capabilities(out: &mut String) {
    let _ = writeln!(out, "## Frontend capabilities\n");
    let _ = writeln!(
        out,
        "| Language | Grammar | Imports | Review signals | Taint flows | Runtime risk | Conventions | Declared |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    for frontend in all_frontends() {
        if frontend.id != "generic" {
            let _ = writeln!(out, "{}", frontend_row(frontend));
        }
    }
    let _ = writeln!(
        out,
        "\nNotes:\n\n\
- **Rust runtime risk** reads “✓ (dedicated)”: coverage comes from the\n\
  standalone `language.rust.panic-risk` audit — structural infallibility\n\
  detection, report-renderer path awareness — rather than the shared\n\
  per-node runtime-risk table other languages use. Too contextual for that\n\
  generic shape; it still counts toward the `RuntimeRisk` capability.\n\
- JavaScript and TypeScript (with their React dialects) share one frontend\n\
  family: the same grammar shapes, import extractor, and signal tables.\n"
    );
}

fn render_bounded_analysis(out: &mut String) {
    let _ = writeln!(
        out,
        "## Bounded cross-file analysis\n\n\
The frontend matrix shows which language facts are wired. These additional\n\
cross-file behaviors have narrower evidence boundaries:\n\n\
- **Rust:** review detects removed public free functions, changed public\n\
  function arity, and removed or privatized named public struct fields when a\n\
  surviving caller has an unambiguous file-backed module-qualified use.\n\
  Grouped `use` imports are expanded for removed-function contracts. The field\n\
  check covers direct struct literals; macros, `cfg`, re-exports, aliases, and\n\
  ambiguous module paths remain outside the claim.\n\
- **JavaScript and TypeScript:** local missing-import checks cover supported\n\
  extensionless relative imports and uniquely resolved configured aliases.\n\
  Package exports, dynamic imports, and ambiguous resolver candidates remain\n\
  uncertain.\n\
- **Python:** missing-import checks include bounded repository-rooted absolute\n\
  module candidates and package initializers. Namespace directories count as\n\
  safe local package candidates; environment-provided and installed modules\n\
  cannot be confirmed from repository files alone.\n\
- **Go:** local workspace and local `replace` module targets are considered\n\
  without fetching dependencies. Remote replacements, build-tag variants, and\n\
  generated packages are not treated as resolved repository sources.\n\
- **C#:** SQL taint analysis follows simple local command receiver aliases and\n\
  merges supported straight-line assignments across simple `if` branches.\n\
  Unmodeled control flow, ambiguous aliases, loops, and nested scopes cause\n\
  conservative abstention.\n\
- **Java and Kotlin:** same-package type references add local dependency edges\n\
  for impact analysis when a source type resolves uniquely. The edge does not\n\
  establish runtime reachability.\n\n\
These checks are bounded static analysis, not compiler or runtime proofs. Use\n\
the project's own build, type-check, and tests to verify a change.\n"
    );
}

fn render_unwired_languages(out: &mut String) {
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
        if !names.is_empty() {
            let _ = writeln!(out, "- **{heading}:** {}.", names.join(", "));
        }
    }
}

fn render_rule_philosophy(out: &mut String) {
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
        runtime_risk_cell(frontend),
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

fn runtime_risk_cell(frontend: &LanguageFrontend) -> &'static str {
    if frontend.risk.is_some() {
        "✓"
    } else if frontend.dedicated_risk_audit.is_some() {
        "✓ (dedicated)"
    } else {
        "—"
    }
}

fn support_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::RuleAware => "rule-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::DetectOnly => "detect-only",
    }
}
