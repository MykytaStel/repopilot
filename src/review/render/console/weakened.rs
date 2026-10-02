//! "Checks this change weakened": integrity signals listed right after the
//! decision. They are what a green run hides, so they should not wait at the
//! bottom of a long review. The full entries, with details and tiers, stay in
//! "Review signals" below, which agent hooks also read.

use crate::review::model::ReviewReport;
use crate::review::signals::tiered::{ReviewSignal, SignalFamily};

const SHOWN: usize = 10;

pub(super) fn render(output: &mut String, report: &ReviewReport) {
    let tiered = &report.tiered_signals;
    let weakened: Vec<&ReviewSignal> = tiered
        .definitely
        .iter()
        .chain(&tiered.maybe)
        .chain(&tiered.noise)
        .filter(|signal| signal.family == SignalFamily::Integrity && !signal.suppressed)
        .collect();
    if weakened.is_empty() {
        return;
    }
    output.push_str("\nChecks this change weakened (details under Review signals):\n");
    for signal in weakened.iter().take(SHOWN) {
        let location = match signal.line {
            Some(line) if !signal.path.is_empty() => format!(" \u{2014} {}:{line}", signal.path),
            _ if !signal.path.is_empty() => format!(" \u{2014} {}", signal.path),
            _ => String::new(),
        };
        output.push_str(&format!("  \u{2691} {}{location}\n", signal.headline));
    }
    if weakened.len() > SHOWN {
        output.push_str(&format!("  ... and {} more\n", weakened.len() - SHOWN));
    }
    output.push('\n');
}
