//! Whole-repo architecture rules that query the import graph: dead modules,
//! test leaks into production, declared-layer violations, and package-boundary
//! violations. Layer violations are strictly opt-in (`[[architecture.layers]]`).
//! Package-boundary violations auto-enable on a detected npm/pnpm/Cargo/Go
//! workspace and can also be driven explicitly by `[architecture] package_roots`;
//! with neither a workspace nor config the rule is silent.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analysis::{ArchitectureClassifier, ArchitectureContext, FileRole, LanguageFamily};
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::graph::v2::{GraphNodeId, build_coupling_graph_snapshot};
use crate::graph::{CouplingGraph, ImportResolutionStats};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};

mod edge_evidence;
mod layers;
mod packages;

#[cfg(test)]
mod tests;

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
    pub fn audit(
        &self,
        facts: &ScanFacts,
        config: &ScanConfig,
        graph: &CouplingGraph,
        resolution: &ImportResolutionStats,
        root: &Path,
    ) -> Vec<Finding> {
        let classifier = ArchitectureClassifier::new(&config.module_mappings);

        let mut file_context = HashMap::new();
        let mut facts_by_path = HashMap::new();
        for file in &facts.files {
            let context = classifier.classify(file);
            let abs_path = root.join(&file.path);
            file_context.insert(abs_path.clone(), context.clone());
            file_context.insert(file.path.clone(), context);
            facts_by_path.insert(abs_path, file);
            facts_by_path.insert(file.path.clone(), file);
        }

        let known_files: HashSet<PathBuf> = facts
            .files
            .iter()
            .map(|f| crate::graph::resolver::normalize_path(&f.path))
            .collect();

        let (snapshot, path_by_id) = build_coupling_graph_snapshot(graph);

        let mut node_info: HashMap<GraphNodeId, NodeInfo> = HashMap::new();
        for node in &snapshot.nodes {
            if let Some(path) = path_by_id.get(&node.id)
                && let Some(context) = file_context.get(path)
                && let Some(file_facts) = facts_by_path.get(path)
            {
                node_info.insert(
                    node.id.clone(),
                    NodeInfo {
                        relative: relative_path(path, root),
                        context: context.clone(),
                        facts: Some(file_facts),
                    },
                );
            }
        }

        let mut fan_in: HashMap<GraphNodeId, usize> = HashMap::new();
        for edge in &snapshot.edges {
            *fan_in.entry(edge.to.clone()).or_insert(0) += 1;
        }

        let mut findings = Vec::new();

        for node in &snapshot.nodes {
            if let Some(info) = node_info.get(&node.id)
                && let Some(finding) =
                    dead_module_finding(info, fan_in.get(&node.id).copied(), resolution)
            {
                findings.push(finding);
            }
        }

        let layer_index = LayerIndex::from_config(config);
        let detected_packages = crate::scan::workspace::detect_workspace_packages(root);
        let package_index = PackageIndex::new(config, &detected_packages, root);

        let mut reported_edges = HashSet::new();
        for edge in &snapshot.edges {
            if !reported_edges.insert((edge.from.clone(), edge.to.clone())) {
                continue;
            }
            let (Some(source), Some(target)) = (node_info.get(&edge.from), node_info.get(&edge.to))
            else {
                continue;
            };

            if let Some(finding) = test_leak_finding(source, target, root, &known_files) {
                findings.push(finding);
            }
            if let Some(finding) = layer_index.violation_finding(source, target, root, &known_files)
            {
                findings.push(finding);
            }
            if let Some(finding) =
                package_index.violation_finding(source, target, root, &known_files)
            {
                findings.push(finding);
            }
        }

        findings
    }
}

/// A production file that nothing imports and that is not an entrypoint or a
/// package's public API surface is likely dead code.
///
/// "Nothing imports this file" is an absence claim, so it is only as good as
/// the import graph: an unresolved relative import whose final segment matches
/// this file's name could well be the missing importer (skip entirely), and
/// any other unresolved relative import still means the graph is incomplete
/// (report at `Medium` instead of the registry's `High`).
fn dead_module_finding(
    info: &NodeInfo,
    fan_in: Option<usize>,
    resolution: &ImportResolutionStats,
) -> Option<Finding> {
    let ctx = &info.context;
    // "Dead module" only means something for files that participate in the
    // import graph. Docs, config, stylesheets, lockfiles, images, and shell
    // scripts (Markup / Shell / Unknown families) are never "imported", so they
    // always have fan_in=0 and would otherwise all be flagged — e.g. every
    // `.claude/*.md`, `*.css`, `*.json`, or lockfile in a repo. Restrict to
    // languages the resolver actually wires up.
    let is_importable_code = matches!(
        ctx.language_family,
        LanguageFamily::CurlyBrace | LanguageFamily::Python | LanguageFamily::Go
    );
    // A file that carries its own tests is exercised by the suite, and its only
    // importer is often a `#[cfg(test)] mod ...;` declaration, whose edge is
    // intentionally excluded from the production import graph. Treating such a
    // file as dead would be a false positive (e.g. a `proptests.rs` reached
    // only under `cfg(test)`), so exempt files with inline tests.
    let has_inline_tests = info.facts.is_some_and(|facts| facts.has_inline_tests);
    if ctx.file_role != FileRole::Production
        || !is_importable_code
        || ctx.is_entrypoint
        || ctx.is_public_api
        || fan_in.unwrap_or(0) != 0
        || has_inline_tests
    {
        return None;
    }

    let stem = info
        .relative
        .file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_default();
    if resolution.could_target_stem(&stem) {
        return None;
    }

    let mut snippet = "fan_in=0, role=Production, entrypoint=false".to_string();
    let confidence = if resolution.is_empty() {
        Confidence::High
    } else {
        snippet.push_str(&format!(
            " ({} unresolved relative import(s) in the repository — the import graph may be incomplete)",
            resolution.total()
        ));
        Confidence::Medium
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
