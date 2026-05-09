use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::rules::lookup_rule_metadata;
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

        if out.len() - start_len > budget_chars * 8 / 10 {
            let _ = writeln!(out, "\n*[Output truncated to stay within token budget]*");
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
        sorted.sort_by(|a, b| b.severity.cmp(&a.severity));

        let max_per_category = if compact { 3 } else { 5 };

        for (i, finding) in sorted.iter().copied().take(max_per_category).enumerate() {
            render_finding_entry(out, finding, i + 1);
        }

        if sorted.len() > max_per_category {
            let _ = writeln!(
                out,
                "*…and {} more {} findings*\n",
                sorted.len() - max_per_category,
                label.to_lowercase()
            );
        }
    }
}

pub(super) fn render_finding_entry(out: &mut String, finding: &Finding, index: usize) {
    let sev = finding.severity.label();

    let location = finding.evidence.first().map(|e| {
        let path = e.path.display().to_string();
        if e.line_start > 0 {
            format!("{path}:{}", e.line_start)
        } else {
            path
        }
    });

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

    let recommendation = lookup_rule_metadata(&finding.rule_id)
        .and_then(|m| m.recommendation)
        .or(if finding.description.is_empty() {
            None
        } else {
            Some(finding.description.as_str())
        });

    if let Some(rec) = recommendation {
        let _ = writeln!(out, "> **Fix:** {rec}");
    }

    if let Some(url) = &finding.docs_url {
        let _ = writeln!(out, "> **Docs:** {url}");
    }

    out.push('\n');
}
