//! Whole-repo architecture rules that query the import graph: dead modules,
//! test leaks into production, declared-layer violations, and package-boundary
//! violations. Layer violations are strictly opt-in (`[[architecture.layers]]`).
//! Package-boundary violations auto-enable on a detected npm/pnpm/Cargo/Go
//! workspace and can also be driven explicitly by `[architecture] package_roots`;
//! with neither a workspace nor config the rule is silent.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::analysis::{ArchitectureClassifier, ArchitectureContext, FileRole};
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::graph::ImportResolutionStats;
use crate::graph::v2::{GraphClaim, GraphReadiness, graph_capabilities, graph_readiness};
use crate::scan::facts::FileFacts;

mod audit_helpers;
mod edge_evidence;
mod entrypoints;
mod layers;
mod packages;

#[cfg(test)]
mod tests;

use super::graph_context::GraphAuditContext;
use audit_helpers::{
    build_file_indexes, build_node_info, dead_module_findings, edge_findings, fan_in_by_target,
};
use edge_evidence::edge_evidence;
use layers::LayerIndex;
use packages::PackageIndex;

pub struct GraphQueriesAudit;

pub(crate) struct NodeInfo<'a> {
    pub relative: PathBuf,
    pub context: ArchitectureContext,
    pub facts: Option<&'a FileFacts>,
}

impl GraphQueriesAudit {
    pub(crate) fn audit(&self, analysis: &GraphAuditContext<'_>) -> Vec<Finding> {
        let classifier = ArchitectureClassifier::new(&analysis.config.module_mappings);
        let (file_context, facts_by_path, known_files) =
            build_file_indexes(analysis.facts, analysis.root, &classifier);
        let node_info = build_node_info(
            analysis.snapshot,
            analysis.path_by_id,
            analysis.root,
            &file_context,
            &facts_by_path,
        );
        let fan_in = fan_in_by_target(&analysis.snapshot.edges);
        let capabilities = graph_capabilities(analysis.snapshot);
        let mut findings = dead_module_findings(
            analysis.snapshot,
            &node_info,
            &fan_in,
            &capabilities,
            analysis.resolution,
        );

        let layer_index = LayerIndex::from_config(analysis.config);
        let detected_packages = crate::scan::workspace::detect_workspace_packages(analysis.root);
        let package_index = PackageIndex::new(analysis.config, &detected_packages, analysis.root);
        findings.extend(edge_findings(
            analysis.snapshot,
            &node_info,
            analysis.root,
            &known_files,
            &layer_index,
            &package_index,
        ));
        findings
    }
}

// Emit dead-module evidence only when the import graph is ready for an absence
// claim; unresolved internal imports lower confidence or suppress the claim.
fn dead_module_finding(
    info: &NodeInfo,
    fan_in: Option<usize>,
    readiness: GraphReadiness,
) -> Option<Finding> {
    let ctx = &info.context;
    // "Nothing imports this" is only evidence when an import could have become
    // an edge. Docs, stylesheets, and lockfiles are never imported, and neither
    // are C#, Swift, PHP, or C++ sources as far as this graph is concerned:
    // they reference types through namespaces and headers the resolver does not
    // map to files. Java and Kotlin do produce import edges, but same-package
    // references need no import and are invisible here. Ask the resolver which
    // languages support absence claims so this cannot drift from graph semantics.
    let supports_absence = crate::graph::resolver::supports_file_absence_claims(&info.relative);
    // A file that carries its own tests is exercised by the suite, and its only
    // importer is often a `#[cfg(test)] mod ...;` declaration, whose edge is
    // intentionally excluded from the production import graph. Treating such a
    // file as dead would be a false positive (e.g. a `proptests.rs` reached
    // only under `cfg(test)`), so exempt files with inline tests.
    let has_inline_tests = info.facts.is_some_and(|facts| facts.has_inline_tests);
    if ctx.file_role != FileRole::Production
        || !supports_absence
        || ctx.is_entrypoint
        || ctx.is_public_api
        || fan_in.unwrap_or(0) != 0
        || has_inline_tests
        // A tool config, build script, framework-autoloaded module, routed
        // file, browser entry, or documentation example is reached without an
        // import, so its fan-in is zero in every healthy repository.
        || entrypoints::reached_without_import(
            &info.relative,
            info.facts.is_some_and(|facts| facts.in_executable_package),
        )
        .is_some()
    {
        return None;
    }

    if matches!(readiness, GraphReadiness::Unavailable { .. }) {
        return None;
    }

    let mut snippet = "fan_in=0, role=Production, entrypoint=false".to_string();
    // `Confidence::Medium` is the "unset, use the registry default" sentinel in
    // `populate_rule_metadata`, so a contextual demotion must use `Low` to
    // survive — otherwise the High default is restored and the demotion is lost.
    let confidence = if let GraphReadiness::Limited {
        unresolved_internal,
    } = readiness
    {
        snippet.push_str(&format!(
            " ({} unresolved internal import(s) in the repository — the import graph may be incomplete)",
            unresolved_internal
        ));
        Confidence::Low
    } else {
        Confidence::High
    };

    let mut finding = architecture_finding(
        "architecture.dead-module",
        "Dead module detected",
        "This production file is not imported by any other project file and is not a known entrypoint.".to_string(),
        Evidence {
            path: info.relative.clone(),
            line_start: 1,
            line_end: None,
            snippet,
        },
    );
    finding.confidence = confidence;
    Some(finding)
}

fn target_absence_readiness(
    info: &NodeInfo,
    capabilities: &crate::graph::v2::GraphCapabilities,
    resolution: &ImportResolutionStats,
) -> GraphReadiness {
    let stem = info
        .relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let extension = info
        .relative
        .extension()
        .and_then(|extension| extension.to_str());
    graph_readiness(
        capabilities,
        resolution,
        GraphClaim::TargetAbsence { stem, extension },
    )
}

/// A production file importing a test or fixture file leaks test-only code into
/// the shipped build.
fn test_leak_finding(
    source: &NodeInfo,
    target: &NodeInfo,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<Finding> {
    if source.context.file_role != FileRole::Production {
        return None;
    }
    let kind = match target.context.file_role {
        FileRole::Test => "test",
        FileRole::Fixture => "fixture",
        _ => return None,
    };

    let (line_start, line_end) = if let Some(facts) = source.facts {
        edge_evidence(facts, &target.relative, root, known_files)
    } else {
        (1, None)
    };

    Some(architecture_finding(
        "architecture.test-leak",
        "Test code leaked into production",
        format!("Production file imports a {kind} file."),
        Evidence {
            path: source.relative.clone(),
            line_start,
            line_end,
            snippet: format!("Imports: {}", target.relative.display()),
        },
    ))
}

/// Shared constructor for architecture findings. Severity and confidence are
/// left at the `Info`/`Medium` sentinels so the rule registry owns them via
/// `populate_rule_metadata` (single source of truth — no inline severity here).
pub(crate) fn architecture_finding(
    rule_id: &str,
    title: &str,
    description: String,
    evidence: Evidence,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        title: title.to_string(),
        description,
        category: FindingCategory::Architecture,
        severity: Severity::Info,
        confidence: Default::default(),
        evidence: vec![evidence],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

pub(crate) fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
