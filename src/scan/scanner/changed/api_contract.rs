use crate::analysis::ParsedArtifact;
use crate::analysis::api_contract::{
    JavaScriptContractFactProvider, RemovedExportOccurrence, detect_removed_export_imports,
};
use crate::analysis::symbols::{JavaScriptSymbolFacts, SymbolKind};
use crate::findings::provenance::{AnalysisScope, FindingProvenance};
use crate::findings::types::{Evidence, Finding, FindingCategory};
use crate::graph::resolver::normalize_path;
use crate::review::diff::{ChangedFile, DiffTarget};
use crate::review::signals::api_contract::extract_javascript_symbol_facts;
use crate::review::signals::content::pre_change_source;
use crate::scan::cache::relative_cache_path;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::parsed_cache::ParsedFactsCache;
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::{
    ChangedDiscoveryStage, ChangedFileAnalysisStage, ChangedRepoContextStage, ChangedScanEngine,
};

pub(super) struct ChangedApiContractContext<'a> {
    pub repo_root: &'a Path,
    pub base_ref: Option<&'a str>,
    pub changed_files: &'a [ChangedFile],
    pub changed_artifacts: &'a BTreeMap<PathBuf, ParsedArtifact>,
    pub repo_context: &'a ScanFacts,
    pub graph: &'a CouplingGraph,
}

impl ChangedScanEngine<'_> {
    pub(super) fn run_api_contract_analysis(
        &self,
        discovery: &ChangedDiscoveryStage,
        file_stage: &mut ChangedFileAnalysisStage,
        repo_stage: &ChangedRepoContextStage,
    ) {
        let findings = detect_findings(
            ChangedApiContractContext {
                repo_root: &discovery.repo_root,
                base_ref: self.base_ref,
                changed_files: &discovery.changed_files,
                changed_artifacts: &file_stage.facts.artifacts,
                repo_context: &repo_stage.repo_context,
                graph: &repo_stage.coupling_graph,
            },
            &mut file_stage.parsed_cache,
        );
        file_stage.findings.extend(findings);
    }
}

pub(super) fn detect_findings(
    context: ChangedApiContractContext<'_>,
    parsed_cache: &mut ParsedFactsCache,
) -> Vec<Finding> {
    let modified_exporters = context
        .changed_files
        .iter()
        .filter(|file| file.status == crate::review::diff::ChangeStatus::Modified)
        .map(|file| repository_relative(&file.path, context.repo_root))
        .collect::<Vec<_>>();
    if modified_exporters.is_empty() {
        return Vec::new();
    }
    let current_files = context
        .repo_context
        .files
        .iter()
        .map(|file| absolute_path(context.repo_root, &file.path))
        .collect::<HashSet<_>>();
    if current_files.is_empty() {
        return Vec::new();
    }
    let target = context
        .base_ref
        .map_or(DiffTarget::WorkingTree, |base| DiffTarget::Refs {
            base,
            head: "HEAD",
        });
    let mut provider = ScanFactProvider::new(
        context.repo_root,
        target,
        context.changed_files,
        context.changed_artifacts,
        &context.repo_context.files,
        &context.repo_context.parsed_content_hashes,
        parsed_cache,
    );

    detect_removed_export_imports(
        context.repo_root,
        &modified_exporters,
        context.graph,
        &current_files,
        &mut provider,
    )
    .iter()
    .map(occurrence_to_finding)
    .collect()
}

struct ScanFactProvider<'a> {
    repo_root: &'a Path,
    target: DiffTarget<'a>,
    changed_files: HashMap<PathBuf, &'a ChangedFile>,
    changed_artifacts: &'a BTreeMap<PathBuf, ParsedArtifact>,
    repo_files: &'a [FileFacts],
    content_hashes: &'a BTreeMap<PathBuf, String>,
    parsed_cache: &'a mut ParsedFactsCache,
    before_facts: HashMap<PathBuf, Option<JavaScriptSymbolFacts>>,
}

impl<'a> ScanFactProvider<'a> {
    fn new(
        repo_root: &'a Path,
        target: DiffTarget<'a>,
        changed_files: &'a [ChangedFile],
        changed_artifacts: &'a BTreeMap<PathBuf, ParsedArtifact>,
        repo_files: &'a [FileFacts],
        content_hashes: &'a BTreeMap<PathBuf, String>,
        parsed_cache: &'a mut ParsedFactsCache,
    ) -> Self {
        let changed_files = changed_files
            .iter()
            .map(|file| (repository_relative(&file.path, repo_root), file))
            .collect();
        Self {
            repo_root,
            target,
            changed_files,
            changed_artifacts,
            repo_files,
            content_hashes,
            parsed_cache,
            before_facts: HashMap::new(),
        }
    }

