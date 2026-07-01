//! Renders the human-readable rules reference (`docs/rules-reference.md`) from
//! the rule registry, so the published catalog can never drift from the code
//! that actually emits findings. The drift check lives in
//! `tests/rules_reference_doc.rs`; regenerate with
//! `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`.

use crate::findings::types::FindingCategory;
use crate::rules::all_rule_metadata;
use crate::rules::metadata::RuleMetadata;
use std::fmt::Write;

/// Category render order. Every registered rule belongs to exactly one of these,
/// so a new `FindingCategory` variant fails the registry's own coverage tests
/// before it can silently drop out of this reference.
const CATEGORY_ORDER: &[FindingCategory] = &[
    FindingCategory::Architecture,
    FindingCategory::CodeQuality,
    FindingCategory::Testing,
    FindingCategory::Security,
    FindingCategory::Framework,
];

const GENERATED_BANNER: &str = "<!-- @generated from the rule registry — do not edit by hand. -->\n<!-- Regenerate with `REPOPILOT_BLESS=1 cargo test --test rules_reference_doc`. -->";

/// Deterministic Markdown for the full rule catalog: grouped by category, rules
/// sorted by id within each group, defaults pulled straight from the registry.
pub fn render_rules_reference_markdown() -> String {
    let mut rules: Vec<&'static RuleMetadata> = all_rule_metadata().collect();
    rules.sort_by(|a, b| a.rule_id.cmp(b.rule_id));

    let category_count = CATEGORY_ORDER
        .iter()
        .filter(|category| rules.iter().any(|rule| rule.category == **category))
        .count();

    let mut out = String::new();
    let _ = writeln!(out, "# RepoPilot Rules Reference\n");
    let _ = writeln!(out, "{GENERATED_BANNER}\n");
    let _ = writeln!(
        out,
        "RepoPilot ships {} rules across {} categories. Every finding traces back \
to one of these rules. Severity and confidence below are the registry defaults: \
context (audit tiers, knowledge packs) may lower them, or raise them up to a \
rule's declared ceiling, but never past it.\n",
        rules.len(),
        category_count
    );

    for category in CATEGORY_ORDER {
        let in_category: Vec<&RuleMetadata> = rules
            .iter()
            .copied()
            .filter(|rule| rule.category == *category)
            .collect();
        if in_category.is_empty() {
            continue;
        }

        let _ = writeln!(out, "## {}\n", category_title(category));
        for rule in in_category {
            render_rule(&mut out, rule);
        }
    }

    // Exactly one trailing newline keeps the golden stable across editors.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn render_rule(out: &mut String, rule: &RuleMetadata) {
    let _ = writeln!(out, "### `{}` — {}\n", rule.rule_id, rule.title);
    let _ = writeln!(out, "- **Severity:** {}", rule.default_severity.label());
    let _ = writeln!(out, "- **Confidence:** {}", rule.default_confidence.label());
    let _ = writeln!(
        out,
        "- **Lifecycle:** {}",
        rule.requirements.lifecycle.label()
    );
    let _ = writeln!(out, "- **Signal source:** {}", rule.signal_source.label());
    let _ = writeln!(
        out,
        "- **Execution scope:** {}",
        rule.requirements.scope.label()
    );
    let required_facts = rule
        .requirements
        .fact_kinds
        .iter()
        .map(|fact| fact.label())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "- **Required facts:** {required_facts}");
    let _ = writeln!(
        out,
        "- **Cache policy:** {}",
        rule.requirements.cache_policy.label()
    );
    let produces = rule
        .requirements
        .produces
        .iter()
        .map(|output| output.label())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "- **Produces:** {produces}\n");

    let description = rule.description.trim();
    if !description.is_empty() {
        let _ = writeln!(out, "{description}\n");
    }

    if let Some(recommendation) = non_empty(rule.recommendation) {
        let _ = writeln!(out, "**Recommendation:** {recommendation}\n");
    }
    if let Some(notes) = non_empty(rule.false_positive_notes) {
        let _ = writeln!(out, "**Known false positives:** {notes}\n");
    }
    if let Some(url) = non_empty(rule.docs_url) {
        let _ = writeln!(out, "**Reference:** <{url}>\n");
    }
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

fn category_title(category: &FindingCategory) -> &'static str {
    match category {
        FindingCategory::Architecture => "Architecture",
        FindingCategory::CodeQuality => "Code quality",
        FindingCategory::Testing => "Testing",
        FindingCategory::Security => "Security",
        FindingCategory::Framework => "Framework",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_lists_every_registered_rule() {
        let markdown = render_rules_reference_markdown();
        for rule in all_rule_metadata() {
            assert!(
                markdown.contains(&format!("### `{}`", rule.rule_id)),
                "rules reference is missing {}",
                rule.rule_id
            );
        }
    }

    #[test]
    fn reference_is_deterministic() {
        assert_eq!(
            render_rules_reference_markdown(),
            render_rules_reference_markdown()
        );
    }

    #[test]
    fn reference_has_one_trailing_newline() {
        let markdown = render_rules_reference_markdown();
        assert!(markdown.ends_with('\n'));
        assert!(!markdown.ends_with("\n\n"));
    }
}
