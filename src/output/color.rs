use crate::findings::types::Severity;
use std::io::IsTerminal;

/// Colors a severity label string like `[HIGH]` or `[Critical]`.
pub fn severity_label(label: &str) -> String {
    if !std::io::stdout().is_terminal() {
        return label.to_string();
    }
    match label {
        "Critical" | "CRITICAL" => format!("\x1b[1;31m{label}\x1b[0m"),
        "High" | "HIGH" => format!("\x1b[31m{label}\x1b[0m"),
        "Medium" | "MEDIUM" => format!("\x1b[33m{label}\x1b[0m"),
        "Low" | "LOW" => format!("\x1b[36m{label}\x1b[0m"),
        _ => format!("\x1b[90m{label}\x1b[0m"),
    }
}

/// Renders text in dim/faint style for secondary information.
pub fn dim(text: &str) -> String {
    if !std::io::stdout().is_terminal() {
        return text.to_string();
    }
    format!("\x1b[2m{text}\x1b[0m")
}

/// Formats `"<n> <label>"` (e.g. `"3 high"`) with the correct severity color.
pub fn severity_count(severity: Severity, n: usize) -> String {
    let text = format!("{n} {}", severity.lowercase_label());
    if !std::io::stdout().is_terminal() {
        return text;
    }
    match severity {
        Severity::Critical => format!("\x1b[1;31m{text}\x1b[0m"),
        Severity::High => format!("\x1b[31m{text}\x1b[0m"),
        Severity::Medium => format!("\x1b[33m{text}\x1b[0m"),
        Severity::Low => format!("\x1b[36m{text}\x1b[0m"),
        Severity::Info => format!("\x1b[90m{text}\x1b[0m"),
    }
}
