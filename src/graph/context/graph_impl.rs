use super::summary::{build_language_summary, directory_count};
use super::*;

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
            // The serialized context-graph cache does not carry deferred-import
            // edges; cycle detection on a changed-scan cache hit treats all edges
            // as eager. Full scans (the default surface) resolve deferred edges.
            deferred_edges: Default::default(),
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

        let mut facts = self.to_scan_facts();
        for file in &mut facts.files {
            if file.path.is_relative() {
                file.path = repo_root.join(&file.path);
            }
        }
        self.edges = relative_edges(build_coupling_graph(&facts, repo_root).edges, repo_root);
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
            deferred_imports: Vec::new(),
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
