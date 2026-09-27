use super::super::content_signals::ContentToggles;
use crate::review::diff::{ChangedFile, DiffTarget};
use crate::review::signals::content::ReviewSource;
use crate::review::signals::{BoundaryCategory, classify, content};
use std::path::Path;

pub(super) struct LoadedReviewSources {
    pub(super) boundary_category: Option<BoundaryCategory>,
    pub(super) needs_ast_fallback: bool,
    pub(super) pre: Option<ReviewSource>,
    pub(super) post: Option<ReviewSource>,
}

pub(super) fn load_review_sources(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    boundary_enabled: bool,
    any_content: bool,
    custom: Option<&globset::GlobSet>,
    toggles: &ContentToggles,
) -> Vec<LoadedReviewSources> {
    let pre_sources = content::pre_change_sources(repo_root, target, changed_files, toggles);
    let post_required = changed_files
        .iter()
        .map(|file| {
            let is_test = crate::audits::context::classify::helpers::is_test_file(&file.path);
            let has_boundary = (boundary_enabled && !is_test)
                .then(|| classify::classify_boundary(&file.path_string(), custom))
                .flatten()
                .is_some();
            any_content || (boundary_enabled && !is_test && !has_boundary)
        })
        .collect::<Vec<_>>();
    let post_sources =
        content::post_change_sources(repo_root, target, changed_files, &post_required);

    changed_files
        .iter()
        .zip(pre_sources)
        .zip(post_sources)
        .map(|((file, pre), post)| {
            let is_test = crate::audits::context::classify::helpers::is_test_file(&file.path);
            let boundary_category = (boundary_enabled && !is_test)
                .then(|| classify::classify_boundary(&file.path_string(), custom))
                .flatten();
            let needs_ast_fallback = boundary_enabled && !is_test && boundary_category.is_none();
            LoadedReviewSources {
                boundary_category,
                needs_ast_fallback,
                pre,
                post,
            }
        })
        .collect()
}
