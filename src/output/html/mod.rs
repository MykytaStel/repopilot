mod assets;
mod document;
mod escape;
mod finding;
mod sections;

use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::output::report_stats::build_report_stats;
use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let stats = build_report_stats(summary);
    let cards = sections::render_summary_cards(summary, &stats);
    let risk_section = sections::render_risk_section(&stats);
    let top_rules_section = sections::render_top_rules_section(&stats);
    let languages_section = sections::render_languages_section(summary);
    let frameworks_section = sections::render_frameworks_section(summary);
    let filter_bar = sections::render_filter_bar(&stats);
    let findings_section = finding::render_findings_section(summary, |_| None);
    let path = summary.root_path.to_string_lossy();

    document::render_document(document::DocumentParts {
        path: &path,
        baseline_meta: "",
        cards: &cards,
        risk_section: &risk_section,
        top_rules_section: &top_rules_section,
        languages_section: &languages_section,
        frameworks_section: &frameworks_section,
        filter_bar: &filter_bar,
        findings_section: &findings_section,
    })
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    let stats = build_report_stats(&report.summary);
    let cards = sections::render_baseline_summary_cards(report, &stats);
    let risk_section = sections::render_risk_section(&stats);
    let top_rules_section = sections::render_top_rules_section(&stats);
    let languages_section = sections::render_languages_section(&report.summary);
    let frameworks_section = sections::render_frameworks_section(&report.summary);
    let filter_bar = sections::render_filter_bar(&stats);
    let findings_section = finding::render_findings_section(&report.summary, |index| {
        Some(report.finding_status(index).lowercase_label())
    });
    let baseline_meta = sections::render_baseline_meta(report, ci_gate);
    let path = report.summary.root_path.to_string_lossy();

    document::render_document(document::DocumentParts {
        path: &path,
        baseline_meta: &baseline_meta,
        cards: &cards,
        risk_section: &risk_section,
        top_rules_section: &top_rules_section,
        languages_section: &languages_section,
        frameworks_section: &frameworks_section,
        filter_bar: &filter_bar,
        findings_section: &findings_section,
    })
}
