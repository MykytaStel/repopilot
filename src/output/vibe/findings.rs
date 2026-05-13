use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::output::finding_helpers::{
    finding_location, finding_location_key, finding_recommendation,
};
use std::fmt::Write as FmtWrite;

type CategoryFilter = fn(&FindingCategory) -> bool;

pub(super) fn render_findings_by_category(
    out: &mut String,
    findings: &[&Finding],
    budget_chars: usize,
    compact: bool,
) {
    let category_order: &[(&str, CategoryFilter)] = &[
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

    let start_len = out.len();

    for (label, predicate) in category_order {
        let group: Vec<&Finding> = findings
            .iter()
            .copied()
            .filter(|f| predicate(&f.category))
            .collect();
        if group.is_empty() {
            continue;
        }

        if output_reached_budget(out, start_len, budget_chars) {
            render_truncation_notice(out);
            break;
        }

        let critical_n = group
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();
        let high_n = group
            .iter()
            .filter(|f| f.severity == Severity::High)
            .count();
        let total_n = group.len();

        let severity_note = if critical_n > 0 {
            format!("{critical_n} critical")
        } else if high_n > 0 {
            format!("{high_n} high")
        } else {
            format!("{total_n} findings")
        };

        let _ = writeln!(out, "## {label} ({severity_note})");
        out.push('\n');

        let mut sorted = group.clone();
        sorted.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| left.rule_id.cmp(&right.rule_id))
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| finding_location_key(left).cmp(&finding_location_key(right)))
        });

        let max_per_category = if compact { 3 } else { 5 };
        let mut rendered_count = 0;
        let mut truncated = false;

        for finding in sorted.iter().copied().take(max_per_category) {
            let mut entry = String::new();
            render_finding_entry(&mut entry, finding, rendered_count + 1);
            if rendered_count > 0
                && out.len().saturating_sub(start_len) + entry.len() > budget_chars
            {
                render_truncation_notice(out);
                truncated = true;
                break;
            }
            out.push_str(&entry);
            rendered_count += 1;
        }

        if truncated {
            break;
        }

        if sorted.len() > rendered_count {
            let _ = writeln!(
                out,
                "*…and {} more {} findings*\n",
                sorted.len() - rendered_count,
                label.to_lowercase()
            );
        }
    }
}

pub(super) fn render_finding_entry(out: &mut String, finding: &Finding, index: usize) {
    let sev = finding.severity.label();

    let location = finding_location(finding);

    let loc_str = location
        .as_deref()
        .map(|l| format!(" — `{l}`"))
        .unwrap_or_default();

    let _ = writeln!(out, "**{index}. [{sev}] {}**{loc_str}", finding.title);

    if let Some(ev) = finding.evidence.first() {
        if !ev.snippet.is_empty() {
            let snippet = ev.snippet.lines().take(3).collect::<Vec<_>>().join("\n");
            let _ = writeln!(out, "```\n{snippet}\n```");
        }
    }

    let _ = writeln!(out, "> **Fix:** {}", finding_recommendation(finding));

    if let Some(url) = &finding.docs_url {
        let _ = writeln!(out, "> **Docs:** {url}");
    }

    out.push('\n');
}

fn output_reached_budget(out: &str, start_len: usize, budget_chars: usize) -> bool {
    out.len().saturating_sub(start_len) >= budget_chars
}

fn render_truncation_notice(out: &mut String) {
    let _ = writeln!(out, "\n*[Output truncated to stay within token budget]*");
}
