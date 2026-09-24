use crate::scan::types::{DiagnosticSeverity, ScanSummary};
use std::fmt::Write;

/// Compact output keeps warnings and errors on the first screen and folds
/// informational diagnostics into one count; full detail lists everything.
pub(super) fn render_console(output: &mut String, summary: &ScanSummary, compact: bool) {
    let diagnostics = &summary.artifacts.diagnostics;
    if diagnostics.is_empty() {
        return;
    }
    let hidden = if compact {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Info)
            .count()
    } else {
        0
    };
    if hidden == diagnostics.len() {
        let _ = writeln!(
            output,
            "Diagnostics: {hidden} informational; rerun with --detail full to list them"
        );
        return;
    }

    output.push_str("Diagnostics:\n");
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| !compact || diagnostic.severity != DiagnosticSeverity::Info)
    {
        let _ = writeln!(
            output,
            "  [{}] {}: {}",
            severity_label(diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        );
    }
    if hidden > 0 {
        let _ = writeln!(
            output,
            "  ... {hidden} informational diagnostic(s); rerun with --detail full"
        );
    }
}

pub(super) fn render_markdown(output: &mut String, summary: &ScanSummary) {
    if summary.artifacts.diagnostics.is_empty() {
        return;
    }

    output.push_str("- **Diagnostics:**\n");
    for diagnostic in &summary.artifacts.diagnostics {
        let _ = writeln!(
            output,
            "  - `[{}] {}`: {}",
            severity_label(diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        );
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}