    fn changed_artifact(&self, path: &Path) -> Option<&ParsedArtifact> {
        self.changed_artifacts
            .iter()
            .find(|(candidate, _)| repository_relative(candidate, self.repo_root) == path)
            .map(|(_, artifact)| artifact)
    }

    fn current_cache_key(&self, path: &Path) -> Option<(String, Option<String>)> {
        let hash = self
            .content_hashes
            .iter()
            .find(|(candidate, _)| repository_relative(candidate, self.repo_root) == path)
            .map(|(_, hash)| hash.clone())?;
        let language = self
            .repo_files
            .iter()
            .find(|file| repository_relative(&file.path, self.repo_root) == path)
            .and_then(|file| file.language.clone());
        Some((hash, language))
    }
}

impl JavaScriptContractFactProvider for ScanFactProvider<'_> {
    fn pre_change_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        if let Some(facts) = self.before_facts.get(path) {
            return facts.clone();
        }
        let facts = self
            .changed_files
            .get(path)
            .and_then(|file| pre_change_source(self.repo_root, file, self.target))
            .and_then(|source| extract_javascript_symbol_facts(&source));
        self.before_facts.insert(path.to_path_buf(), facts.clone());
        facts
    }

    fn current_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        if let Some(artifact) = self.changed_artifact(path) {
            return artifact.javascript_symbols.clone();
        }
        let (hash, language) = self.current_cache_key(path)?;
        self.parsed_cache
            .lookup_javascript_symbols(&hash, language.as_deref())
    }
}

fn occurrence_to_finding(occurrence: &RemovedExportOccurrence) -> Finding {
    let symbol_kind = match occurrence.symbol_kind {
        SymbolKind::Value => "value",
        SymbolKind::Type => "type",
    };
    let exporter = occurrence
        .exporter_path
        .to_string_lossy()
        .replace('\\', "/");
    let snippet = format!(
        "named {symbol_kind} import '{} as {}' from '{}' resolves to '{}'",
        occurrence.exported_name, occurrence.local_name, occurrence.module_specifier, exporter,
    );

    Finding {
        rule_id: "behavioral.removed-export-still-imported".to_string(),
        description: format!(
            "Removed {symbol_kind} export '{}' from {exporter} remains imported as local binding '{}'.",
            occurrence.exported_name, occurrence.local_name,
        ),
        category: FindingCategory::CodeQuality,
        evidence: vec![Evidence {
            path: occurrence.importer_path.clone(),
            line_start: occurrence.line_start,
            line_end: Some(occurrence.line_end),
            snippet,
        }],
        provenance: FindingProvenance {
            analysis_scope: AnalysisScope::GitDiff,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn repository_relative(path: &Path, repo_root: &Path) -> PathBuf {
    PathBuf::from(relative_cache_path(repo_root, path))
}

fn absolute_path(repo_root: &Path, path: &Path) -> PathBuf {
    normalize_path(&if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::Severity;
    use std::path::PathBuf;

    #[test]
    fn occurrence_projects_to_git_diff_finding_on_the_caller_import() {
        let finding = occurrence_to_finding(&RemovedExportOccurrence {
            exporter_path: PathBuf::from("src/api.ts"),
            importer_path: PathBuf::from("src/caller.ts"),
            exported_name: "loadUser".to_string(),
            local_name: "load".to_string(),
            symbol_kind: SymbolKind::Value,
            module_specifier: "./api.ts".to_string(),
            line_start: 3,
            line_end: 3,
            byte_start: 12,
            byte_end: 28,
        });

        assert_eq!(finding.rule_id, "behavioral.removed-export-still-imported");
        assert_eq!(finding.category, FindingCategory::CodeQuality);
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(finding.provenance.analysis_scope, AnalysisScope::GitDiff);
        assert_eq!(finding.evidence[0].path, PathBuf::from("src/caller.ts"));
        assert_eq!(finding.evidence[0].line_start, 3);
        assert!(finding.evidence[0].snippet.contains("src/api.ts"));
        assert!(finding.evidence[0].snippet.contains("loadUser as load"));
    }
}
