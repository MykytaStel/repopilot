use crate::findings::types::Finding;
use crate::graph::CouplingGraph;
use crate::scan::facts::ScanFacts;
use crate::scan::types::LanguageSummary;
use crate::scan::types::{ScanCacheTelemetry, ScanMode, ScanSummary, ScanTimings};
use std::collections::HashMap;

pub(super) fn sort_findings(findings: &mut [Finding]) {
    crate::risk::sort_findings(findings);
}

pub(super) fn build_language_summary(languages: HashMap<String, usize>) -> Vec<LanguageSummary> {
    let mut summary: Vec<LanguageSummary> = languages
        .into_iter()
        .map(|(name, files_analyzed)| LanguageSummary {
            name,
            files_analyzed,
        })
        .collect();

    summary.sort_by(|left, right| {
        right
            .files_analyzed
            .cmp(&left.files_analyzed)
            .then_with(|| left.name.cmp(&right.name))
    });

    summary
}

pub(super) struct ScanSummaryParts {
    pub(super) mode: ScanMode,
    pub(super) base_ref: Option<String>,
    pub(super) changed_files_count: usize,
    pub(super) repo_level_rules_included: bool,
    pub(super) scan_duration_us: u64,
    pub(super) scan_timings: Option<ScanTimings>,
    pub(super) cache_telemetry: Option<ScanCacheTelemetry>,
    pub(super) coupling_graph: Option<CouplingGraph>,
}

pub(super) fn build_scan_summary(
    facts: ScanFacts,
    findings: Vec<Finding>,
    parts: ScanSummaryParts,
) -> ScanSummary {
    let health_score = ScanSummary::compute_health_score(&findings, facts.non_empty_lines);
    let visible_findings_count = findings.len();

    ScanSummary {
        root_path: facts.root_path,
        mode: parts.mode,
        base_ref: parts.base_ref,
        changed_files_count: parts.changed_files_count,
        repo_level_rules_included: parts.repo_level_rules_included,
        files_discovered: facts.files_discovered,
        files_analyzed: facts.files_analyzed,
        directories_count: facts.directories_count,
        non_empty_lines: facts.non_empty_lines,
        large_files_skipped: facts.large_files_skipped,
        files_skipped_low_signal: facts.files_skipped_low_signal,
        binary_files_skipped: facts.binary_files_skipped,
        skipped_bytes: facts.skipped_bytes,
        languages: facts.languages,
        detected_frameworks: facts.detected_frameworks,
        framework_projects: facts.framework_projects,
        react_native: facts.react_native,
        findings,
        coupling_graph: parts.coupling_graph,
        scan_duration_us: parts.scan_duration_us,
        health_score,
        visible_findings_count,
        hidden_suggestions_count: 0,
        hidden_suggestions: Vec::new(),
        visibility_profile: None,
        files_skipped_by_limit: facts.files_skipped_by_limit,
        files_skipped_repopilotignore: facts.files_skipped_repopilotignore,
        repopilotignore_path: facts.repopilotignore_path,
        scan_timings: parts.scan_timings,
        cache_telemetry: parts.cache_telemetry,
    }
}
