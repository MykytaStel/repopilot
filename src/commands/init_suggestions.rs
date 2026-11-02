mod detect;

pub(crate) use detect::detect;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetectedStack {
    pub(crate) name: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SuggestedCheck {
    pub(crate) id: String,
    pub(crate) command: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CriticalPathCandidate {
    pub(crate) pattern: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitSuggestions {
    pub(crate) stacks: Vec<DetectedStack>,
    pub(crate) checks: Vec<SuggestedCheck>,
    pub(crate) configured_check_ids: Vec<String>,
    pub(crate) config_comparison_available: bool,
    pub(crate) critical_paths: Vec<CriticalPathCandidate>,
}

pub(crate) fn render(suggestions: &InitSuggestions) -> String {
    let mut output = String::new();
    render_stacks(&mut output, suggestions);
    render_checks(&mut output, suggestions);
    render_critical_paths(&mut output, suggestions);
    output.push_str("No commands were run; these are deterministic suggestions only.\n");
    output
}

fn render_stacks(output: &mut String, suggestions: &InitSuggestions) {
    if suggestions.stacks.is_empty() {
        output.push_str("Detected stack: unknown\n");
    } else {
        output.push_str(&format!(
            "Detected stack: {}\n",
            suggestions
                .stacks
                .iter()
                .map(|stack| format!("{} ({})", stack.name, stack.source))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}

fn render_checks(output: &mut String, suggestions: &InitSuggestions) {
    if suggestions.checks.is_empty() {
        if suggestions.config_comparison_available && !suggestions.configured_check_ids.is_empty() {
            output.push_str(
                "No additional stack-specific verification checks suggested; detected checks are already configured.\n",
            );
        } else {
            output.push_str("No stack-specific verification checks detected.\n");
        }
    } else {
        output.push_str("Suggested verification checks (not run):\n");
        for check in &suggestions.checks {
            output.push_str(&format!(
                "  - {}: {} (source: {})\n",
                check.id, check.command, check.source
            ));
        }
    }

    if !suggestions.configured_check_ids.is_empty() {
        output.push_str(&format!(
            "Already configured verification checks: {} (omitted from suggestions)\n",
            suggestions.configured_check_ids.join(", ")
        ));
    }
    if !suggestions.config_comparison_available {
        output.push_str(
            "Configured-check comparison unavailable; all detected suggestions remain listed.\n",
        );
    }
}

fn render_critical_paths(output: &mut String, suggestions: &InitSuggestions) {
    if suggestions.critical_paths.is_empty() {
        output.push_str("No critical path candidates detected.\n");
    } else {
        output.push_str("Critical path candidates (review before committing):\n");
        for path in &suggestions.critical_paths {
            output.push_str(&format!("  - {} (source: {})\n", path.pattern, path.source));
        }
    }
}

#[cfg(test)]
mod tests;
