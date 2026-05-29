use crate::findings::quality::SignalQualitySummary;
use crate::findings::types::Finding;
use crate::graph::CouplingGraph;
use crate::graph::context::{
    ContextGraphCacheInfo, ContextGraphSummary, RepoContextGraph, summarize_context_graph,
};
use crate::scan::facts::ScanFacts;
use crate::scan::types::LanguageSummary;
use crate::scan::types::{
    ScanArtifacts, ScanCacheTelemetry, ScanDiagnostic, ScanMetadata, ScanMetrics, ScanMode,
    ScanSummary, ScanTimings,
};
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
    pub(super) context_graph_summary: Option<ContextGraphSummary>,
    pub(super) context_graph_cache: Option<ContextGraphCacheInfo>,
    pub(super) diagnostics: Vec<ScanDiagnostic>,
    pub(super) signal_quality: SignalQualitySummary,
}

pub(super) fn build_scan_summary(
    facts: ScanFacts,
    findings: Vec<Finding>,
    parts: ScanSummaryParts,
) -> ScanSummary {
    let health_score = ScanSummary::compute_health_score(&findings, facts.non_empty_lines);
    let raw_findings_count = findings.len();
    let visible_findings_count = findings.len();
    let signal_quality = parts.signal_quality;
    let context_graph_summary =
        parts
            .context_graph_summary
            .or_else(|| match parts.coupling_graph.as_ref() {
                Some(coupling_graph) => {
                    let graph = RepoContextGraph::from_scan_facts(
                        &facts,
                        &facts.root_path,
                        coupling_graph.clone(),
                    );
                    Some(summarize_context_graph(&graph, &findings, &[]))
                }
                None => None,
            });

    ScanSummary {
        metadata: ScanMetadata {
            root_path: facts.root_path,
            mode: parts.mode,
            base_ref: parts.base_ref,
            repo_level_rules_included: parts.repo_level_rules_included,
            scan_duration_us: parts.scan_duration_us,
            scan_timings: parts.scan_timings,
            cache_telemetry: parts.cache_telemetry,
            local_feedback: None,
            visibility_profile: None,
            repopilotignore_path: facts.repopilotignore_path,
        },
        metrics: ScanMetrics {
            files_discovered: facts.files_discovered,
            files_analyzed: facts.files_analyzed,
            directories_count: facts.directories_count,
            non_empty_lines: facts.non_empty_lines,
            large_files_skipped: facts.large_files_skipped,
            files_skipped_low_signal: facts.files_skipped_low_signal,
            binary_files_skipped: facts.binary_files_skipped,
            skipped_bytes: facts.skipped_bytes,
            files_skipped_by_limit: facts.files_skipped_by_limit,
            files_skipped_repopilotignore: facts.files_skipped_repopilotignore,
            changed_files_count: parts.changed_files_count,
            health_score,
            raw_findings_count,
            visible_findings_count,
            hidden_suggestions_count: 0,
            languages: facts.languages,
        },
        artifacts: ScanArtifacts {
            findings,
            detected_frameworks: facts.detected_frameworks,
            framework_projects: facts.framework_projects,
            react_native: facts.react_native,
            coupling_graph: parts.coupling_graph,
            context_graph_summary,
            context_graph_cache: parts.context_graph_cache,
            hidden_suggestions: Vec::new(),
            diagnostics: parts.diagnostics,
            raw_signal_quality: signal_quality.clone(),
            visible_signal_quality: signal_quality.clone(),
            signal_quality,
        },
    }
}
