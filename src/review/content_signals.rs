use crate::review::diff::{ChangedFile, DiffTarget};
use crate::review::signals::algorithmic::{self, AlgorithmicSignal};
use crate::review::signals::behavioral::{self, BehavioralSignal, DependencyContext};
use crate::review::signals::content;
use crate::review::signals::taint::{self, TaintSignal};
use std::path::Path;

/// Content-based review signals detected over the changed files' re-read source.
pub(super) struct ContentSignals {
    pub behavioral: Vec<BehavioralSignal>,
    pub algorithmic: Vec<AlgorithmicSignal>,
    pub taint: Vec<TaintSignal>,
}

/// Which content-based detectors to run; each maps to a config toggle.
pub(super) struct ContentToggles {
    pub behavioral: bool,
    pub algorithmic: bool,
    pub taint: bool,
}

/// Run the content-based detectors (behavioral, algorithmic, taint-lite) over each
/// changed file, re-reading its pre/post source via the content bridge once. Each
/// detector is gated by its toggle; all off means no git reads at all.
pub(super) fn detect_content_signals(
    repo_root: &Path,
    target: DiffTarget<'_>,
    changed_files: &[ChangedFile],
    toggles: ContentToggles,
) -> ContentSignals {
    let mut signals = ContentSignals {
        behavioral: Vec::new(),
        algorithmic: Vec::new(),
        taint: Vec::new(),
    };
    if !toggles.behavioral && !toggles.algorithmic && !toggles.taint {
        return signals;
    }
    let dependencies = DependencyContext::from_repo_root(repo_root);

    for file in changed_files {
        let post = content::post_change_source(repo_root, file, target);
        let pre = if toggles.behavioral || toggles.algorithmic {
            content::pre_change_source(repo_root, file, target)
        } else {
            None
        };

        if toggles.behavioral {
            if let Some(post) = &post {
                signals
                    .behavioral
                    .extend(behavioral::detect_behavioral_added(
                        file,
                        post,
                        &dependencies,
                    ));
            }
            signals
                .behavioral
                .extend(behavioral::detect_behavioral_removed(
                    file,
                    pre.as_ref(),
                    post.as_ref(),
                ));
        }
        if toggles.algorithmic {
            signals.algorithmic.extend(algorithmic::detect_algorithmic(
                file,
                pre.as_ref(),
                post.as_ref(),
            ));
        }
        if toggles.taint {
            signals
                .taint
                .extend(taint::detect_taint(file, post.as_ref()));
        }
    }

    signals
}
