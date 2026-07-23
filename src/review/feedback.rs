use crate::knowledge::overlay::{OverlayEntry, OverlayTarget, active_overlay};
use crate::review::signals::tiered::{ReviewSignal, TieredSignals};
use crate::scan::types::ScanSummary;
use std::path::Path;

pub(super) fn apply_review_feedback(
    tiered: &mut TieredSignals,
    summary: &mut ScanSummary,
    _repo_root: &Path,
) {
    let entries = active_overlay().entries();
    if entries.is_empty() {
        return;
    }
    let mut suppressed = 0;
    for signal in tiered.iter_mut() {
        let Some(entry) = overlay_review_suppression(entries, signal) else {
            continue;
        };
        signal.suppressed = true;
        signal.gate_eligible = false;
        signal.suppression_reason = entry.reason.clone();
        suppressed += 1;
    }
    if let Some(report) = summary.local_feedback.as_mut() {
        report.suppressed_review_signals_count = suppressed;
    }
}

pub(super) fn overlay_review_suppression<'a>(
    entries: &'a [OverlayEntry],
    signal: &ReviewSignal,
) -> Option<&'a OverlayEntry> {
    entries.iter().find(|entry| {
        let OverlayTarget::Kind(kind) = &entry.target else {
            return false;
        };
        if kind != &signal.kind {
            return false;
        }
        if let Some(expires) = entry.expires
            && expires < chrono::Utc::now().date_naive()
        {
            return false;
        }
        match &entry.path_glob {
            None => true,
            Some(glob) => glob.is_match(&signal.path),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::provenance::AnalysisScope;
    use crate::findings::types::Confidence;
    use crate::knowledge::overlay::{OverlayEntry, OverlayTarget};
    use crate::review::signals::tiered::{
        ConfidenceTier, ReviewSignal, ReviewSignalProvenance, SignalFamily,
    };
    use crate::rules::{RuleLifecycle, SignalSource};

    fn kind_entry(kind: &str, path: &str) -> OverlayEntry {
        OverlayEntry {
            index: 1,
            target: OverlayTarget::Kind(kind.to_string()),
            path_text: Some(path.to_string()),
            path_glob: Some(globset::Glob::new(path).unwrap().compile_matcher()),
            severity: None,
            reason: Some("ops scripts shell out on purpose".to_string()),
            expires: None,
        }
    }

    fn sample_signal(kind: &str, path: &str) -> ReviewSignal {
        ReviewSignal {
            signal_id: "test-signal".to_string(),
            kind: kind.to_string(),
            family: SignalFamily::Behavioral,
            tier: ConfidenceTier::MaybeSensitive,
            confidence: Confidence::Medium,
            path: path.to_string(),
            line: None,
            line_start: None,
            line_end: None,
            evidence_lines: Vec::new(),
            headline: "test headline".to_string(),
            detail: None,
            blast_radius: 0,
            provenance: ReviewSignalProvenance {
                detector: "test".to_string(),
                lifecycle: RuleLifecycle::Preview,
                signal_source: SignalSource::TextHeuristic,
                analysis_scope: AnalysisScope::File,
            },
            suppressed: false,
            suppression_reason: None,
            gate_eligible: true,
            verification_plan: None,
        }
    }

    #[test]
    fn suppresses_a_matching_kind_and_path() {
        let entries = vec![kind_entry("behavioral", "scripts/**")];
        let signal = sample_signal("behavioral", "scripts/deploy.sh");

        assert!(overlay_review_suppression(&entries, &signal).is_some());
    }

    #[test]
    fn does_not_suppress_a_non_matching_path() {
        let entries = vec![kind_entry("behavioral", "scripts/**")];
        let signal = sample_signal("behavioral", "src/main.rs");

        assert!(overlay_review_suppression(&entries, &signal).is_none());
    }
}
