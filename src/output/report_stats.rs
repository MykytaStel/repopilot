use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::scan::types::ScanSummary;
use counts::{CountWithSeverity, category_counts_from_map, increment, top_counts_from_map};
use std::collections::BTreeMap;

pub(crate) const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

mod counts;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportStats {
    pub total_findings: usize,
    pub severity_counts: [usize; 5],
    pub category_counts: Vec<NamedCount>,
    pub top_rules: Vec<NamedCount>,
    pub top_paths: Vec<NamedCount>,
    pub top_packages: Vec<NamedCount>,
    pub finding_density: f64,
    pub health_score: u8,
    pub risk_label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedCount {
    pub label: String,
    pub count: usize,
    pub severity: Option<Severity>,
}

impl ReportStats {
    pub fn severity_count(&self, severity: Severity) -> usize {
        self.severity_counts[severity_index(severity)]
    }
}

pub(crate) fn build_report_stats(summary: &ScanSummary) -> ReportStats {
    let mut severity_counts = [0usize; 5];
    let mut category_counts: BTreeMap<&'static str, CountWithSeverity> = BTreeMap::new();
    let mut rule_counts: BTreeMap<String, CountWithSeverity> = BTreeMap::new();
    let mut path_counts: BTreeMap<String, CountWithSeverity> = BTreeMap::new();
    let mut package_counts: BTreeMap<String, CountWithSeverity> = BTreeMap::new();

    for finding in &summary.findings {
        severity_counts[severity_index(finding.severity)] += 1;
        increment(
            category_counts.entry(finding.category.label()).or_default(),
            finding.severity,
        );
        increment(
            rule_counts.entry(finding.rule_id.clone()).or_default(),
            finding.severity,
        );

        if let Some(evidence) = finding.evidence.first() {
            increment(
                path_counts
                    .entry(evidence.path.display().to_string())
                    .or_default(),
                finding.severity,
            );
        }

        if let Some(package) = &finding.workspace_package {
            increment(
                package_counts.entry(package.clone()).or_default(),
                finding.severity,
            );
        }
    }

    let total_findings = summary.findings.len();
    let finding_density = if summary.lines_of_code > 0 {
        total_findings as f64 * 1000.0 / summary.lines_of_code as f64
    } else {
        0.0
    };

    ReportStats {
        total_findings,
        severity_counts,
        category_counts: category_counts_from_map(category_counts),
        top_rules: top_counts_from_map(rule_counts, 10),
        top_paths: top_counts_from_map(path_counts, 10),
        top_packages: top_counts_from_map(package_counts, 10),
        finding_density,
        health_score: summary.health_score,
        risk_label: risk_label_for_counts(&severity_counts, total_findings),
    }
}

pub(crate) fn risk_label_for_findings(findings: &[&Finding]) -> &'static str {
    let mut severity_counts = [0usize; 5];
    for finding in findings {
        severity_counts[severity_index(finding.severity)] += 1;
    }
    risk_label_for_counts(&severity_counts, findings.len())
}

pub(crate) fn severity_index(severity: Severity) -> usize {
    match severity {
        Severity::Critical => 0,
        Severity::High => 1,
        Severity::Medium => 2,
        Severity::Low => 3,
        Severity::Info => 4,
    }
}

pub(crate) fn severity_order() -> [Severity; 5] {
    [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
        Severity::Info,
    ]
}

pub(crate) fn category_order() -> [FindingCategory; 5] {
    [
        FindingCategory::Security,
        FindingCategory::Architecture,
        FindingCategory::Framework,
        FindingCategory::CodeQuality,
        FindingCategory::Testing,
    ]
}

pub(crate) fn sorted_findings(findings: &[Finding]) -> Vec<&Finding> {
    let mut sorted = findings.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| crate::risk::compare_findings(left, right));
    sorted
}

pub(crate) fn findings_for_category<'a>(
    findings: &'a [Finding],
    category: &FindingCategory,
) -> Vec<&'a Finding> {
    sorted_findings(findings)
        .into_iter()
        .filter(|finding| &finding.category == category)
        .collect()
}

pub(crate) fn findings_for_rule<'a>(
    findings: &'a [&'a Finding],
    rule_id: &str,
) -> Vec<&'a Finding> {
    findings
        .iter()
        .copied()
        .filter(|finding| finding.rule_id == rule_id)
        .collect()
}

pub(crate) fn rule_ids_for_findings(findings: &[&Finding]) -> Vec<String> {
    let mut rules = findings
        .iter()
        .map(|finding| finding.rule_id.clone())
        .collect::<Vec<_>>();
    rules.sort();
    rules.dedup();
    rules
}

pub(crate) fn first_location(finding: &Finding) -> Option<String> {
    finding.evidence.first().map(|evidence| {
        if evidence.line_start > 0 {
            format!("{}:{}", evidence.path.display(), evidence.line_start)
        } else {
            evidence.path.display().to_string()
        }
    })
}

pub(crate) fn risk_label_for_counts(
    severity_counts: &[usize; 5],
    total_findings: usize,
) -> &'static str {
    if severity_counts[severity_index(Severity::Critical)] > 0 {
        "High"
    } else if severity_counts[severity_index(Severity::High)] >= 3 {
        "Elevated"
    } else if severity_counts[severity_index(Severity::High)] > 0
        || severity_counts[severity_index(Severity::Medium)] >= 10
    {
        "Moderate"
    } else if total_findings > 0 {
        "Low"
    } else {
        "Clean"
    }
}
