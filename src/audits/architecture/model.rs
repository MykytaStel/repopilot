use super::path_scope::{ArchitecturePathScope, classify_architecture_path};
use crate::scan::facts::{FileFacts, ScanFacts};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchitecturePathRole {
    Production,
    Test,
    Fixture,
    Documentation,
    Example,
    Generated,
    Vendor,
    BuildOutput,
    Tooling,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ArchitectureFile<'a> {
    pub(crate) facts: &'a FileFacts,
    pub(crate) role: ArchitecturePathRole,
}

impl<'a> ArchitectureFile<'a> {
    pub(crate) fn path(&self) -> &Path {
        &self.facts.path
    }

    fn is_production(&self) -> bool {
        self.role == ArchitecturePathRole::Production
    }
}

#[derive(Debug)]
pub(crate) struct ArchitectureAnalysis<'a> {
    pub(crate) files: Vec<ArchitectureFile<'a>>,
}

impl<'a> ArchitectureAnalysis<'a> {
    pub(crate) fn from_facts(facts: &'a ScanFacts) -> Self {
        let files = facts
            .files
            .iter()
            .map(|facts| ArchitectureFile {
                facts,
                role: role_for_path(&facts.path),
            })
            .collect();

        Self { files }
    }

    pub(crate) fn production_files(&self) -> impl Iterator<Item = &ArchitectureFile<'a>> {
        self.files.iter().filter(|file| file.is_production())
    }
}

fn role_for_path(path: &Path) -> ArchitecturePathRole {
    match classify_architecture_path(path) {
        ArchitecturePathScope::Production => ArchitecturePathRole::Production,
        ArchitecturePathScope::Test => ArchitecturePathRole::Test,
        ArchitecturePathScope::Fixture => ArchitecturePathRole::Fixture,
        ArchitecturePathScope::Documentation => ArchitecturePathRole::Documentation,
        ArchitecturePathScope::Example => ArchitecturePathRole::Example,
        ArchitecturePathScope::Generated => ArchitecturePathRole::Generated,
        ArchitecturePathScope::Vendor => ArchitecturePathRole::Vendor,
        ArchitecturePathScope::BuildOutput => ArchitecturePathRole::BuildOutput,
        ArchitecturePathScope::Tooling => ArchitecturePathRole::Tooling,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn given_production_source_path_when_analyzed_then_it_is_available_as_production_file() {
        let facts = scan_facts(&["src/features/payments/domain/service.ts"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);
        let production_paths = production_paths(&analysis);

        assert_eq!(
            production_paths,
            vec![PathBuf::from("src/features/payments/domain/service.ts")]
        );
    }

    #[test]
    fn given_tests_path_when_analyzed_then_it_is_classified_as_test_and_excluded() {
        let facts = scan_facts(&["tests/features/payments/domain/service_test.rs"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert_eq!(analysis.files[0].role, ArchitecturePathRole::Test);
        assert!(production_paths(&analysis).is_empty());
    }

    #[test]
    fn given_tests_fixtures_path_when_analyzed_then_it_is_classified_as_fixture_and_excluded() {
        let facts = scan_facts(&["tests/fixtures/rules/security/src/config.ts"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert_eq!(analysis.files[0].role, ArchitecturePathRole::Fixture);
        assert!(production_paths(&analysis).is_empty());
    }

    #[test]
    fn given_docs_path_when_analyzed_then_it_is_classified_as_documentation_and_excluded() {
        let facts = scan_facts(&["docs/reference/api/v1/client.ts"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert_eq!(analysis.files[0].role, ArchitecturePathRole::Documentation);
        assert!(production_paths(&analysis).is_empty());
    }

    #[test]
    fn given_examples_path_when_analyzed_then_it_is_classified_as_example_and_excluded() {
        let facts = scan_facts(&["examples/react-native/deep/sample/src/app.tsx"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert_eq!(analysis.files[0].role, ArchitecturePathRole::Example);
        assert!(production_paths(&analysis).is_empty());
    }

    #[test]
    fn given_generated_file_when_analyzed_then_it_is_classified_as_generated_and_excluded() {
        let facts = scan_facts(&["src/openapi/client/types.generated.ts"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert_eq!(analysis.files[0].role, ArchitecturePathRole::Generated);
        assert!(production_paths(&analysis).is_empty());
    }

    #[test]
    fn given_vendor_path_when_analyzed_then_it_is_classified_as_vendor_and_excluded() {
        let facts = scan_facts(&["vendor/company/package/deep/source/file.ts"]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert_eq!(analysis.files[0].role, ArchitecturePathRole::Vendor);
        assert!(production_paths(&analysis).is_empty());
    }

    #[test]
    fn given_target_dist_and_build_paths_when_analyzed_then_they_are_excluded_as_build_output() {
        let facts = scan_facts(&[
            "target/debug/build/package/out/generated.rs",
            "dist/assets/js/chunks/deep/file.js",
            "build/generated/client.ts",
        ]);

        let analysis = ArchitectureAnalysis::from_facts(&facts);

        assert!(
            analysis
                .files
                .iter()
                .all(|file| file.role == ArchitecturePathRole::BuildOutput)
        );
        assert!(production_paths(&analysis).is_empty());
    }

    fn production_paths(analysis: &ArchitectureAnalysis<'_>) -> Vec<PathBuf> {
        analysis
            .production_files()
            .map(|file| file.path().to_path_buf())
            .collect()
    }

    fn scan_facts(paths: &[&str]) -> ScanFacts {
        ScanFacts {
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
