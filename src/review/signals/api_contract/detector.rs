use super::{
    ChangedReviewSources, ImportedSymbolFact, JavaScriptSymbolFacts, RemovedExportSignal,
    SymbolKind, extract_javascript_symbol_facts,
};
use crate::graph::resolve_import;
use crate::review::diff::{ChangeStatus, DiffTarget, target_file_inventory};
use crate::review::paths::normalized_review_path;
use crate::review::signals::composites::build_importers_by_target;
use crate::review::signals::content::{ReviewSource, post_change_source_at_path};
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(crate) fn detect_removed_export_imports(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_sources: &[ChangedReviewSources<'_>],
    graph: Option<&CouplingGraph>,
) -> Vec<RemovedExportSignal> {
    let Some(graph) = graph else {
        return Vec::new();
    };
    if !changed_sources.iter().any(is_supported_modification) {
        // Nothing can be proven, so the file inventory and the reverse importer
        // index are not worth building for this review.
        return Vec::new();
    }
    let importers = build_importers_by_target(graph, repo_root);
    let Some(known_files) = target_file_inventory(repo_root, target) else {
        return Vec::new();
    };
    let known_files = known_files
        .iter()
        .map(|path| absolute_path(repo_root, path))
        .collect::<HashSet<_>>();
    let mut caller_facts = CallerFactCache::new(repo_root, target, changed_sources);
    let mut signals = Vec::new();

    for sources in changed_sources {
        let Some((exporter, removed)) = removed_export_delta(sources, repo_root, &mut caller_facts)
        else {
            continue;
        };
        let Some(candidates) = importers.get(&exporter) else {
            continue;
        };
        signals.extend(confirm_callers(
            &exporter,
            &removed,
            candidates,
            &known_files,
            &mut caller_facts,
        ));
    }
    signals.sort();
    signals.dedup();
    signals
}

fn removed_export_delta(
    sources: &ChangedReviewSources<'_>,
    repo_root: &Path,
    caller_facts: &mut CallerFactCache<'_, '_, '_>,
) -> Option<(PathBuf, RemovedExports)> {
    if !is_supported_modification(sources) {
        return None;
    }
    let pre_facts = extract_javascript_symbol_facts(sources.pre?)?;
    let exporter = normalized_review_path(&sources.file.path, repo_root);
    let post_facts = caller_facts.facts_for(&exporter)?;
    if post_facts.wildcard_re_export {
        // A star re-export can still supply every name, so no removal is proven.
        return None;
    }
    let removed = removed_symbols(&pre_facts.exports, &post_facts);
    (!removed.is_empty()).then_some((exporter, removed))
}

fn is_supported_modification(sources: &ChangedReviewSources<'_>) -> bool {
    sources.file.status == ChangeStatus::Modified && is_supported_path(&sources.file.path)
}

fn confirm_callers(
    exporter: &Path,
    removed: &RemovedExports,
    candidates: &BTreeSet<PathBuf>,
    known_files: &HashSet<PathBuf>,
    caller_facts: &mut CallerFactCache<'_, '_, '_>,
) -> Vec<RemovedExportSignal> {
    let mut signals = Vec::new();
    for importer in candidates.iter().filter(|path| is_supported_path(path)) {
        let Some(facts) = caller_facts.facts_for(importer) else {
            continue;
        };
        signals.extend(facts.imports.iter().filter_map(|import| {
            confirmed_occurrence(
                import,
                importer,
                exporter,
                removed,
                caller_facts.repo_root,
                known_files,
            )
        }));
    }
    signals
}

fn confirmed_occurrence(
    import: &ImportedSymbolFact,
    importer: &Path,
    exporter: &Path,
    removed: &RemovedExports,
    repo_root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<RemovedExportSignal> {
    if !import.module_specifier.starts_with('.')
        || !removed.matches(&import.imported_name, import.kind)
    {
        return None;
    }
    let resolved = resolve_import(
        &import.module_specifier,
        &absolute_path(repo_root, importer),
        repo_root,
        known_files,
    );
    let exporter_absolute = absolute_path(repo_root, exporter);
    if resolved.as_deref() != Some(exporter_absolute.as_path()) {
        return None;
    }
    Some(RemovedExportSignal {
        exporter_path: exporter.to_path_buf(),
        importer_path: importer.to_path_buf(),
        exported_name: import.imported_name.clone(),
        local_name: import.local_name.clone(),
        symbol_kind: import.kind,
        module_specifier: import.module_specifier.clone(),
        line_start: import.line_start,
        line_end: import.line_end,
        byte_start: import.byte_start,
        byte_end: import.byte_end,
    })
}

/// Named exports the change dropped from one module.
#[derive(Debug, Default, PartialEq, Eq)]
struct RemovedExports {
    /// Exact `(name, kind)` pairs the post-change module no longer declares.
    pairs: BTreeSet<(String, SymbolKind)>,
    /// Names the post-change module no longer supplies under any kind. Such a
    /// name breaks every named import of it, however the caller spelled the
    /// import, so the kind is not part of the match.
    vanished: BTreeSet<String>,
}

impl RemovedExports {
    fn matches(&self, name: &str, kind: SymbolKind) -> bool {
        self.vanished.contains(name) || self.pairs.contains(&(name.to_string(), kind))
    }

    fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

fn removed_symbols(
    pre: &[super::ExportedSymbolFact],
    post: &JavaScriptSymbolFacts,
) -> RemovedExports {
    let after_pairs = post
        .exports
        .iter()
        .map(|fact| (fact.name.clone(), fact.kind))
        .collect::<BTreeSet<_>>();
    let after_names = post
        .exports
        .iter()
        .map(|fact| fact.name.as_str())
        .collect::<HashSet<_>>();
    let forwarded = post
        .re_exports
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let pairs = pre
        .iter()
        .map(|fact| (fact.name.clone(), fact.kind))
        .filter(|(name, _)| !forwarded.contains(name.as_str()))
        .filter(|symbol| !after_pairs.contains(symbol))
        .collect::<BTreeSet<_>>();
    let vanished = pairs
        .iter()
        .map(|(name, _)| name.clone())
        .filter(|name| !after_names.contains(name.as_str()))
        .collect();
    RemovedExports { pairs, vanished }
}

struct CallerFactCache<'root, 'target, 'source> {
    repo_root: &'root Path,
    target: DiffTarget<'target>,
    changed_post: HashMap<PathBuf, Option<&'source ReviewSource>>,
    facts: HashMap<PathBuf, Option<JavaScriptSymbolFacts>>,
}

impl<'root, 'target, 'source> CallerFactCache<'root, 'target, 'source> {
    fn new(
        repo_root: &'root Path,
        target: DiffTarget<'target>,
        changed_sources: &[ChangedReviewSources<'source>],
    ) -> Self {
        let changed_post = changed_sources
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
            changed_post,
            facts: HashMap::new(),
        }
    }

    fn facts_for(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        if let Some(facts) = self.facts.get(path) {
            return facts.clone();
        }
        let facts = match self.changed_post.get(path).copied() {
            Some(Some(source)) => extract_javascript_symbol_facts(source),
            Some(None) => None,
            None => post_change_source_at_path(self.repo_root, path, self.target)
                .and_then(|source| extract_javascript_symbol_facts(&source)),
        };
        self.facts.insert(path.to_path_buf(), facts.clone());
        facts
    }
}

fn absolute_path(repo_root: &Path, path: &Path) -> PathBuf {
    crate::graph::resolver::normalize_path(&if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    })
}

fn is_supported_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("ts" | "tsx" | "js" | "jsx")
    )
}
