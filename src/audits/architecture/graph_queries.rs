use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analysis::{ArchitectureClassifier, ArchitectureContext, FileRole, ModuleKind};
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::graph::CouplingGraph;
use crate::graph::v2::{GraphNodeId, build_coupling_graph_snapshot};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub struct GraphQueriesAudit;

impl GraphQueriesAudit {
    pub fn audit(
        &self,
        facts: &ScanFacts,
        config: &ScanConfig,
        graph: &CouplingGraph,
        root: &Path,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        let classifier = ArchitectureClassifier::new(&config.module_mappings);

        let mut file_context = HashMap::new();
        for file in &facts.files {
            let context = classifier.classify(file);
            let absolute_path = if file.path.is_absolute() {
                file.path.clone()
            } else {
                root.join(&file.path)
            };
            file_context.insert(absolute_path.clone(), context.clone());
            file_context.insert(file.path.clone(), context);
        }

        let (snapshot, path_by_id) = build_coupling_graph_snapshot(graph);

        // Map from NodeId to PathBuf and Context
        let mut node_ctx: HashMap<GraphNodeId, (PathBuf, ArchitectureContext)> = HashMap::new();
        for node in &snapshot.nodes {
            if let Some(path) = path_by_id.get(&node.id)
                && let Some(ctx) = file_context.get(path)
            {
                node_ctx.insert(node.id.clone(), (path.clone(), ctx.clone()));
            }
        }

        // Compute fan_in per node
        let mut fan_in: HashMap<GraphNodeId, usize> = HashMap::new();
        for edge in &snapshot.edges {
            *fan_in.entry(edge.to.clone()).or_insert(0) += 1;
        }

        // Rule: Dead Modules
        for node in &snapshot.nodes {
            if let Some((path, ctx)) = node_ctx.get(&node.id)
                && ctx.file_role == FileRole::Production
                && !ctx.is_entrypoint
                && fan_in.get(&node.id).copied().unwrap_or(0) == 0
            {
                findings.push(Finding {
                        id: String::new(),
                        rule_id: "architecture.dead-module".to_string(),
                        recommendation: Finding::recommendation_for_rule_id(
                            "architecture.dead-module",
                        ),
                        title: "Dead module detected".to_string(),
                        description: "This production file is not imported by any other project file and is not a known entrypoint.".to_string(),
                        category: FindingCategory::Architecture,
                        severity: Severity::Low,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: relative_path(path, root),
                            line_start: 1,
                            line_end: None,
                            snippet: "fan_in=0, role=Production, entrypoint=false".to_string(),
                        }],
                        workspace_package: None,
                        docs_url: None,
                        provenance: Default::default(),
                        risk: Default::default(),
                    });
            }
        }

        // Edges: Layer violations, test leaks, package boundaries
        let mut reported_edges = HashSet::new();

        for edge in &snapshot.edges {
            // Avoid duplicate edge findings (e.g. if the graph has multiple edges between the same two files)
            if !reported_edges.insert((edge.from.clone(), edge.to.clone())) {
                continue;
            }

            let source_info = node_ctx.get(&edge.from);
            let target_info = node_ctx.get(&edge.to);

            if let (Some((source_path, source_ctx)), Some((target_path, target_ctx))) =
                (source_info, target_info)
            {
                let rel_source = relative_path(source_path, root);
                let rel_target = relative_path(target_path, root);

                // Rule: Test Leak
                let is_pure_test = (target_ctx.file_role == FileRole::Test
                    || target_ctx.file_role == FileRole::Fixture)
                    && {
                        let path_str = target_path.to_string_lossy().to_lowercase();
                        path_str.contains("/tests/")
                            || path_str.contains("\\tests\\")
                            || path_str.contains("/fixtures/")
                            || path_str.contains("\\fixtures\\")
                            || path_str.contains("/__tests__/")
                            || path_str.contains("\\__tests__\\")
                            || path_str.contains(".test.")
                            || path_str.contains(".spec.")
                            || path_str.ends_with("_test.rs")
                            || path_str.ends_with("_tests.rs")
                            || path_str.ends_with("_test.go")
                            || path_str.ends_with("_test.py")
                            || path_str.starts_with("tests/")
                            || path_str.starts_with("tests\\")
                            || path_str.starts_with("fixtures/")
                            || path_str.starts_with("fixtures\\")
                    }
                    && !target_path.to_string_lossy().ends_with("without_test.rs");

                if source_ctx.file_role == FileRole::Production && is_pure_test {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "architecture.test-leak".to_string(),
                        recommendation: Finding::recommendation_for_rule_id(
                            "architecture.test-leak",
                        ),
                        title: "Test code leaked into production".to_string(),
                        description: format!(
                            "Production file imports a {} file.",
                            if target_ctx.file_role == FileRole::Test {
                                "Test"
                            } else {
                                "Fixture"
                            }
                        ),
                        category: FindingCategory::Architecture,
                        severity: Severity::High,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: rel_source.clone(),
                            line_start: 1,
                            line_end: None,
                            snippet: format!("Imports: {}", rel_target.display()),
                        }],
                        workspace_package: None,
                        docs_url: None,
                        provenance: Default::default(),
                        risk: Default::default(),
                    });
                }

                // Rule: Layer Violation
                let is_layer_violation = (source_ctx.module_kind == ModuleKind::Domain
                    && (target_ctx.module_kind == ModuleKind::Ui
                        || target_ctx.module_kind == ModuleKind::Infrastructure))
                    || (source_ctx.module_kind == ModuleKind::Infrastructure
                        && target_ctx.module_kind == ModuleKind::Ui);

                if is_layer_violation {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "architecture.layer-violation".to_string(),
                        recommendation: Finding::recommendation_for_rule_id(
                            "architecture.layer-violation",
                        ),
                        title: "Layer violation detected".to_string(),
                        description: format!(
                            "A {:?} layer module imports a {:?} layer module.",
                            source_ctx.module_kind, target_ctx.module_kind
                        ),
                        category: FindingCategory::Architecture,
                        severity: Severity::Medium,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: rel_source.clone(),
                            line_start: 1,
                            line_end: None,
                            snippet: format!("Imports: {}", rel_target.display()),
                        }],
                        workspace_package: None,
                        docs_url: None,
                        provenance: Default::default(),
                        risk: Default::default(),
                    });
                }

                // Rule: Package Boundary Violation
                // We define a boundary violation as importing a file that is NOT a public API,
                // AND that file belongs to a different directory module.
                if !target_ctx.is_public_api {
                    let source_dir = source_path.parent();
                    let target_dir = target_path.parent();

                    if source_dir != target_dir && target_dir.is_some() {
                        // Very simple heuristic: if it's not the same directory, it crossed a boundary into private internals.
                        findings.push(Finding {
                            id: String::new(),
                            rule_id: "architecture.package-boundary-violation".to_string(),
                            recommendation: Finding::recommendation_for_rule_id("architecture.package-boundary-violation"),
                            title: "Package boundary violation".to_string(),
                            description: "A file imports a private module from another package/feature instead of using its public API.".to_string(),
                            category: FindingCategory::Architecture,
                            severity: Severity::Medium,
                            confidence: Default::default(),
                            evidence: vec![Evidence {
                                path: rel_source.clone(),
                                line_start: 1,
                                line_end: None,
                                snippet: format!("Imports internal file: {}", rel_target.display()),
                            }],
                            workspace_package: None,
                            docs_url: None,
                            provenance: Default::default(),
                            risk: Default::default(),
                        });
                    }
                }
            }
        }

        findings
    }
}

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
