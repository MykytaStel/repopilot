pub mod context;
mod cycles;
pub mod imports;
pub mod resolver;
pub mod v2;

mod coupling_metrics;
mod resolution_stats;

pub use coupling_metrics::coupling_file_metrics;
pub(crate) use coupling_metrics::coupling_file_metrics_from_snapshot;
pub(crate) use cycles::mark_cycle_detection_depth_exceeded;
pub use cycles::{
    clear_cycle_detection_depth_exceeded, detect_cycles, detect_cycles_bounded,
    was_cycle_detection_depth_exceeded, without_deferred_edges,
    without_rust_module_containment_edges,
};
pub use imports::extract_imports;
pub use resolution_stats::{
    ImportResolutionStats, UnresolvedImportEvidence, UnresolvedImportKind,
    UnresolvedImportLimitation, UnresolvedImportProof,
};
pub use resolver::resolve_import;

use crate::scan::facts::ScanFacts;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub use crate::scan::types::CouplingGraph;

struct ResolvedGraphEdges {
    edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    deferred_edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    resolution: ImportResolutionStats,
}

// ── Data structures ───────────────────────────────────────────────────────────

pub struct FileMetrics {
    pub path: PathBuf,
    pub fan_in: usize,
    pub fan_out: usize,
    /// instability = fan_out / (fan_in + fan_out); 0.0 when both are zero.
    pub instability: f32,
}

// ── Graph construction ────────────────────────────────────────────────────────

pub fn build_coupling_graph(facts: &ScanFacts, root: &Path) -> CouplingGraph {
    build_coupling_graph_with_resolution(facts, root).0
}

/// Builds the coupling graph and, alongside it, the imports the resolver could
/// not map to scanned files. The graph carries only proven edges; the stats
/// tell absence-based consumers (dead modules, fan-in metrics) where the graph
/// is provably incomplete.
pub fn build_coupling_graph_with_resolution(
    facts: &ScanFacts,
    root: &Path,
) -> (CouplingGraph, ImportResolutionStats) {
    let known_file_by_normalized: HashMap<PathBuf, PathBuf> = facts
        .files
        .iter()
        .map(|file| (resolver::normalize_path(&file.path), file.path.clone()))
        .collect();
    let known_files: HashSet<PathBuf> = known_file_by_normalized.keys().cloned().collect();
    let inventory = resolver::ResolverInventory::new(&known_files);
    let resolved = resolve_graph_edges(facts, root, &known_file_by_normalized, &inventory);
    (
        build_coupling_graph_from_edges(resolved.edges, resolved.deferred_edges),
        resolved.resolution,
    )
}

fn resolve_graph_edges(
    facts: &ScanFacts,
    root: &Path,
    known_file_by_normalized: &HashMap<PathBuf, PathBuf>,
    inventory: &resolver::ResolverInventory<'_>,
) -> ResolvedGraphEdges {
    let repo_dirs =
        resolution_stats::repo_directory_names(facts.files.iter().map(|file| file.path.as_path()));
    let repo_python_modules = resolution_stats::repo_python_module_names(
        facts.files.iter().map(|file| file.path.as_path()),
        root,
    );
    let repo_jvm_packages = resolution_stats::repo_jvm_package_suffixes(
        facts.files.iter().map(|file| file.path.as_path()),
    );

    let mut edges: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut deferred_edges: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut resolution = ImportResolutionStats::default();

    for file in &facts.files {
        process_graph_file(
            file,
            root,
            known_file_by_normalized,
            inventory,
            &repo_dirs,
            &repo_python_modules,
            &repo_jvm_packages,
            &mut edges,
            &mut deferred_edges,
            &mut resolution,
        );
    }

    ResolvedGraphEdges {
        edges,
        deferred_edges,
        resolution,
    }
}

fn build_coupling_graph_from_edges(
    edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    deferred_edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
) -> CouplingGraph {
    let mut nodes: BTreeSet<PathBuf> = edges.keys().cloned().collect();
    for targets in edges.values() {
        nodes.extend(targets.iter().cloned());
    }
    CouplingGraph {
        edges,
        deferred_edges,
        nodes,
    }
}

