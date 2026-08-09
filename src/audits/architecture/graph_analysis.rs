use super::graph_context::GraphAuditContext;
use super::graph_queries::GraphQueriesAudit;
use super::import_coupling::ImportCouplingAudit;
use crate::findings::types::Finding;
use crate::graph::v2::build_coupling_graph_snapshot;
use crate::graph::{CouplingGraph, build_coupling_graph_with_resolution};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::Path;

pub(crate) struct GraphAnalysisResult {
    pub(crate) coupling_findings: Vec<Finding>,
    pub(crate) query_findings: Vec<Finding>,
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

    GraphAnalysisResult {
        coupling_findings,
        query_findings,
        graph,
    }
}
