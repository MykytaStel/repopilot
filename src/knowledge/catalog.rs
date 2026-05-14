use crate::knowledge::bundled_knowledge;
use crate::knowledge::model::SupportLevel;
use crate::output::OutputFormat;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeCatalogSection {
    All,
    Languages,
    Frameworks,
    Runtimes,
    Paradigms,
    Rules,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeCatalogReport {
    pub summary: KnowledgeCatalogSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<KnowledgeLanguageEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<KnowledgeNamedEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runtimes: Vec<KnowledgeNamedEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub paradigms: Vec<KnowledgeNamedEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<KnowledgeRuleEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeCatalogSummary {
    pub languages: usize,
    pub frameworks: usize,
    pub runtimes: usize,
    pub paradigms: usize,
    pub rules: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeLanguageEntry {
    pub id: String,
    pub name: String,
    pub support: String,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeNamedEntry {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeRuleEntry {
    pub rule_id: String,
    pub minimum_support: Option<String>,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub runtimes: Vec<String>,
    pub paradigms: Vec<String>,
    pub suppress_low_signal: bool,
    pub suppress_generated: bool,
    pub suppress_config: bool,
    pub overrides: usize,
}

pub fn build_knowledge_catalog_report(
    section: KnowledgeCatalogSection,
) -> Result<KnowledgeCatalogReport, Box<dyn std::error::Error>> {
    let knowledge = bundled_knowledge();

    let include_languages = matches!(
        section,
        KnowledgeCatalogSection::All | KnowledgeCatalogSection::Languages
    );
    let include_frameworks = matches!(
        section,
        KnowledgeCatalogSection::All | KnowledgeCatalogSection::Frameworks
    );
    let include_runtimes = matches!(
        section,
        KnowledgeCatalogSection::All | KnowledgeCatalogSection::Runtimes
    );
    let include_paradigms = matches!(
        section,
        KnowledgeCatalogSection::All | KnowledgeCatalogSection::Paradigms
    );
    let include_rules = matches!(
        section,
        KnowledgeCatalogSection::All | KnowledgeCatalogSection::Rules
    );

    let summary = KnowledgeCatalogSummary {
        languages: knowledge.languages.len(),
        frameworks: knowledge.frameworks.len(),
        runtimes: knowledge.runtimes.len(),
        paradigms: knowledge.paradigms.len(),
        rules: knowledge.rule_applicability.len(),
    };

    let languages = if include_languages {
        knowledge
            .languages
            .iter()
            .map(|language| KnowledgeLanguageEntry {
                id: language.id.clone(),
                name: language.name.clone(),
                support: support_level_label(language.support).to_string(),
                extensions: language.extensions.clone(),
                filenames: language.filenames.clone(),
                aliases: language.aliases.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    let frameworks = if include_frameworks {
        knowledge
            .frameworks
            .iter()
            .map(|framework| KnowledgeNamedEntry {
                id: framework.id.clone(),
                name: framework.name.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    let runtimes = if include_runtimes {
        knowledge
            .runtimes
            .iter()
            .map(|runtime| KnowledgeNamedEntry {
                id: runtime.id.clone(),
                name: runtime.name.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    let paradigms = if include_paradigms {
        knowledge
            .paradigms
            .iter()
            .map(|paradigm| KnowledgeNamedEntry {
                id: paradigm.id.clone(),
                name: paradigm.name.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    let rules = if include_rules {
        knowledge
            .rule_applicability
            .iter()
            .map(|rule| KnowledgeRuleEntry {
                rule_id: rule.rule_id.clone(),
                minimum_support: rule
                    .minimum_support
                    .map(support_level_label)
                    .map(str::to_string),
                languages: sorted_vec(&rule.languages),
                frameworks: sorted_vec(&rule.frameworks),
                runtimes: sorted_vec(&rule.runtimes),
                paradigms: sorted_vec(&rule.paradigms),
                suppress_low_signal: rule.suppress_low_signal,
                suppress_generated: rule.suppress_generated,
                suppress_config: rule.suppress_config,
                overrides: rule.overrides.len(),
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(KnowledgeCatalogReport {
        summary,
        languages,
        frameworks,
        runtimes,
        paradigms,
        rules,
    })
}

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

fn support_level_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::DetectOnly => "detect-only",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::RuleAware => "rule-aware",
    }
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

fn sorted_vec(set: &HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort();
    v
}
