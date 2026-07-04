use super::summary::{build_language_summary, directory_count};
use super::*;
use crate::graph::resolve_import;
use crate::graph::resolver::normalize_path;

impl RepoContextGraph {
    pub fn from_scan_facts(facts: &ScanFacts, root: &Path, coupling_graph: CouplingGraph) -> Self {
        Self {
            root_path: root.to_path_buf(),
            nodes: facts
                .files
                .iter()
                .map(|file| RepoContextNode::from_file(file, root))
                .collect(),
            edges: relative_edges(coupling_graph.edges, root),
            deferred_edges: relative_edges(coupling_graph.deferred_edges, root),
            detected_frameworks: facts.detected_frameworks.clone(),
            framework_projects: facts.framework_projects.clone(),
            react_native: facts.react_native.clone(),
        }
    }

    pub fn to_scan_facts(&self) -> ScanFacts {
        let files = self
            .nodes
            .iter()
            .map(RepoContextNode::to_file_facts)
            .collect::<Vec<_>>();

        let mut languages = HashMap::new();
        let mut non_empty_lines = 0usize;
        for file in &files {
            non_empty_lines += file.non_empty_lines;
            if let Some(language) = &file.language {
                *languages.entry(language.clone()).or_insert(0usize) += 1;
            }
        }

        ScanFacts {
            root_path: self.root_path.clone(),
            files_discovered: files.len(),
            files_analyzed: files.len(),
            directories_count: directory_count(&files),
            non_empty_lines,
            languages: build_language_summary(languages),
            files,
            detected_frameworks: self.detected_frameworks.clone(),
            framework_projects: self.framework_projects.clone(),
            react_native: self.react_native.clone(),
            ..ScanFacts::default()
        }
    }

    pub fn coupling_graph(&self) -> CouplingGraph {
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.path.clone())
            .chain(
                self.edges
                    .values()
                    .flat_map(|targets| targets.iter().cloned()),
            )
            .collect::<BTreeSet<_>>();

        CouplingGraph {
            edges: self.edges.clone(),
            deferred_edges: self.deferred_edges.clone(),
            nodes,
        }
    }

    pub fn apply_changed_facts(
        &mut self,
        repo_root: &Path,
        changed_files: &[ChangedFile],
        patch_files: &[FileFacts],
    ) {
        let removed = changed_files
            .iter()
            .filter(|file| file.status == ChangeStatus::Deleted)
            .map(|file| file.path.clone())
            .collect::<HashSet<_>>();

        self.nodes.retain(|node| !removed.contains(&node.path));
        self.nodes
            .retain(|node| !patch_files.iter().any(|file| file.path == node.path));
        self.nodes.extend(
            patch_files
                .iter()
                .map(|file| RepoContextNode::from_file(file, repo_root)),
        );
        self.nodes.sort_by(|left, right| left.path.cmp(&right.path));

        let known_file_set_changed = changed_files.iter().any(|file| {
            matches!(
                file.status,
                ChangeStatus::Added | ChangeStatus::Deleted | ChangeStatus::Renamed
            )
        });
        if known_file_set_changed {
            let files = self
                .nodes
                .iter()
                .map(RepoContextNode::to_file_facts)
                .collect::<Vec<_>>();
            let (edges, deferred_edges) = resolve_graph_edges(repo_root, &self.nodes, &files);
            self.edges = edges;
            self.deferred_edges = deferred_edges;
            return;
        }

        let patch_sources = patch_files
            .iter()
            .map(|file| relative_graph_path(repo_root, &file.path))
            .collect::<HashSet<_>>();

        remove_edge_sources_and_targets(&mut self.edges, &removed, &patch_sources);
        remove_edge_sources_and_targets(&mut self.deferred_edges, &removed, &patch_sources);

        let known_files = known_files_by_normalized_path(repo_root, &self.nodes);
        let known_file_paths = known_files.keys().cloned().collect::<HashSet<_>>();
        for file in patch_files {
            let source = relative_graph_path(repo_root, &file.path);
            let (edges, deferred_edges) =
                resolve_file_edges(file, repo_root, &known_files, &known_file_paths);
            self.edges.insert(source.clone(), edges);
            if deferred_edges.is_empty() {
                self.deferred_edges.remove(&source);
            } else {
                self.deferred_edges.insert(source, deferred_edges);
            }
        }
    }
}

