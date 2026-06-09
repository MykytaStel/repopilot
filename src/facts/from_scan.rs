use super::{FactConfidence, FactSource, FileFact, RepoFacts};
use crate::scan::facts::ScanFacts;

pub fn repo_facts_from_scan(scan: &ScanFacts) -> RepoFacts {
    let files = scan
        .files
        .iter()
        .map(|file| FileFact {
            path: file.path.clone(),
            language: file.language.clone(),
            non_empty_lines: file.non_empty_lines,
            // Scan facts combine filesystem metadata with detected and analyzed facts.
            source: FactSource::Mixed,
            confidence: FactConfidence::High,
        })
        .collect();

    RepoFacts {
        root: scan.root_path.clone(),
        files,
        diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::facts::FileFacts;
    use std::path::PathBuf;

    #[test]
    fn empty_scan_facts_convert_to_empty_repo_facts() {
        let scan = ScanFacts::default();

        let facts = repo_facts_from_scan(&scan);

        assert_eq!(facts.root, PathBuf::new());
        assert!(facts.files.is_empty());
        assert!(facts.diagnostics.is_empty());
    }

    #[test]
    fn bridge_preserves_supported_file_facts_without_content() {
        let scan = ScanFacts {
            root_path: PathBuf::from("/repo"),
            files: vec![FileFacts {
                path: PathBuf::from("/repo/src/lib.rs"),
                language: Some("Rust".to_string()),
                non_empty_lines: 42,
                branch_count: 3,
                imports: vec!["crate::facts".to_string()],
                content: None,
                has_inline_tests: true,
            }],
            ..ScanFacts::default()
        };

        let facts = repo_facts_from_scan(&scan);

        assert_eq!(facts.root, PathBuf::from("/repo"));
        assert_eq!(facts.files.len(), 1);
        assert_eq!(facts.files[0].path, PathBuf::from("/repo/src/lib.rs"));
        assert_eq!(facts.files[0].language.as_deref(), Some("Rust"));
        assert_eq!(facts.files[0].non_empty_lines, 42);
        assert_eq!(facts.files[0].source, FactSource::Mixed);
        assert_eq!(facts.files[0].confidence, FactConfidence::High);
        assert!(facts.diagnostics.is_empty());
    }
}
