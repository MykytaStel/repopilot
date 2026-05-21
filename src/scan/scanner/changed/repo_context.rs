use super::*;

impl<'a> ChangedScanEngine<'a> {
    pub(super) fn run_repo_context(
        &self,
        discovery: &ChangedDiscoveryStage,
        facts: &mut ScanFacts,
        graph_patch_files: &[FileFacts],
    ) -> io::Result<ChangedRepoContextStage> {
        let start = Instant::now();
        let mut diagnostics = Vec::new();
        let repo_root = &discovery.repo_root;

        if let Some(mut load) = load_repo_context_graph(repo_root, self.config) {
            load.graph
                .apply_changed_facts(repo_root, &discovery.changed_files, graph_patch_files);
            if let Err(error) = write_repo_context_graph(repo_root, self.config, &load.graph) {
                diagnostics.push(cache_diagnostic(&error));
            }

            let repo_context = load.graph.to_scan_facts();
            let coupling_graph = load.graph.coupling_graph();

            facts.detected_frameworks = repo_context.detected_frameworks.clone();
            facts.framework_projects = repo_context.framework_projects.clone();
            facts.react_native = repo_context.react_native.clone();

            return Ok(ChangedRepoContextStage {
                repo_context,
                coupling_graph,
                context_graph: load.graph,
                cache_info: load.cache_info,
                diagnostics,
                elapsed_us: start.elapsed().as_micros() as u64,
            });
        }

        let mut repo_context =
            super::collection::collect_scan_facts_without_content(repo_root, self.config)?;

        repo_context.detected_frameworks = detect_frameworks(repo_root);
        repo_context.framework_projects = detect_framework_projects(repo_root);
        repo_context.react_native = detect_react_native_profile(&repo_context);

        let coupling_graph =
            relative_coupling_graph(build_coupling_graph(&repo_context, repo_root), repo_root);
        let context_graph =
            RepoContextGraph::from_scan_facts(&repo_context, repo_root, coupling_graph.clone());
        let mut cache_info =
            context_graph_cache_miss(repo_root, "missing-or-invalid-context-graph-cache");
        match write_repo_context_graph(repo_root, self.config, &context_graph) {
            Ok(_) => {
                cache_info.reason.push_str("; cache-updated");
            }
            Err(error) => diagnostics.push(cache_diagnostic(&error)),
        }

        facts.detected_frameworks = repo_context.detected_frameworks.clone();
        facts.framework_projects = repo_context.framework_projects.clone();
        facts.react_native = repo_context.react_native.clone();

        Ok(ChangedRepoContextStage {
            repo_context,
            coupling_graph,
            context_graph,
            cache_info,
            diagnostics,
            elapsed_us: start.elapsed().as_micros() as u64,
        })
    }

    pub(super) fn score_findings(
        &self,
        repo_stage: &ChangedRepoContextStage,
        findings: &mut [Finding],
    ) -> u64 {
        let start = Instant::now();
        assess_findings(findings, &repo_stage.repo_context);
        apply_graph_overlay(findings, &repo_stage.coupling_graph);
        apply_cluster_overlay(findings);
        start.elapsed().as_micros() as u64
    }
}
