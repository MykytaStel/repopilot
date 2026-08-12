use super::*;
use crate::audits::architecture::graph_context::GraphAuditContext;
use crate::findings::types::{Confidence, Severity};
use crate::graph::v2::build_coupling_graph_snapshot;
use crate::graph::{CouplingGraph, ImportResolutionStats};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::types::DiagnosticSeverity;
use std::fs;
use std::path::{Path, PathBuf};

fn analyze(
    root: &Path,
    facts: &ScanFacts,
    resolution: &ImportResolutionStats,
) -> UnresolvedLocalImportAnalysis {
    let graph = CouplingGraph::default();
    let (snapshot, path_by_id) = build_coupling_graph_snapshot(&graph);
    let context = GraphAuditContext {
        facts,
        config: &ScanConfig::default(),
        root,
        graph: &graph,
        resolution,
        snapshot: &snapshot,
        path_by_id: &path_by_id,
    };
    analyze_unresolved_local_imports(&context)
}

fn facts(path: PathBuf, language: &str, content: &str, imports: &[&str]) -> ScanFacts {
    ScanFacts {
        files: vec![FileFacts {
            path,
            language: Some(language.to_string()),
            content: Some(content.to_string()),
            imports: imports.iter().map(|value| (*value).to_string()).collect(),
            ..FileFacts::default()
        }],
        ..ScanFacts::default()
    }
}

#[test]
fn missing_explicit_typescript_target_emits_high_confidence_finding_at_import_line() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("src/app.ts");
    let facts = facts(
        source.clone(),
        "TypeScript",
        "// application entry\nimport { run } from \"./missing.ts\";\nrun();\n",
        &["./missing.ts"],
    );
    let mut resolution = ImportResolutionStats::default();
    resolution.record_classified(&source, "./missing.ts", root);

    let result = analyze(root, &facts, &resolution);

    assert_eq!(result.findings.len(), 1);
    let finding = &result.findings[0];
    assert_eq!(finding.rule_id, "architecture.unresolved-local-import");
    assert_eq!(finding.severity, Severity::High);
    assert_eq!(finding.confidence, Confidence::High);
    assert_eq!(finding.evidence[0].path, PathBuf::from("src/app.ts"));
    assert_eq!(finding.evidence[0].line_start, 2);
    assert!(finding.evidence[0].snippet.contains("./missing.ts"));
    assert!(result.diagnostics.is_empty());
}

#[test]
fn missing_explicit_python_relative_module_emits_finding() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("pkg/app.py");
    let facts = facts(
        source.clone(),
        "Python",
        "from .missing import run\n\nrun()\n",
        &[".missing"],
    );
    let mut resolution = ImportResolutionStats::default();
    resolution.record_classified(&source, ".missing", root);
    resolution.record_classified(&source, ".missing.run", root);

    let result = analyze(root, &facts, &resolution);

    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].evidence[0].line_start, 1);
    assert!(result.findings[0].description.contains(".missing"));
}

#[test]
fn candidate_present_on_disk_but_absent_from_scan_is_not_reported() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/present.ts"), "export const value = 1;\n").unwrap();
    let source = root.join("src/app.ts");
    let facts = facts(
        source.clone(),
        "TypeScript",
        "import { value } from \"./present.ts\";\n",
        &["./present.ts"],
    );
    let mut resolution = ImportResolutionStats::default();
    resolution.record_classified(&source, "./present.ts", root);

    let result = analyze(root, &facts, &resolution);

    assert!(result.findings.is_empty());
}

#[test]
fn ambiguous_imports_are_aggregated_into_one_info_diagnostic() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("src/app.ts");
    let facts = facts(
        source.clone(),
        "TypeScript",
        "import a from \"./generated-a\";\nimport b from \"./generated-b\";\n",
        &["./generated-a", "./generated-b"],
    );
    let mut resolution = ImportResolutionStats::default();
    resolution.record_classified(&source, "./generated-a", root);
    resolution.record_classified(&source, "./generated-b", root);

    let result = analyze(root, &facts, &resolution);

    assert!(result.findings.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        "analysis.unresolved-local-import-limited"
    );
    assert_eq!(result.diagnostics[0].severity, DiagnosticSeverity::Info);
    assert!(result.diagnostics[0].message.contains('2'));
}