impl RepoContextNode {
    fn from_file(file: &FileFacts, root: &Path) -> Self {
        let context = classify_file(file);
        let roles = context.role_ids().into_iter().map(str::to_string).collect();
        let frameworks = context
            .framework_ids()
            .into_iter()
            .map(str::to_string)
            .collect();
        let runtimes = context
            .runtime_ids()
            .into_iter()
            .map(str::to_string)
            .collect();
        let paradigms = context
            .paradigm_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let is_generated = context.has_role(FileRole::Generated);
        let is_config = context.has_role(FileRole::Config);

        Self {
            path: relative_graph_path(root, &file.path),
            language: file.language.clone(),
            roles,
            frameworks,
            runtimes,
            paradigms,
            workspace_package: None,
            non_empty_lines: file.non_empty_lines,
            imports: file.imports.clone(),
            deferred_imports: file.deferred_imports.clone(),
            is_test: context.is_test,
            is_generated,
            is_config,
        }
    }

    fn to_file_facts(&self) -> FileFacts {
        FileFacts {
            path: self.path.clone(),
            language: self.language.clone(),
            non_empty_lines: self.non_empty_lines,
            branch_count: 0,
            imports: self.imports.clone(),
            content: None,
            has_inline_tests: self.is_test,
            in_executable_package: false,
            deferred_imports: self.deferred_imports.clone(),
        }
    }
}

fn relative_edges(
    edges: BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    root: &Path,
) -> BTreeMap<PathBuf, BTreeSet<PathBuf>> {
    edges
        .into_iter()
        .map(|(source, targets)| {
            (
                relative_graph_path(root, &source),
                targets
                    .into_iter()
                    .map(|target| relative_graph_path(root, &target))
                    .collect(),
            )
        })
        .collect()
}

fn remove_edge_sources_and_targets(
    edges: &mut BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    removed: &HashSet<PathBuf>,
    patch_sources: &HashSet<PathBuf>,
) {
    for source in removed.iter().chain(patch_sources) {
        edges.remove(source);
    }
    for targets in edges.values_mut() {
        targets.retain(|target| !removed.contains(target));
    }
}

fn resolve_graph_edges(
    root: &Path,
    nodes: &[RepoContextNode],
    files: &[FileFacts],
) -> (
    BTreeMap<PathBuf, BTreeSet<PathBuf>>,
    BTreeMap<PathBuf, BTreeSet<PathBuf>>,
) {
    let known_files = known_files_by_normalized_path(root, nodes);
    let known_file_paths = known_files.keys().cloned().collect::<HashSet<_>>();
    let mut edges = BTreeMap::new();
    let mut deferred_edges = BTreeMap::new();

    for file in files {
        let source = relative_graph_path(root, &file.path);
        let (resolved_edges, resolved_deferred_edges) =
            resolve_file_edges(file, root, &known_files, &known_file_paths);
        edges.insert(source.clone(), resolved_edges);
        if !resolved_deferred_edges.is_empty() {
            deferred_edges.insert(source, resolved_deferred_edges);
        }
    }

    (edges, deferred_edges)
}

fn known_files_by_normalized_path(
    root: &Path,
    nodes: &[RepoContextNode],
) -> BTreeMap<PathBuf, PathBuf> {
    nodes
        .iter()
        .map(|node| {
            let absolute = absolute_graph_path(root, &node.path);
            (normalize_path(&absolute), node.path.clone())
        })
        .collect()
}

fn resolve_file_edges(
    file: &FileFacts,
    root: &Path,
    known_files: &BTreeMap<PathBuf, PathBuf>,
    known_file_paths: &HashSet<PathBuf>,
) -> (BTreeSet<PathBuf>, BTreeSet<PathBuf>) {
    let source = relative_graph_path(root, &file.path);
    let source_abs = normalize_path(&absolute_graph_path(root, &source));
    let deferred_raws = file
        .deferred_imports
        .iter()
        .map(|raw| raw.trim())
        .collect::<HashSet<_>>();
    let mut edges = BTreeSet::new();
    let mut eager_targets = BTreeSet::new();

    for raw in &file.imports {
        if let Some(resolved) = resolve_import(raw, &source_abs, root, known_file_paths)
            && resolved != source_abs
            && let Some(target) = known_files.get(&resolved)
        {
            if !deferred_raws.contains(raw.trim()) {
                eager_targets.insert(target.clone());
            }
            edges.insert(target.clone());
        }
    }

    let mut deferred_edges = BTreeSet::new();
    for raw in &file.deferred_imports {
        if let Some(resolved) = resolve_import(raw, &source_abs, root, known_file_paths)
            && resolved != source_abs
            && let Some(target) = known_files.get(&resolved)
            && !eager_targets.contains(target)
        {
            deferred_edges.insert(target.clone());
        }
    }

    (edges, deferred_edges)
}

fn absolute_graph_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn relative_graph_path(root: &Path, path: &Path) -> PathBuf {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();
    PathBuf::from(rel)
}
