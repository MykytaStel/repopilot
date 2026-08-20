use crate::analysis::api_contract::{
    JavaScriptContractFactProvider, RemovedExportOccurrence,
    detect_removed_export_imports as detect_fact_level_removed_exports,
};
use crate::graph::resolver::normalize_path;
use crate::review::diff::ChangedFile;
use crate::review::diff::{ChangeStatus, DiffTarget, target_file_inventory};
use crate::review::paths::normalized_review_path;
use crate::review::signals::content::{ReviewSource, post_change_source_at_path};
use crate::scan::types::CouplingGraph;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;

pub(crate) use crate::analysis::symbols::{JavaScriptSymbolFacts, SymbolKind};

pub(crate) type RemovedExportSignal = RemovedExportOccurrence;

pub(crate) struct ChangedReviewSources<'a> {
    pub file: &'a ChangedFile,
    pub pre: Option<&'a ReviewSource>,
    pub post: Option<&'a ReviewSource>,
}

pub(crate) fn detect_removed_export_imports(
    repo_root: &std::path::Path,
    target: DiffTarget<'_>,
    changed_sources: &[ChangedReviewSources<'_>],
    graph: Option<&CouplingGraph>,
) -> Vec<RemovedExportSignal> {
    let Some(graph) = graph else {
        return Vec::new();
    };
    let modified_exporters = changed_sources
        .iter()
        .filter(|sources| sources.file.status == ChangeStatus::Modified)
        .map(|sources| normalized_review_path(&sources.file.path, repo_root))
        .collect::<Vec<_>>();
    if modified_exporters.is_empty() {
        return Vec::new();
    }
    let Some(current_files) = target_file_inventory(repo_root, target) else {
        return Vec::new();
    };
    let current_files = current_files
        .iter()
        .map(|path| absolute_path(repo_root, path))
        .collect::<HashSet<_>>();
    let mut provider = ReviewFactProvider::new(repo_root, target, changed_sources);

    detect_fact_level_removed_exports(
        repo_root,
        &modified_exporters,
        graph,
        &current_files,
        &mut provider,
    )
}

pub(crate) fn extract_javascript_symbol_facts(
    source: &ReviewSource,
) -> Option<JavaScriptSymbolFacts> {
    crate::analysis::symbols::javascript::extract_javascript_symbol_facts(
        source.content(),
        source.language_label(),
        source.tree()?,
    )
}

struct ReviewFactProvider<'root, 'target, 'source> {
    repo_root: &'root Path,
    target: DiffTarget<'target>,
    before: HashMap<PathBuf, Option<&'source ReviewSource>>,
    current: HashMap<PathBuf, Option<&'source ReviewSource>>,
    before_facts: HashMap<PathBuf, Option<JavaScriptSymbolFacts>>,
    current_facts: HashMap<PathBuf, Option<JavaScriptSymbolFacts>>,
}

impl<'root, 'target, 'source> ReviewFactProvider<'root, 'target, 'source> {
    fn new(
        repo_root: &'root Path,
        target: DiffTarget<'target>,
        changed_sources: &[ChangedReviewSources<'source>],
    ) -> Self {
        let before = changed_sources
            .iter()
            .map(|sources| {
                (
                    normalized_review_path(&sources.file.path, repo_root),
                    sources.pre,
                )
            })
            .collect();
        let current = changed_sources
            .iter()
            .map(|sources| {
                (
                    normalized_review_path(&sources.file.path, repo_root),
                    sources.post,
                )
            })
            .collect();
        Self {
            repo_root,
            target,
            before,
            current,
            before_facts: HashMap::new(),
            current_facts: HashMap::new(),
        }
    }
}

impl JavaScriptContractFactProvider for ReviewFactProvider<'_, '_, '_> {
    fn pre_change_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        if let Some(facts) = self.before_facts.get(path) {
            return facts.clone();
        }
        let facts = self
            .before
            .get(path)
            .copied()
            .flatten()
            .and_then(extract_javascript_symbol_facts);
        self.before_facts.insert(path.to_path_buf(), facts.clone());
        facts
    }

    fn current_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        if let Some(facts) = self.current_facts.get(path) {
            return facts.clone();
        }
        let facts = match self.current.get(path).copied() {
            Some(Some(source)) => extract_javascript_symbol_facts(source),
            Some(None) => None,
            None => post_change_source_at_path(self.repo_root, path, self.target)
                .and_then(|source| extract_javascript_symbol_facts(&source)),
        };
        self.current_facts.insert(path.to_path_buf(), facts.clone());
        facts
    }
}

fn absolute_path(repo_root: &Path, path: &Path) -> PathBuf {
    normalize_path(&if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    })
}

#[cfg(test)]
mod tests;
