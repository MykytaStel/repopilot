#[path = "api_contract_finding.rs"]
mod finding;
#[path = "api_contract_source.rs"]
mod source_facts;
use finding::occurrence_to_finding;

use crate::analysis::ParsedArtifact;
use crate::analysis::api_contract::{
    JavaScriptContractFactProvider, detect_removed_export_imports, is_contract_path,
};
use crate::analysis::symbols::JavaScriptSymbolFacts;
use crate::findings::types::Finding;
use crate::graph::resolver::normalize_path;
use crate::review::diff::{ChangedFile, DiffTarget};
use crate::review::signals::api_contract::extract_javascript_symbol_facts;
use crate::review::signals::content::{
    ReviewSource, batched_pre_change_sources, pre_change_source,
};
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
    provider.prefetch_pre_change_sources(&modified_exporters);

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
    pre_change_sources: HashMap<PathBuf, Option<ReviewSource>>,
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
            pre_change_sources: HashMap::new(),
            before_facts: HashMap::new(),
        }
    }

    /// Reads every supported exporter's pre-change source in one Git batch
    /// instead of one `git show` per file. Facts are still extracted only for
    /// an exporter that the detectors check.
    fn prefetch_pre_change_sources(&mut self, exporters: &[PathBuf]) {
        let (paths, files): (Vec<_>, Vec<_>) = exporters
            .iter()
            .filter(|path| is_contract_path(path))
            .filter_map(|path| Some((path.clone(), (*self.changed_files.get(path)?).clone())))
            .unzip();
        let sources = batched_pre_change_sources(self.repo_root, self.target, &files);
        self.pre_change_sources
            .extend(paths.into_iter().zip(sources));
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
        let source = match self.pre_change_sources.remove(path) {
            Some(source) => source,
            None => self
                .changed_files
                .get(path)
                .and_then(|file| pre_change_source(self.repo_root, file, self.target)),
        };
        let facts = source.and_then(|source| extract_javascript_symbol_facts(&source));
        self.before_facts.insert(path.to_path_buf(), facts.clone());
        facts
    }

    fn current_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        if let Some(artifact) = self.changed_artifact(path)
            && let Some(symbols) = artifact.javascript_symbols.as_ref()
        {
            return Some(symbols.clone());
        }
        if let Some((hash, language)) = self.current_cache_key(path)
            && let Some(facts) = self
                .parsed_cache
                .lookup_javascript_symbols(&hash, language.as_deref())
        {
            return Some(facts);
        }
        let file = self
            .repo_files
            .iter()
            .find(|file| repository_relative(&file.path, self.repo_root) == path)?;
        source_facts::rebuild(self.repo_root, path, file, self.parsed_cache)
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
