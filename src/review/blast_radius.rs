use crate::review::diff::ChangedFile;
use crate::review::paths::normalized_review_path;
use crate::review::signals::composites;
use crate::scan::types::ScanSummary;
use std::collections::BTreeSet;
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

    let changed_paths: BTreeSet<PathBuf> = changed_files
        .iter()
        .map(|file| normalized_review_path(&file.path, repo_root))
        .collect();

    let importers_by_target = composites::build_importers_by_target(graph, repo_root);

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
