use super::graph_context::GraphAuditContext;
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::graph::imports::lines::import_line_spans;
use crate::graph::{UnresolvedImportLimitation, UnresolvedImportProof};
use crate::scan::facts::FileFacts;
use crate::scan::types::{DiagnosticSeverity, ScanDiagnostic};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const RULE_ID: &str = "architecture.unresolved-local-import";

pub(crate) struct UnresolvedLocalImportAnalysis {
    pub(crate) findings: Vec<Finding>,
    pub(crate) diagnostics: Vec<ScanDiagnostic>,
}

pub(crate) fn analyze_unresolved_local_imports(
    context: &GraphAuditContext<'_>,
) -> UnresolvedLocalImportAnalysis {
    let mut findings = Vec::new();
    let mut limitations = BTreeMap::new();
    for unresolved in context.resolution.evidence() {
        if is_shadowed_python_submodule(unresolved, context.resolution) {
            continue;
        }
        if speculation::is_python_package_member(unresolved, source_facts(context, unresolved)) {
            *limitations
                .entry(UnresolvedImportLimitation::PythonPackageMember)
                .or_insert(0usize) += 1;
            continue;
        }
        match &unresolved.proof {
            UnresolvedImportProof::DefinitiveLocalCandidates(candidates)
                if !candidates.iter().any(|candidate| candidate.is_file()) =>
            {
                findings.push(missing_import_finding(context, unresolved, candidates));
            }
            UnresolvedImportProof::DefinitiveLocalCandidates(_) => {}
            UnresolvedImportProof::Limited(reason) => {
                *limitations.entry(*reason).or_insert(0usize) += 1;
            }
        }
    }
    findings.sort_by(|left, right| left.evidence[0].path.cmp(&right.evidence[0].path));
    UnresolvedLocalImportAnalysis {
        findings,
        diagnostics: limitation_diagnostics(limitations),
    }
}

fn source_facts<'a>(
    context: &'a GraphAuditContext<'_>,
    unresolved: &crate::graph::UnresolvedImportEvidence,
) -> Option<&'a FileFacts> {
    context.facts.files.iter().find(|file| {
        crate::graph::resolver::normalize_path(&file.path)
            == crate::graph::resolver::normalize_path(&unresolved.source)
    })
}

fn is_shadowed_python_submodule(
    unresolved: &crate::graph::UnresolvedImportEvidence,
    resolution: &crate::graph::ImportResolutionStats,
) -> bool {
    if unresolved
        .source
        .extension()
        .and_then(|value| value.to_str())
        != Some("py")
    {
        return false;
    }
    resolution.evidence().any(|parent| {
        parent.source == unresolved.source
            && matches!(
                parent.proof,
                UnresolvedImportProof::DefinitiveLocalCandidates(_)
            )
            && unresolved
                .raw_import
                .strip_prefix(&parent.raw_import)
                .is_some_and(|suffix| suffix.starts_with('.') && suffix.len() > 1)
    })
}

fn missing_import_finding(
    context: &GraphAuditContext<'_>,
    unresolved: &crate::graph::UnresolvedImportEvidence,
    candidates: &[PathBuf],
) -> Finding {
    let path = relative_path(&unresolved.source, context.root);
    let source_facts = source_facts(context, unresolved);
    let stored_spans = context
        .facts
        .import_spans_by_file
        .iter()
        .find(|(path, _)| {
            crate::graph::resolver::normalize_path(path)
                == crate::graph::resolver::normalize_path(&unresolved.source)
        })
        .map(|(_, spans)| spans);
    let (line_start, line_end) = import_span(source_facts, stored_spans, &unresolved.raw_import);
    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        title: "Local import target is missing".to_string(),
        description: format!(
            "The local import `{}` does not resolve to any supported source file candidate.",
            unresolved.raw_import
        ),
        recommendation: "Restore the imported module, update the import path, or run the project compiler to confirm the intended generated target.".to_string(),
        category: FindingCategory::Architecture,
        severity: Severity::High,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path,
            line_start,
            line_end,
            snippet: candidate_snippet(&unresolved.raw_import, candidates, context.root),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn import_span(
    file: Option<&FileFacts>,
    stored_spans: Option<&BTreeMap<String, (usize, usize)>>,
    raw_import: &str,
) -> (usize, Option<usize>) {
    if let Some(&(start, end)) = stored_spans.and_then(|spans| spans.get(raw_import)) {
        return (start, (end > start).then_some(end));
    }
    let Some(file) = file else {
        return (1, None);
    };
    let Some(content) = file.content.as_deref() else {
        return (1, None);
    };
    import_line_spans(content, file.language.as_deref(), &[raw_import.to_string()])
        .get(raw_import)
        .copied()
        .map(|(start, end)| (start, (end > start).then_some(end)))
        .unwrap_or((1, None))
}

fn candidate_snippet(raw_import: &str, candidates: &[PathBuf], root: &Path) -> String {
    let checked = candidates
        .iter()
        .map(|candidate| relative_path(candidate, root).display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("unresolved local import `{raw_import}`; checked: {checked}")
}

fn limitation_diagnostics(
    limitations: BTreeMap<UnresolvedImportLimitation, usize>,
) -> Vec<ScanDiagnostic> {
    limitations
        .into_values()
        .map(|count| ScanDiagnostic {
            code: "analysis.unresolved-local-import-limited".to_string(),
            severity: DiagnosticSeverity::Info,
            message: format!(
                "{count} unresolved internal import(s) used ambiguous or unsupported resolution semantics and were not reported as broken code."
            ),
            path: None,
        })
        .collect()
}

fn relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

mod speculation;

#[cfg(test)]
mod tests;
