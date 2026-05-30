use crate::audits::context::LanguageKind;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::knowledge::language::language_kind_for_file;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct TooManyModulesAudit;

impl ProjectAudit for TooManyModulesAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        let mut dir_file_counts: HashMap<PathBuf, usize> = HashMap::new();
        let classifier = crate::analysis::ArchitectureClassifier::new(&config.module_mappings);

        for file in &facts.files {
            let context = classifier.classify(file);
            if context.file_role != crate::analysis::FileRole::Production {
                continue;
            }
            if is_low_signal_audit_path(&file.path) {
                continue;
            }
            if !is_source_module_language(language_kind_for_file(file)) {
                continue;
            }
            if let Some(parent) = file.path.parent() {
                *dir_file_counts.entry(parent.to_path_buf()).or_insert(0) += 1;
            }
        }

        dir_file_counts
            .into_iter()
            .filter(|(_, count)| *count > config.max_directory_modules)
            .map(|(dir, count)| build_finding(dir, count, config.max_directory_modules))
            .collect()
    }
}

fn is_source_module_language(language: LanguageKind) -> bool {
    !matches!(
        language,
        LanguageKind::Dockerfile
            | LanguageKind::Json
            | LanguageKind::Toml
            | LanguageKind::Yaml
            | LanguageKind::Markdown
            | LanguageKind::Html
            | LanguageKind::Css
            | LanguageKind::Scss
            | LanguageKind::Sql
            | LanguageKind::Unknown
    )
}

fn build_finding(dir: PathBuf, file_count: usize, threshold: usize) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "architecture.too-many-modules".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.too-many-modules"),
        title: "Directory contains too many modules".to_string(),
        description: format!(
            "This directory has {file_count} files, exceeding the threshold of {threshold}. Consider splitting into sub-modules to reduce coupling."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Medium,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: dir,
            line_start: 1,
            line_end: None,
            snippet: format!("{file_count} source modules in directory; threshold is {threshold}."),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}
