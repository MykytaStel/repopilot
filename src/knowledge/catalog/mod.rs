mod model;
mod render;

pub use model::{
    KnowledgeCatalogReport, KnowledgeCatalogSection, KnowledgeCatalogSummary,
    KnowledgeLanguageEntry, KnowledgeNamedEntry, KnowledgeRuleEntry,
};
pub use render::render_knowledge_catalog_report;

use crate::knowledge::active_knowledge;
use std::collections::HashSet;

pub fn build_knowledge_catalog_report(
    section: KnowledgeCatalogSection,
) -> Result<KnowledgeCatalogReport, Box<dyn std::error::Error>> {
    let knowledge = active_knowledge();

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
            .map(KnowledgeLanguageEntry::from_profile)
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
                    .map(model::support_level_label)
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

fn sorted_vec(set: &HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    #[test]
    fn rules_section_keeps_only_summary_and_rules() {
        let report = build_knowledge_catalog_report(KnowledgeCatalogSection::Rules)
            .expect("rules catalog should build");

        assert_eq!(report.summary.languages, active_knowledge().languages.len());
        assert!(report.languages.is_empty());
        assert!(report.frameworks.is_empty());
        assert!(report.runtimes.is_empty());
        assert!(report.paradigms.is_empty());
        assert_eq!(
            report.rules.len(),
            active_knowledge().rule_applicability.len()
        );
    }

    #[test]
    fn json_render_keeps_existing_shape() {
        let report = build_knowledge_catalog_report(KnowledgeCatalogSection::Rules)
            .expect("rules catalog should build");
        let rendered = render_knowledge_catalog_report(&report, OutputFormat::Json)
            .expect("catalog should render");
        let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid json");

        assert!(value.get("summary").is_some());
        assert!(value.get("rules").is_some());
        assert!(value.get("languages").is_none());
    }

    #[test]
    fn markdown_rules_render_is_stable() {
        let report = build_knowledge_catalog_report(KnowledgeCatalogSection::Rules)
            .expect("rules catalog should build");
        let rendered = render_knowledge_catalog_report(&report, OutputFormat::Markdown)
            .expect("catalog should render");

        assert!(rendered.contains("# RepoPilot Knowledge Catalog"));
        assert!(rendered.contains("## Summary"));
        assert!(rendered.contains("## Rules"));
        assert!(rendered.contains("`language.rust.panic-risk`"));
    }
}
