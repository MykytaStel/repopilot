mod confidence;
mod diagnostic;
mod file;
mod repo;
mod source;

pub use confidence::FactConfidence;
pub use diagnostic::FactDiagnostic;
pub use file::FileFact;
pub use repo::RepoFacts;
pub use source::FactSource;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_repo_facts_are_empty() {
        let facts = RepoFacts::default();

        assert_eq!(facts.root, PathBuf::new());
        assert!(facts.files.is_empty());
        assert!(facts.diagnostics.is_empty());
    }

    #[test]
    fn file_fact_stores_core_metadata() {
        let fact = FileFact {
            path: PathBuf::from("src/lib.rs"),
            language: Some("Rust".to_string()),
            non_empty_lines: 42,
            source: FactSource::Ast,
            confidence: FactConfidence::High,
        };

        assert_eq!(fact.path, PathBuf::from("src/lib.rs"));
        assert_eq!(fact.language.as_deref(), Some("Rust"));
        assert_eq!(fact.non_empty_lines, 42);
        assert_eq!(fact.source, FactSource::Ast);
        assert_eq!(fact.confidence, FactConfidence::High);
    }

    #[test]
    fn fact_diagnostic_stores_code_message_and_path() {
        let diagnostic = FactDiagnostic {
            code: "facts.read_failed".to_string(),
            message: "could not read source file".to_string(),
            path: Some(PathBuf::from("src/missing.rs")),
        };

        assert_eq!(diagnostic.code, "facts.read_failed");
        assert_eq!(diagnostic.message, "could not read source file");
        assert_eq!(diagnostic.path, Some(PathBuf::from("src/missing.rs")));
    }
}
