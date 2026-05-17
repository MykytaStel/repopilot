use super::model::KnowledgeCatalogReport;
use crate::output::OutputFormat;

pub fn render_knowledge_catalog_report(
    report: &KnowledgeCatalogReport,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_console(report)),
        OutputFormat::Markdown => Ok(render_markdown(report)),
        OutputFormat::Json | OutputFormat::Html | OutputFormat::Sarif => {
            serde_json::to_string_pretty(report)
        }
    }
}

fn render_console(report: &KnowledgeCatalogReport) -> String {
    let mut output = String::new();

    output.push_str("RepoPilot Knowledge Catalog\n\n");
    output.push_str("Summary:\n");
    output.push_str(&format!(" Languages: {}\n", report.summary.languages));
    output.push_str(&format!(" Frameworks: {}\n", report.summary.frameworks));
    output.push_str(&format!(" Runtimes: {}\n", report.summary.runtimes));
    output.push_str(&format!(" Paradigms: {}\n", report.summary.paradigms));
    output.push_str(&format!(" Rules: {}\n", report.summary.rules));

    if !report.languages.is_empty() {
        output.push_str("\nLanguages:\n");
        for language in &report.languages {
            output.push_str(&format!(
                " - {} ({}) [{}]\n",
                language.name, language.id, language.support
            ));
            if !language.extensions.is_empty() {
                output.push_str(&format!(
                    "   extensions: {}\n",
                    language.extensions.join(", ")
                ));
            }
            if !language.filenames.is_empty() {
                output.push_str(&format!(
                    "   filenames: {}\n",
                    language.filenames.join(", ")
                ));
            }
        }
    }

    if !report.frameworks.is_empty() {
        output.push_str("\nFrameworks:\n");
        for framework in &report.frameworks {
            output.push_str(&format!(" - {} ({})\n", framework.name, framework.id));
        }
    }

    if !report.runtimes.is_empty() {
        output.push_str("\nRuntimes:\n");
        for runtime in &report.runtimes {
            output.push_str(&format!(" - {} ({})\n", runtime.name, runtime.id));
        }
    }

    if !report.paradigms.is_empty() {
        output.push_str("\nParadigms:\n");
        for paradigm in &report.paradigms {
            output.push_str(&format!(" - {} ({})\n", paradigm.name, paradigm.id));
        }
    }

    if !report.rules.is_empty() {
        output.push_str("\nRules:\n");
        for rule in &report.rules {
            output.push_str(&format!(" - {}\n", rule.rule_id));
            if let Some(minimum_support) = &rule.minimum_support {
                output.push_str(&format!("   minimum support: {minimum_support}\n"));
            }
            output.push_str(&format!(
                "   languages: {}\n",
                comma_or_all(&rule.languages)
            ));
            output.push_str(&format!(
                "   frameworks: {}\n",
                comma_or_all(&rule.frameworks)
            ));
            output.push_str(&format!("   runtimes: {}\n", comma_or_all(&rule.runtimes)));
            output.push_str(&format!(
                "   paradigms: {}\n",
                comma_or_all(&rule.paradigms)
            ));
            output.push_str(&format!("   overrides: {}\n", rule.overrides));
        }
    }

    output
}

fn render_markdown(report: &KnowledgeCatalogReport) -> String {
    let mut output = String::new();

    output.push_str("# RepoPilot Knowledge Catalog\n\n");
    output.push_str("## Summary\n\n");
    output.push_str(&format!("- **Languages:** {}\n", report.summary.languages));
    output.push_str(&format!(
        "- **Frameworks:** {}\n",
        report.summary.frameworks
    ));
    output.push_str(&format!("- **Runtimes:** {}\n", report.summary.runtimes));
    output.push_str(&format!("- **Paradigms:** {}\n", report.summary.paradigms));
    output.push_str(&format!("- **Rules:** {}\n", report.summary.rules));

    if !report.languages.is_empty() {
        output.push_str("\n## Languages\n\n");
        for language in &report.languages {
            output.push_str(&format!(
                "- `{}` — {} — support: `{}`\n",
                language.id, language.name, language.support
            ));
            if !language.extensions.is_empty() {
                output.push_str(&format!(
                    "  - extensions: {}\n",
                    markdown_code_list(&language.extensions)
                ));
            }
            if !language.filenames.is_empty() {
                output.push_str(&format!(
                    "  - filenames: {}\n",
                    markdown_code_list(&language.filenames)
                ));
            }
        }
    }

    if !report.frameworks.is_empty() {
        output.push_str("\n## Frameworks\n\n");
        for framework in &report.frameworks {
            output.push_str(&format!("- `{}` — {}\n", framework.id, framework.name));
        }
    }

    if !report.runtimes.is_empty() {
        output.push_str("\n## Runtimes\n\n");
        for runtime in &report.runtimes {
            output.push_str(&format!("- `{}` — {}\n", runtime.id, runtime.name));
        }
    }

    if !report.paradigms.is_empty() {
        output.push_str("\n## Paradigms\n\n");
        for paradigm in &report.paradigms {
            output.push_str(&format!("- `{}` — {}\n", paradigm.id, paradigm.name));
        }
    }

    if !report.rules.is_empty() {
        output.push_str("\n## Rules\n\n");
        for rule in &report.rules {
            output.push_str(&format!("- `{}`\n", rule.rule_id));
            if let Some(minimum_support) = &rule.minimum_support {
                output.push_str(&format!("  - minimum support: `{minimum_support}`\n"));
            }
            output.push_str(&format!(
                "  - languages: {}\n",
                markdown_code_list_or_all(&rule.languages)
            ));
            output.push_str(&format!(
                "  - frameworks: {}\n",
                markdown_code_list_or_all(&rule.frameworks)
            ));
            output.push_str(&format!(
                "  - runtimes: {}\n",
                markdown_code_list_or_all(&rule.runtimes)
            ));
            output.push_str(&format!(
                "  - paradigms: {}\n",
                markdown_code_list_or_all(&rule.paradigms)
            ));
            output.push_str(&format!("  - overrides: `{}`\n", rule.overrides));
        }
    }

    output
}

fn comma_or_all(values: &[String]) -> String {
    if values.is_empty() {
        "all".to_string()
    } else {
        values.join(", ")
    }
}

fn markdown_code_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn markdown_code_list_or_all(values: &[String]) -> String {
    if values.is_empty() {
        "`all`".to_string()
    } else {
        markdown_code_list(values)
    }
}
