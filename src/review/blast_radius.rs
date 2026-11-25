use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::review::signals::composites;
use crate::scan::types::ScanSummary;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub fn compute_blast_radius(
    summary: &ScanSummary,
    repo_root: &Path,
    changed_files: &[ChangedFile],
) -> Vec<PathBuf> {
    let graph = match &summary.artifacts.coupling_graph {
        Some(graph) => graph,
        None => return Vec::new(),
    };

    let importers = composites::build_importers_by_target(graph, repo_root);
    compute_blast_radius_with_importers(repo_root, changed_files, &importers)
}

pub(super) fn compute_blast_radius_with_importers(
    repo_root: &Path,
    changed_files: &[ChangedFile],
    importers_by_target: &BTreeMap<PathBuf, BTreeSet<PathBuf>>,
) -> Vec<PathBuf> {
    let changed_paths: BTreeSet<PathBuf> = changed_files
        .iter()
        .map(|file| normalized_review_path(&file.path, repo_root))
        .collect();

    let mut impacted = BTreeSet::new();

    for changed in &changed_paths {
        if let Some(importers) = importers_by_target.get(changed) {
            for importer in importers {
                if !changed_paths.contains(importer) {
                    impacted.insert(importer.clone());
                }
            }
        }
    }

    impacted.into_iter().collect()
}
