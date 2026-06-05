use crate::review::diff::{ChangedFile, DiffTarget};
use crate::review::signals::algorithmic::{self, AlgorithmicSignal};
use crate::review::signals::behavioral::{self, BehavioralSignal};
use crate::review::signals::content;
use std::path::Path;

/// Run the content-based detectors (behavioral + algorithmic) over each changed
/// file, re-reading its pre/post source via the content bridge. Each is gated by
/// its config toggle; both off means no git reads at all.
pub(super) fn detect_content_signals(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    behavioral_enabled: bool,
    algorithmic_enabled: bool,
) -> (Vec<BehavioralSignal>, Vec<AlgorithmicSignal>) {
    let mut behavioral = Vec::new();
    let mut algorithmic = Vec::new();
    if !behavioral_enabled && !algorithmic_enabled {
        return (behavioral, algorithmic);
    }

    for file in changed_files {
        let post = content::post_change_source(repo_root, file, target);
        let pre = content::pre_change_source(repo_root, file, target);

        if behavioral_enabled {
            if let Some(post) = &post {
                behavioral.extend(behavioral::detect_behavioral_added(file, post));
            }
            behavioral.extend(behavioral::detect_behavioral_removed(
                file,
                pre.as_ref(),
                post.as_ref(),
            ));
        }
        if algorithmic_enabled {
            algorithmic.extend(algorithmic::detect_algorithmic(
                file,
                pre.as_ref(),
                post.as_ref(),
            ));
        }
    }

    (behavioral, algorithmic)
}
