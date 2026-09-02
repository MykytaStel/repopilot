use crate::scan::types::{DiagnosticSeverity, ScanSummary};
use std::fmt::Write;

pub(super) fn render_console(output: &mut String, summary: &ScanSummary) {
    if summary.artifacts.diagnostics.is_empty() {
        return;
    }

    output.push_str("Diagnostics:\n");
    for diagnostic in &summary.artifacts.diagnostics {
        writeln!(
            output,
            "  [{}] {}: {}",
            severity_label(diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        )
        .unwrap();
    }
}

pub(super) fn render_markdown(output: &mut String, summary: &ScanSummary) {
    if summary.artifacts.diagnostics.is_empty() {
        return;
    }

    output.push_str("- **Diagnostics:**\n");
    for diagnostic in &summary.artifacts.diagnostics {
        writeln!(
            output,
            "  - `[{}] {}`: {}",
            severity_label(diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        )
        .unwrap();
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}
