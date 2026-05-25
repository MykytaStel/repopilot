use super::path_scope::is_production_architecture_candidate;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::{Path, PathBuf};

pub struct DeepNestingAudit;

impl ProjectAudit for DeepNestingAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        let root_depth = facts.root_path.components().count();

        facts
            .files
            .iter()
            .filter(|file| is_production_architecture_candidate(&file.path))
            .filter_map(|file| production_depth_over_threshold(&file.path, root_depth, config))
            .max_by_key(|(_, depth)| *depth)
            .map(|(path, depth)| build_finding(path, depth, config.max_directory_depth))
            .into_iter()
            .collect()
    }
}

fn production_depth_over_threshold(
    path: &Path,
    root_depth: usize,
    config: &ScanConfig,
) -> Option<(PathBuf, usize)> {
    let depth = path.components().count().saturating_sub(root_depth);

    (depth > config.max_directory_depth).then(|| (path.to_path_buf(), depth))
}

fn build_finding(deepest_path: PathBuf, depth: usize, threshold: usize) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "architecture.deep-nesting".to_string(),
        recommendation: Finding::recommendation_for_rule_id("architecture.deep-nesting"),
        title: "Deeply nested production directory structure detected".to_string(),
        description: format!(
            "Production source contains files nested {depth} levels deep, exceeding the threshold \
             of {threshold}. Deep nesting often indicates over-engineered module hierarchies."
        ),
        category: FindingCategory::Architecture,
        severity: Severity::Low,
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: deepest_path,
            line_start: 1,
            line_end: None,
            snippet: format!("Production nesting depth: {depth}; threshold is {threshold}."),
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
        let facts = scan_facts(&[
            "./src/domain/user.ts",
            "./tests/fixtures/rules/security.secret-candidate/true_positive_env_value/src/config.ts",
        ]);

        let findings = audit_with_depth(&facts, 5);

        assert!(
            findings.is_empty(),
            "rule fixture paths should not become production architecture findings"
        );
    }

    #[test]
    fn ignores_test_paths_even_when_they_are_deeper_than_source_paths() {
        let facts = scan_facts(&[
            "./src/features/payments/service.ts",
            "./tests/unit/features/payments/checkout/mobile/session/service.test.ts",
        ]);

        let findings = audit_with_depth(&facts, 5);

        assert!(
            findings.is_empty(),
            "test paths are allowed to be deeper than production source paths"
        );
    }

    #[test]
    fn ignores_docs_examples_generated_vendor_and_build_paths() {
        let facts = scan_facts(&[
            "./docs/reference/api/v1/generated/client/config.ts",
            "./examples/react-native/deep/sample/src/App.tsx",
            "./src/generated/openapi/client/v1/types.generated.ts",
            "./vendor/company/package/deep/source/file.ts",
            "./target/debug/build/package/out/generated.rs",
            "./dist/assets/js/chunks/deep/file.js",
        ]);

        let findings = audit_with_depth(&facts, 5);

        assert!(findings.is_empty());
    }

    #[test]
    fn reports_deep_production_path() {
        let production_path = "./src/features/payments/checkout/mobile/session/domain/handler.ts";
        let facts = scan_facts(&[production_path]);

        let findings = audit_with_depth(&facts, 5);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "architecture.deep-nesting");
        assert_eq!(findings[0].evidence[0].path, PathBuf::from(production_path));
        assert!(
            findings[0].title.contains("production"),
            "finding copy should make the production scope explicit"
        );
    }

    fn audit_with_depth(facts: &ScanFacts, max_directory_depth: usize) -> Vec<Finding> {
        let audit = DeepNestingAudit;
        let config = ScanConfig {
            max_directory_depth,
            ..ScanConfig::default()
        };

        audit.audit(facts, &config)
    }

    fn scan_facts(paths: &[&str]) -> ScanFacts {
        ScanFacts {
            root_path: PathBuf::from("."),
            files: paths.iter().map(|path| file_facts(path)).collect(),
            ..ScanFacts::default()
        }
    }

    fn file_facts(path: &str) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: Some("TypeScript".to_string()),
            non_empty_lines: 1,
            branch_count: 0,
            imports: Vec::new(),
            content: Some("export const value = true;\n".to_string()),
            has_inline_tests: false,
        }
    }
}
