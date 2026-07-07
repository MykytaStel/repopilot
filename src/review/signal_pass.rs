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
use crate::review::signals::behavioral::{self, DependencyContext};
use crate::review::signals::{BoundarySignal, algorithmic, classify, content, taint};
use std::path::Path;

/// Runs all four delta types over `changed_files` in one pass, sharing each
/// file's pre/post `ReviewSource` (and its memoized parse) across boundary's
/// AST fallback and the three content-based detectors.
pub(super) fn detect_review_signals(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    boundary_config: &SecurityBoundarySection,
    toggles: ContentToggles,
) -> (Vec<BoundarySignal>, ContentSignals) {
    let boundary_enabled = boundary_config.enabled;
    let any_content = toggles.behavioral || toggles.algorithmic || toggles.taint;

    let mut boundary_signals: Vec<BoundarySignal> = Vec::new();
    let mut content_signals = ContentSignals {
        behavioral: Vec::new(),
        algorithmic: Vec::new(),
        taint: Vec::new(),
    };

    if !boundary_enabled && !any_content {
        return (boundary_signals, content_signals);
    }

    let custom = boundary_enabled
        .then(|| classify::build_custom_globset(&boundary_config.extra_patterns))
        .flatten();
    let dependencies = toggles
        .behavioral
        .then(|| DependencyContext::from_repo_root(repo_root));

    for file in changed_files {
        let is_test = crate::audits::context::classify::helpers::is_test_file(&file.path);

        let mut boundary_category = if boundary_enabled && !is_test {
            classify::classify_boundary(&file.path_string(), custom.as_ref())
        } else {
            None
        };
        let needs_ast_fallback = boundary_enabled && !is_test && boundary_category.is_none();

        let need_post = any_content || needs_ast_fallback;
        let post = if need_post {
            content::post_change_source(repo_root, file, target)
        } else {
            None
        };
        let need_pre = toggles.behavioral || toggles.algorithmic;
        let pre = if need_pre {
            content::pre_change_source(repo_root, file, target)
        } else {
            None
        };

        if needs_ast_fallback && let Some(post_source) = &post {
            boundary_category = classify::classify_boundary_ast_from_source(post_source);
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
            if let (Some(post_source), Some(deps)) = (&post, &dependencies) {
                content_signals
                    .behavioral
                    .extend(behavioral::detect_behavioral_added(file, post_source, deps));
            }
            content_signals
                .behavioral
                .extend(behavioral::detect_behavioral_removed(
                    file,
                    pre.as_ref(),
                    post.as_ref(),
                ));
        }
        if toggles.algorithmic {
            content_signals
                .algorithmic
                .extend(algorithmic::detect_algorithmic(
                    file,
                    pre.as_ref(),
                    post.as_ref(),
                ));
        }
        if toggles.taint {
            content_signals
                .taint
                .extend(taint::detect_taint(file, post.as_ref()));
        }
    }

    boundary_signals.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.path.cmp(&right.path))
    });

    (boundary_signals, content_signals)
}
