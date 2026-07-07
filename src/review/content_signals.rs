use crate::review::signals::algorithmic::AlgorithmicSignal;
use crate::review::signals::behavioral::BehavioralSignal;
use crate::review::signals::taint::TaintSignal;

/// Content-based review signals detected over the changed files' re-read source.
///
/// Produced by [`super::signal_pass::detect_review_signals`], which runs the
/// behavioral, algorithmic, and taint-lite detectors alongside boundary
/// detection in one shared pass over each changed file.
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
