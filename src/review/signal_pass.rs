//! The unified review pass: boundary, behavioral, algorithmic, and taint-lite
//! detection over one shared per-file read/parse instead of each delta type
//! re-reading and re-parsing a changed file's pre/post content independently.
//!
//! Each individual detector's matching logic is untouched — this module only
//! decides, once per changed file, whether the pre- and/or post-change
//! `ReviewSource` is needed at all, builds it at most once per side, and hands
//! the same instance to every detector that wants it. `ReviewSource::tree()`
//! then memoizes the actual tree-sitter parse, so a file inspected by all four
//! delta types is read at most twice (pre/post) and parsed at most twice,
//! instead of up to three reads and seven parses.

use super::content_signals::{ContentSignals, ContentToggles};
use crate::config::model::SecurityBoundarySection;
use crate::review::diff::{ChangedFile, DiffTarget};
use crate::review::signals::api_contract::{self, ChangedReviewSources};
use crate::review::signals::behavioral::{self, DependencyContext};
use crate::review::signals::content::ReviewSource;
use crate::review::signals::{
    BoundaryCategory, BoundarySignal, algorithmic, classify, content, taint,
};
use crate::scan::types::CouplingGraph;
use std::path::Path;

struct LoadedReviewSources {
    boundary_category: Option<BoundaryCategory>,
    needs_ast_fallback: bool,
    pre: Option<ReviewSource>,
    post: Option<ReviewSource>,
}

/// Runs the review detectors over `changed_files` in one pass, sharing each
/// file's pre/post `ReviewSource` (and its memoized parse) across boundary's
/// AST fallback and the three content-based detectors.
pub(super) fn detect_review_signals(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    boundary_config: &SecurityBoundarySection,
    toggles: ContentToggles,
    graph: Option<&CouplingGraph>,
) -> (Vec<BoundarySignal>, ContentSignals) {
    let boundary_enabled = boundary_config.enabled;
    let any_content = toggles.behavioral || toggles.algorithmic || toggles.taint;

    let mut boundary_signals: Vec<BoundarySignal> = Vec::new();
    let mut content_signals = ContentSignals::default();

    if !boundary_enabled && !any_content {
        return (boundary_signals, content_signals);
    }

    let custom = boundary_enabled
        .then(|| classify::build_custom_globset(&boundary_config.extra_patterns))
        .flatten();
    let dependencies = toggles
        .behavioral
        .then(|| DependencyContext::from_repo_root(repo_root));

    let loaded_sources = load_review_sources(
        repo_root,
        target,
        changed_files,
        boundary_enabled,
        any_content,
        custom.as_ref(),
        &toggles,
    );
    for (file, sources) in changed_files.iter().zip(&loaded_sources) {
        detect_file_signals(
            file,
            sources,
            &toggles,
            dependencies.as_ref(),
            &mut boundary_signals,
            &mut content_signals,
        );
    }

    if toggles.behavioral {
        content_signals.api_contract =
            detect_api_contract(repo_root, target, changed_files, &loaded_sources, graph);
    }

    boundary_signals.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.path.cmp(&right.path))
    });

    (boundary_signals, content_signals)
}

#[allow(clippy::too_many_arguments)]
fn load_review_sources(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    boundary_enabled: bool,
    any_content: bool,
    custom: Option<&globset::GlobSet>,
    toggles: &ContentToggles,
) -> Vec<LoadedReviewSources> {
    changed_files
        .iter()
        .map(|file| {
            let is_test = crate::audits::context::classify::helpers::is_test_file(&file.path);
            let boundary_category = (boundary_enabled && !is_test)
                .then(|| classify::classify_boundary(&file.path_string(), custom))
                .flatten();
            let needs_ast_fallback = boundary_enabled && !is_test && boundary_category.is_none();
            let post = (any_content || needs_ast_fallback)
                .then(|| content::post_change_source(repo_root, file, target))
                .flatten();
            let pre = (toggles.behavioral || toggles.algorithmic)
                .then(|| content::pre_change_source(repo_root, file, target))
                .flatten();
            LoadedReviewSources {
                boundary_category,
                needs_ast_fallback,
                pre,
                post,
            }
        })
        .collect()
}