#[allow(clippy::too_many_arguments)]
fn process_graph_file(
    file: &crate::scan::facts::FileFacts,
    root: &Path,
    known_file_by_normalized: &HashMap<PathBuf, PathBuf>,
    inventory: &resolver::ResolverInventory<'_>,
    repo_dirs: &HashSet<String>,
    repo_python_modules: &HashSet<String>,
    repo_jvm_packages: &HashSet<PathBuf>,
    edges: &mut BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    deferred_edges: &mut BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    resolution: &mut ImportResolutionStats,
) {
    let source = file.path.clone();
    let normalized_source = resolver::normalize_path(&source);
    let deferred_raws = file
        .deferred_imports
        .iter()
        .map(|raw| raw.trim())
        .collect::<HashSet<_>>();
    let mut eager_targets = BTreeSet::new();
    let outgoing = edges.entry(source.clone()).or_default();

    for raw in &file.imports {
        process_import(
            raw,
            &source,
            &normalized_source,
            &deferred_raws,
            root,
            known_file_by_normalized,
            inventory,
            repo_dirs,
            repo_python_modules,
            repo_jvm_packages,
            outgoing,
            &mut eager_targets,
            resolution,
        );
    }

    add_deferred_imports(
        file,
        root,
        &normalized_source,
        &eager_targets,
        known_file_by_normalized,
        inventory,
        deferred_edges,
    );
}

fn add_deferred_imports(
    file: &crate::scan::facts::FileFacts,
    root: &Path,
    normalized_source: &Path,
    eager_targets: &BTreeSet<PathBuf>,
    known_file_by_normalized: &HashMap<PathBuf, PathBuf>,
    inventory: &resolver::ResolverInventory<'_>,
    deferred_edges: &mut BTreeMap<PathBuf, BTreeSet<PathBuf>>,
) {
    let source = file.path.clone();
    for raw in &file.deferred_imports {
        if let Some(target) = inventory.resolve(raw, normalized_source, root)
            && target.as_path() != normalized_source
        {
            let resolved = known_file_by_normalized
                .get(&target)
                .cloned()
                .unwrap_or(target);
            if !eager_targets.contains(&resolved) {
                deferred_edges
                    .entry(source.clone())
                    .or_default()
                    .insert(resolved);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_import(
    raw: &str,
    source: &Path,
    normalized_source: &Path,
    deferred_raws: &HashSet<&str>,
    root: &Path,
    known_file_by_normalized: &HashMap<PathBuf, PathBuf>,
    inventory: &resolver::ResolverInventory<'_>,
    repo_dirs: &HashSet<String>,
    repo_python_modules: &HashSet<String>,
    repo_jvm_packages: &HashSet<PathBuf>,
    outgoing: &mut BTreeSet<PathBuf>,
    eager_targets: &mut BTreeSet<PathBuf>,
    resolution: &mut ImportResolutionStats,
) {
    match inventory.resolve(raw, normalized_source, root) {
        Some(target) if target != normalized_source => {
            let resolved = known_file_by_normalized
                .get(&target)
                .cloned()
                .unwrap_or(target);
            if !deferred_raws.contains(raw.trim()) {
                eager_targets.insert(resolved.clone());
            }
            outgoing.insert(resolved);
        }
        Some(_) => {}
        None if (if source.extension().and_then(|extension| extension.to_str()) == Some("py")
            && !raw.trim().starts_with('.')
        {
            resolution_stats::is_unresolved_python_import(raw.trim(), source, repo_python_modules)
        } else {
            resolution_stats::is_unresolved_internal_import(
                raw.trim(),
                source,
                repo_jvm_packages,
                repo_dirs,
            )
        }) || resolution_stats::is_unresolved_go_module_import(
            raw.trim(),
            source,
            root,
        ) || resolver::is_configured_ts_alias_import(raw.trim(), source, root) =>
        {
            resolution.record_classified(source, raw.trim(), root)
        }
        None => {}
    }
}

// ── Metrics ───────────────────────────────────────────────────────────────────

pub fn compute_metrics(graph: &CouplingGraph) -> Vec<FileMetrics> {
    // Single pass: accumulate fan_out and fan_in from edges without pre-initialising maps.
    let mut fan_out: HashMap<&PathBuf, usize> = HashMap::new();
    let mut fan_in: HashMap<&PathBuf, usize> = HashMap::new();

    for (from, targets) in &graph.edges {
        fan_out.insert(from, targets.len());
        for target in targets {
            *fan_in.entry(target).or_insert(0) += 1;
        }
    }

    graph
        .nodes
        .iter()
        .map(|path| {
            let fo = fan_out.get(path).copied().unwrap_or(0);
            let fi = fan_in.get(path).copied().unwrap_or(0);
            let instability = if fo + fi == 0 {
                0.0_f32
            } else {
                fo as f32 / (fi + fo) as f32
            };
            FileMetrics {
                path: path.clone(),
                fan_in: fi,
                fan_out: fo,
                instability,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
