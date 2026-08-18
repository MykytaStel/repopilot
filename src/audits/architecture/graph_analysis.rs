use super::graph_context::GraphAuditContext;
use super::graph_queries::GraphQueriesAudit;
use super::import_coupling::ImportCouplingAudit;
use super::unresolved_local_imports::analyze_unresolved_local_imports;
use crate::findings::types::Finding;
use crate::graph::v2::build_coupling_graph_snapshot;
use crate::graph::{CouplingGraph, build_coupling_graph_with_resolution};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use crate::scan::types::ScanDiagnostic;
use std::path::Path;

pub(crate) struct GraphAnalysisResult {
    pub(crate) coupling_findings: Vec<Finding>,
    pub(crate) query_findings: Vec<Finding>,
    pub(crate) broken_import_findings: Vec<Finding>,
    pub(crate) diagnostics: Vec<ScanDiagnostic>,
    pub(crate) graph: CouplingGraph,
}

pub(crate) fn run_graph_analysis(
    facts: &ScanFacts,
    config: &ScanConfig,
    root: &Path,
) -> GraphAnalysisResult {
    let (graph, resolution) = build_coupling_graph_with_resolution(facts, root);
    let (snapshot, path_by_id) = build_coupling_graph_snapshot(&graph);
    let context = GraphAuditContext {
        facts,
        config,
        root,
        graph: &graph,
        resolution: &resolution,
        snapshot: &snapshot,
        path_by_id: &path_by_id,
    };
    let coupling_findings = ImportCouplingAudit.audit(&context);
    let query_findings = GraphQueriesAudit.audit(&context);
    let broken_import_analysis = analyze_unresolved_local_imports(&context);

    GraphAnalysisResult {
        coupling_findings,
        query_findings,
        broken_import_findings: broken_import_analysis.findings,
        diagnostics: broken_import_analysis.diagnostics,
        graph,
    }
}
