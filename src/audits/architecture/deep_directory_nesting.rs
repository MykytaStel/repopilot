use super::model::ArchitectureAnalysis;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::{Component, Path, PathBuf};

pub struct DeepDirectoryNestingAudit;

impl ProjectAudit for DeepDirectoryNestingAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        ArchitectureAnalysis::from_facts(facts)
            .production_files()
            .filter_map(|file| {
                let depth = compute_directory_nesting(file.path());
                if depth > config.max_directory_depth {
                    Some(build_finding(
                        file.path().to_path_buf(),
                        depth,
                        config.max_directory_depth,
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

fn compute_directory_nesting(path: &Path) -> usize {
    path.components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count()
        .saturating_sub(1)
}

fn build_finding(path: PathBuf, depth: usize, threshold: usize) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "architecture.deep-directory-nesting".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.deep-directory-nesting"),
        title: "Deep directory nesting detected".to_string(),
        description: format!(
            "This file is nested {depth} directories deep, exceeding the threshold \
             of {threshold}. Deep directory nesting makes the codebase harder to navigate."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path,
            line_start: 1,
            line_end: None,
            snippet: format!("Directory nesting depth: {depth}; threshold is {threshold}."),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::facts::FileFacts;

    #[test]
    fn ignores_rule_fixtures_when_calculating_deepest_path() {
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![
                file_facts_with_path("./src/domain/user.ts"),
                file_facts_with_path(
                    "./tests/fixtures/rules/security.secret-candidate/true_positive_env_value/src/config.ts",
                ),
            ],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert!(
            findings.is_empty(),
            "rule fixture paths should not become production architecture findings"
        );
    }

    #[test]
    fn ignores_test_paths_even_when_they_are_deeper_than_source_paths() {
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![
                file_facts_with_path("./src/service.ts"),
                file_facts_with_path("./tests/unit/a/b/c/d/e/service.test.ts"),
            ],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert!(
            findings.is_empty(),
            "test paths are allowed to be deeper than production source paths"
        );
    }

    #[test]
    fn ignores_docs_examples_generated_vendor_and_build_paths() {
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![
                file_facts_with_path("./docs/reference/api/v1/generated/client/config.ts"),
                file_facts_with_path("./examples/react-native/deep/sample/src/App.tsx"),
                file_facts_with_path("./src/generated/openapi/client/v1/types.generated.ts"),
                file_facts_with_path("./vendor/company/package/deep/source/file.ts"),
                file_facts_with_path("./target/debug/build/package/out/generated.rs"),
                file_facts_with_path("./dist/assets/js/chunks/deep/file.js"),
            ],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert!(findings.is_empty());
    }

    #[test]
    fn reports_deep_production_path() {
        let production_path = "./src/a/b/c/d/e/f/handler.ts";
        let facts = ScanFacts {
            root_path: PathBuf::from("."),
            files: vec![file_facts_with_path(production_path)],
            ..ScanFacts::default()
        };

        let findings = audit_with_depth(&facts, 5);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "architecture.deep-directory-nesting");
        assert_eq!(findings[0].evidence[0].path, PathBuf::from(production_path));
    }

    fn audit_with_depth(facts: &ScanFacts, max_directory_depth: usize) -> Vec<Finding> {
        let audit = DeepDirectoryNestingAudit;
        let config = ScanConfig {
            max_directory_depth,
            ..ScanConfig::default()
        };

        audit.audit(facts, &config)
    }

    fn file_facts_with_path(path: &str) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: Some("TypeScript".to_string()),
            non_empty_lines: 1,
            branch_count: 0,
            imports: Vec::new(),
            content: Some("".to_string()),
            has_inline_tests: false,
        }
    }
}
