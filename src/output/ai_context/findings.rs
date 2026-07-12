use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::output::ai_context::budget::{CategoryAllocation, allocate_categories, category_weight};
use crate::output::finding_helpers::{
    finding_location, finding_location_key, finding_recommendation,
};
use crate::output::report_text::first_sentence;
use std::fmt::Write as FmtWrite;

type CategoryFilter = fn(&FindingCategory) -> bool;

pub(super) struct CategoryRenderInfo {
    pub label: String,
    pub chars_added: usize,
    pub shown: usize,
}

const CATEGORY_ORDER: &[(&str, CategoryFilter)] = &[
    ("Security", |c| matches!(c, FindingCategory::Security)),
    ("Architecture", |c| {
        matches!(c, FindingCategory::Architecture)
    }),
    ("Code Quality", |c| {
        matches!(c, FindingCategory::CodeQuality)
    }),
    ("Testing", |c| matches!(c, FindingCategory::Testing)),
    ("Framework", |c| matches!(c, FindingCategory::Framework)),
];

/// 80% of total budget goes to findings; the rest covers header, hotfiles, recommendations, footer.
const FINDINGS_BUDGET_RATIO: usize = 4;
const FINDINGS_BUDGET_DENOM: usize = 5;

pub(super) fn render_findings_by_category(
    out: &mut String,
    findings: &[&Finding],
    total_budget_chars: usize,
    compact: bool,
) -> Vec<CategoryRenderInfo> {
    // Pass 1: collect non-empty groups and compute severity weights.
    let groups: Vec<(&str, Vec<&Finding>)> = CATEGORY_ORDER
        .iter()
        .filter_map(|(label, predicate)| {
            let group: Vec<&Finding> = findings
                .iter()
                .copied()
                .filter(|f| predicate(&f.category))
                .collect();
            if group.is_empty() {
                None
            } else {
                Some((*label, group))
            }
        })
        .collect();

    if groups.is_empty() {
        return vec![];
    }

    let weights: Vec<usize> = groups.iter().map(|(_, g)| category_weight(g)).collect();
    let findings_budget = total_budget_chars * FINDINGS_BUDGET_RATIO / FINDINGS_BUDGET_DENOM;
    let allocations = allocate_categories(&weights, findings_budget);

    // Pass 2: render each category within its allocated budget.
    let mut infos = Vec::new();
    for ((label, group), alloc) in groups.iter().zip(allocations.iter()) {
        let pre = out.len();
        let shown = render_category(out, label, group, alloc, compact);
        infos.push(CategoryRenderInfo {
            label: label.to_string(),
            chars_added: out.len() - pre,
            shown,
        });
    }
    infos
}

fn render_category(
    out: &mut String,
    label: &str,
    group: &[&Finding],
    alloc: &CategoryAllocation,
    compact: bool,
) -> usize {
    let mut sorted = group.to_vec();
    sorted.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| finding_location_key(left).cmp(&finding_location_key(right)))
    });
    let total = sorted.len();

    let critical_n = sorted
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high_n = sorted
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();
    let severity_note = if critical_n > 0 {
        format!("{critical_n} critical")
    } else if high_n > 0 {
        format!("{high_n} high")
    } else {
        format!("{total} findings")
    };
    let _ = writeln!(out, "## {label} ({severity_note})");
    out.push('\n');

    if alloc.chars == 0 {
        render_truncation_notice(out);
        return 0;
    }

    let max_per_category = if compact { 3 } else { 5 };
    let chars_start = out.len();
    let mut shown = 0;

    for finding in sorted.iter().copied().take(max_per_category) {
        let mut entry = String::new();
        render_finding_entry(&mut entry, finding, shown + 1, alloc.snippet_lines);
        if out.len() - chars_start + entry.len() > alloc.chars {
            if shown == 0 {
                render_truncation_notice(out);
                return 0;
            }
            let _ = writeln!(
                out,
                "*…and {} more {} findings*\n",
                total - shown,
                label.to_lowercase()
            );
            return shown;
        }
        out.push_str(&entry);
        shown += 1;
    }

    if total > shown {
        let _ = writeln!(
            out,
            "*…and {} more {} findings*\n",
            total - shown,
            label.to_lowercase()
        );
    }

    shown
}

pub(super) fn render_finding_entry(
    out: &mut String,
    finding: &Finding,
    index: usize,
    snippet_lines: usize,
) {
    let sev = finding.severity.label();
    let location = finding_location(finding);
    let loc_str = location
        .as_deref()
        .map(|l| format!(" — `{l}`"))
        .unwrap_or_default();

    let _ = writeln!(out, "**{index}. [{sev}] {}**{loc_str}", finding.title);

    if snippet_lines > 0
        && let Some(ev) = finding.evidence.first()
        && !ev.snippet.is_empty()
    {
        let snippet = crate::findings::redaction::human_evidence_snippet(finding, &ev.snippet)
            .lines()
            .take(snippet_lines)
            .collect::<Vec<_>>()
            .join("\n");
        let _ = writeln!(out, "```\n{snippet}\n```");
    }

    let _ = writeln!(out, "> **Confidence:** {}", finding.confidence.label());

    if !finding.description.trim().is_empty() {
        let _ = writeln!(
            out,
            "> **Context:** {}",
            first_sentence(&finding.description, 220)
        );
    }

    let _ = writeln!(out, "> **Fix:** {}", finding_recommendation(finding));

    if let Some(url) = &finding.docs_url {
        let _ = writeln!(out, "> **Docs:** {url}");
    }

    out.push('\n');
}

fn render_truncation_notice(out: &mut String) {
    let _ = writeln!(out, "\n*[Output truncated to stay within token budget]*");
}