fn detect_file_signals(
    file: &ChangedFile,
    sources: &LoadedReviewSources,
    toggles: &ContentToggles,
    dependencies: Option<&DependencyContext>,
    boundary_signals: &mut Vec<BoundarySignal>,
    content_signals: &mut ContentSignals,
) {
    let mut boundary_category = sources.boundary_category;
    if sources.needs_ast_fallback
        && let Some(post) = &sources.post
    {
        boundary_category = classify::classify_boundary_ast_from_source(post);
    }
    if let Some(category) = boundary_category {
        boundary_signals.push(BoundarySignal {
            category,
            path: file.path_string(),
            status: file.status,
            blast_radius: 0,
        });
    }
    if toggles.behavioral {
        if let (Some(post), Some(deps)) = (&sources.post, dependencies) {
            content_signals
                .behavioral
                .extend(behavioral::detect_behavioral_added(file, post, deps));
        }
        content_signals
            .behavioral
            .extend(behavioral::detect_behavioral_removed(
                file,
                sources.pre.as_ref(),
                sources.post.as_ref(),
            ));
    }
    if toggles.algorithmic {
        content_signals
            .algorithmic
            .extend(algorithmic::detect_algorithmic(
                file,
                sources.pre.as_ref(),
                sources.post.as_ref(),
            ));
    }
    if toggles.taint {
        content_signals
            .taint
            .extend(taint::detect_taint(file, sources.post.as_ref()));
    }
}

fn detect_api_contract(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    loaded_sources: &[LoadedReviewSources],
    graph: Option<&CouplingGraph>,
) -> Vec<api_contract::RemovedExportSignal> {
    let changed_sources = changed_files
        .iter()
        .zip(loaded_sources)
        .map(|(file, sources)| ChangedReviewSources {
            file,
            pre: sources.pre.as_ref(),
            post: sources.post.as_ref(),
        })
        .collect::<Vec<_>>();
    api_contract::detect_removed_export_imports(repo_root, target, &changed_sources, graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::SecurityBoundarySection;
    use crate::review::diff::{ChangeStatus, DiffTarget};
    use crate::scan::types::CouplingGraph;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn behavioral_toggle_controls_removed_export_detection() {
        // Catches API-contract detection bypassing the behavioral toggle or
        // being omitted from the shared source-loading pass.
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        git(root, &["init"]);
        git(root, &["config", "user.email", "repopilot@example.invalid"]);
        git(root, &["config", "user.name", "RepoPilot Test"]);
        write(root, "src/api.ts", "export function loadUser() {}\n");
        write(
            root,
            "src/caller.ts",
            "import { loadUser } from \"./api.ts\";\n",
        );
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "before"]);
        write(root, "src/api.ts", "export function saveUser() {}\n");

        let changed_files = vec![ChangedFile {
            path: PathBuf::from("src/api.ts"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }];
        let mut graph = CouplingGraph::default();
        graph.nodes.insert(PathBuf::from("src/api.ts"));
        graph.nodes.insert(PathBuf::from("src/caller.ts"));
        graph
            .edges
            .entry(PathBuf::from("src/caller.ts"))
            .or_default()
            .insert(PathBuf::from("src/api.ts"));

        let (_, enabled) = detect_review_signals(
            root,
            DiffTarget::WorkingTree,
            &changed_files,
            &SecurityBoundarySection {
                enabled: false,
                extra_patterns: Vec::new(),
            },
            ContentToggles {
                behavioral: true,
                algorithmic: false,
                taint: false,
            },
            Some(&graph),
        );
        assert_eq!(enabled.api_contract.len(), 1);
        assert_eq!(enabled.api_contract[0].exported_name, "loadUser");

        let (_, disabled) = detect_review_signals(
            root,
            DiffTarget::WorkingTree,
            &changed_files,
            &SecurityBoundarySection {
                enabled: false,
                extra_patterns: Vec::new(),
            },
            ContentToggles {
                behavioral: false,
                algorithmic: false,
                taint: false,
            },
            Some(&graph),
        );
        assert!(disabled.api_contract.is_empty());
    }

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
        fs::write(path, content).expect("write source");
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
