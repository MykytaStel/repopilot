//! Detail lines for the compact scan header: changed-files scope, file
//! accounting, diagnostics, and local-feedback summaries. Driven by
//! `render_summary_header` in the parent `compact` module.

use crate::scan::types::{DiagnosticSeverity, ScanSummary};
use std::fmt::Write;

pub(super) fn render_scope(output: &mut String, summary: &ScanSummary) {
    if summary.mode != crate::scan::types::ScanMode::Changed {
        return;
    }

    let base = summary
        .base_ref
        .as_ref()
        .map(|base| format!(" since {base}"))
        .unwrap_or_else(|| " against HEAD".to_string());
    writeln!(
        output,
        "Scope: changed files{base} ({})",
        summary.metrics.changed_files_count
    )
    .unwrap();
}

pub(super) fn render_scope_accounting(output: &mut String, summary: &ScanSummary) {
    let skipped = skipped_files_count(summary);
    if skipped > 0 {
        writeln!(
            output,
            "Files: {} discovered, {} analyzed, {skipped} skipped",
            summary.metrics.files_discovered, summary.metrics.files_analyzed
        )
        .unwrap();
    } else {
        writeln!(
            output,
            "Files: {} discovered, {} analyzed",
            summary.metrics.files_discovered, summary.metrics.files_analyzed
        )
        .unwrap();
    }
}

fn skipped_files_count(summary: &ScanSummary) -> usize {
    summary
        .metrics
        .files_skipped_low_signal
        .saturating_add(summary.metrics.files_skipped_by_limit)
        .saturating_add(summary.metrics.large_files_skipped)
        .saturating_add(summary.metrics.binary_files_skipped)
        .saturating_add(summary.metrics.files_skipped_repopilotignore)
}

pub(super) fn render_diagnostics_line(output: &mut String, summary: &ScanSummary) {
    if summary.artifacts.diagnostics.is_empty() {
        return;
    }

    let warnings = summary
        .artifacts
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
        .count();
    let errors = summary
        .artifacts
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .count();

    if errors > 0 && warnings > 0 {
        writeln!(
            output,
            "Diagnostics: {errors} error(s), {warnings} warning(s)"
        )
        .unwrap();
    } else if errors > 0 {
        writeln!(output, "Diagnostics: {errors} error(s)").unwrap();
    } else if warnings > 0 {
        writeln!(output, "Diagnostics: {warnings} warning(s)").unwrap();
    }
}

pub(super) fn render_local_feedback_line(output: &mut String, summary: &ScanSummary) {
    let Some(feedback) = &summary.local_feedback else {
        return;
    };

    if feedback.suppressed_findings_count == 0
        && feedback.unmatched_suppressions_count == 0
        && feedback.invalid_suppressions_count == 0
    {
        return;
    }

    writeln!(
        output,
        "Local feedback: {} suppressed, {} unmatched, {} invalid",
        feedback.suppressed_findings_count,
        feedback.unmatched_suppressions_count,
        feedback.invalid_suppressions_count
    )
    .unwrap();
}
