use super::init_suggestions::{CriticalPathCandidate, InitSuggestions, SuggestedCheck};

pub(crate) fn render(suggestions: &InitSuggestions) -> String {
    let mut output = String::from(
        "# RepoPilot init suggestions\n\
# Review this file before copying it into repopilot.toml.\n\
# Suggestions are not executed or loaded automatically.\n\n",
    );

    if suggestions.checks.is_empty() {
        output.push_str("# No executable stack-specific checks were detected.\n");
    } else {
        for check in &suggestions.checks {
            render_check(&mut output, check);
        }
    }

    output.push_str(
        "\n# Critical path candidates are comments only. Add reviewed paths to a\n\
# verification check or repository policy explicitly.\n",
    );
    if suggestions.critical_paths.is_empty() {
        output.push_str("# No critical path candidates were detected.\n");
    } else {
        for candidate in &suggestions.critical_paths {
            render_critical_path(&mut output, candidate);
        }
    }
    output
}

fn render_check(output: &mut String, check: &SuggestedCheck) {
    let Some((program, args, role)) = command_spec(check) else {
        output.push_str(&format!(
            "# Unsupported suggestion {} was not exported (source: {}).\n",
            toml_string(&check.id),
            toml_string(&check.source)
        ));
        return;
    };
    let args = args
        .into_iter()
        .map(toml_string)
        .collect::<Vec<_>>()
        .join(", ");
    output.push_str(&format!(
        "# source: {}\n\
[[verification.checks]]\n\
id = {}\n\
role = {}\n\
program = {}\n\
args = [{}]\n\
working_directory = \".\"\n\
# paths = [\"...\"] # Add only after reviewing the applicable scope.\n\n",
        toml_string(&check.source),
        toml_string(&check.id),
        toml_string(role),
        toml_string(program),
        args
    ));
}

fn command_spec(check: &SuggestedCheck) -> Option<(&'static str, Vec<&'static str>, &'static str)> {
    match check.id.as_str() {
        "rust.test" => Some(("cargo", vec!["test", "--all"], "test")),
        "node.test" => Some(("npm", vec!["test"], "test")),
        "node.lint" => Some(("npm", vec!["run", "lint"], "lint")),
        "node.build" => Some(("npm", vec!["run", "build"], "build")),
        "python.test" => Some(("python3", vec!["-m", "pytest", "-q"], "test")),
        "go.test" => Some(("go", vec!["test", "./..."], "test")),
        "maven.test" => Some(("mvn", vec!["test"], "test")),
        "gradle.test" if check.command.starts_with("./gradlew") => {
            Some(("./gradlew", vec!["test"], "test"))
        }
        "gradle.test" => Some(("gradle", vec!["test"], "test")),
        _ => None,
    }
}

fn render_critical_path(output: &mut String, candidate: &CriticalPathCandidate) {
    output.push_str(&format!(
        "# - {} (source: {})\n",
        candidate.pattern, candidate.source
    ));
}

fn toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::init_suggestions::detect;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn export_is_reviewable_toml_and_keeps_paths_as_comments() {
        let temp = tempdir().expect("temp dir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n",
        )
        .expect("cargo");
        fs::create_dir_all(temp.path().join("src/auth")).expect("auth");

        let rendered = render(&detect(temp.path()));
        let parsed: toml::Value = toml::from_str(&rendered).expect("valid TOML");

        assert_eq!(
            parsed["verification"]["checks"][0]["id"].as_str(),
            Some("rust.test")
        );
        assert!(rendered.contains("# - src/auth/** (source: src/auth)"));
        assert!(rendered.contains("# Suggestions are not executed"));
    }
}
