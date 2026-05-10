use crate::findings::types::FindingCategory;
use crate::output::color;
use crate::output::report_stats::{NamedCount, ReportStats, severity_order};

pub(crate) fn console_severity_counts_text(stats: &ReportStats) -> String {
    let parts = severity_order()
        .iter()
        .filter_map(|severity| {
            let count = stats.severity_count(*severity);
            (count > 0).then(|| color::severity_count(*severity, count))
        })
        .collect::<Vec<_>>();

    join_or_none(parts, " | ")
}

pub(crate) fn markdown_severity_counts_text(stats: &ReportStats) -> String {
    let parts = severity_order()
        .iter()
        .filter_map(|severity| {
            let count = stats.severity_count(*severity);
            (count > 0).then(|| format!("{} {}", count, severity.lowercase_label()))
        })
        .collect::<Vec<_>>();

    join_or_none(parts, ", ")
}

pub(crate) fn named_counts_text(counts: &[NamedCount]) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    counts
        .iter()
        .map(|count| format!("{} ({})", count.label, count.count))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn category_title(category: &FindingCategory) -> &'static str {
    match category {
        FindingCategory::Security => "Security",
        FindingCategory::Architecture => "Architecture",
        FindingCategory::Framework => "Framework",
        FindingCategory::CodeQuality => "Code Quality",
        FindingCategory::Testing => "Testing",
    }
}

pub(crate) fn category_label_rank(label: &str) -> usize {
    match label {
        "security" => 0,
        "architecture" => 1,
        "framework" => 2,
        "code-quality" => 3,
        "testing" => 4,
        _ => usize::MAX,
    }
}

pub(crate) fn first_sentence(text: &str, max_len: usize) -> String {
    let sentence = text.split(". ").next().unwrap_or(text);
    if sentence.len() <= max_len {
        sentence.to_string()
    } else {
        format!("{}...", &sentence[..max_len])
    }
}

pub(crate) fn tristate_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}

fn join_or_none(parts: Vec<String>, separator: &str) -> String {
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(separator)
    }
}
