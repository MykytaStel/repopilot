use crate::findings::types::Severity;
use std::cell::Cell;
use std::env;
use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDestination {
    Stdout,
    File,
}

thread_local! {
    static COLOR_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

pub fn resolve_color_enabled(choice: ColorChoice, destination: ColorDestination) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            destination == ColorDestination::Stdout
                && std::io::stdout().is_terminal()
                && env::var_os("NO_COLOR").is_none()
                && env::var_os("CI").is_none()
        }
    }
}

pub fn with_color_enabled<T>(enabled: bool, render: impl FnOnce() -> T) -> T {
    let _guard = ColorOverrideGuard::new(enabled);
    render()
}

struct ColorOverrideGuard {
    previous: Option<bool>,
}

impl ColorOverrideGuard {
    fn new(enabled: bool) -> Self {
        let previous = COLOR_OVERRIDE.with(|override_cell| {
            let previous = override_cell.get();
            override_cell.set(Some(enabled));
            previous
        });
        Self { previous }
    }
}

impl Drop for ColorOverrideGuard {
    fn drop(&mut self) {
        COLOR_OVERRIDE.with(|override_cell| override_cell.set(self.previous));
    }
}

/// Colors a severity label string like `[HIGH]` or `[Critical]`.
pub fn severity_label(label: &str) -> String {
    if !colors_enabled() {
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
    if !colors_enabled() {
        return text.to_string();
    }
    format!("\x1b[2m{text}\x1b[0m")
}

pub fn status_label(label: &str) -> String {
    if !colors_enabled() {
        return label.to_string();
    }
    match label {
        "Clean" => format!("\x1b[32m{label}\x1b[0m"),
        "Attention needed" | "Scan completed with warnings" => {
            format!("\x1b[33m{label}\x1b[0m")
        }
        "Scan completed with errors" => format!("\x1b[31m{label}\x1b[0m"),
        _ => label.to_string(),
    }
}

pub fn risk_label(label: &str) -> String {
    if !colors_enabled() {
        return label.to_string();
    }
    match label {
        "High" | "Elevated" => format!("\x1b[31m{label}\x1b[0m"),
        "Medium" | "Moderate" => format!("\x1b[33m{label}\x1b[0m"),
        "Low" => format!("\x1b[36m{label}\x1b[0m"),
        _ => label.to_string(),
    }
}

/// Formats `"<n> <label>"` (e.g. `"3 high"`) with the correct severity color.
pub fn severity_count(severity: Severity, n: usize) -> String {
    let text = format!("{n} {}", severity.lowercase_label());
    if !colors_enabled() {
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

fn colors_enabled() -> bool {
    COLOR_OVERRIDE
        .with(|override_cell| override_cell.get())
        .unwrap_or_else(|| resolve_color_enabled(ColorChoice::Auto, ColorDestination::Stdout))
}
