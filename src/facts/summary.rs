use super::RepoFacts;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RepoFactsSummary {
    pub total_files: usize,
    pub files_with_language: usize,
    pub total_non_empty_lines: usize,
    pub languages: Vec<LanguageSummary>,
    pub diagnostics_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSummary {
    pub language: String,
    pub files: usize,
    pub non_empty_lines: usize,
}

pub fn summarize_repo_facts(facts: &RepoFacts) -> RepoFactsSummary {
    let mut language_totals = BTreeMap::<String, (usize, usize)>::new();

    for file in &facts.files {
        if let Some(language) = &file.language {
            let totals = language_totals.entry(language.clone()).or_default();
            totals.0 += 1;
            totals.1 += file.non_empty_lines;
        }
    }

    let mut languages = language_totals
        .into_iter()
        .map(|(language, (files, non_empty_lines))| LanguageSummary {
            language,
            files,
            non_empty_lines,
        })
        .collect::<Vec<_>>();

    languages.sort_by(|left, right| {
        right
            .files
            .cmp(&left.files)
            .then_with(|| right.non_empty_lines.cmp(&left.non_empty_lines))
            .then_with(|| left.language.cmp(&right.language))
    });

    RepoFactsSummary {
        total_files: facts.files.len(),
        files_with_language: facts
            .files
            .iter()
            .filter(|file| file.language.is_some())
            .count(),
        total_non_empty_lines: facts.files.iter().map(|file| file.non_empty_lines).sum(),
        languages,
        diagnostics_count: facts.diagnostics.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{FactConfidence, FactDiagnostic, FactSource, FileFact};
    use std::path::PathBuf;

    fn file(path: &str, language: Option<&str>, non_empty_lines: usize) -> FileFact {
        FileFact {
            path: PathBuf::from(path),
            language: language.map(str::to_string),
            non_empty_lines,
            source: FactSource::Mixed,
            confidence: FactConfidence::High,
        }
    }

    #[test]
    fn empty_repo_facts_have_an_empty_summary() {
        assert_eq!(
            summarize_repo_facts(&RepoFacts::default()),
            RepoFactsSummary::default()
        );
    }

    #[test]
    fn summary_counts_groups_and_sorts_facts_deterministically() {
        let facts = RepoFacts {
            root: PathBuf::from("/repo"),
            files: vec![
                file("src/lib.rs", Some("Rust"), 30),
                file("src/main.rs", Some("Rust"), 20),
                file("app.py", Some("Python"), 40),
                file("test.py", Some("Python"), 5),
                file("main.go", Some("Go"), 100),
                file("Main.java", Some("Java"), 100),
                file("LICENSE", None, 7),
            ],
            diagnostics: vec![
                FactDiagnostic {
                    code: "facts.first".to_string(),
                    message: "first diagnostic".to_string(),
                    path: None,
                },
                FactDiagnostic {
                    code: "facts.second".to_string(),
                    message: "second diagnostic".to_string(),
                    path: Some(PathBuf::from("src/lib.rs")),
                },
            ],
        };

        let summary = summarize_repo_facts(&facts);

        assert_eq!(summary.total_files, 7);
        assert_eq!(summary.files_with_language, 6);
        assert_eq!(summary.total_non_empty_lines, 302);
        assert_eq!(summary.diagnostics_count, 2);
        assert_eq!(
            summary.languages,
            vec![
                LanguageSummary {
                    language: "Rust".to_string(),
                    files: 2,
                    non_empty_lines: 50,
                },
                LanguageSummary {
                    language: "Python".to_string(),
                    files: 2,
                    non_empty_lines: 45,
                },
                LanguageSummary {
                    language: "Go".to_string(),
                    files: 1,
                    non_empty_lines: 100,
                },
                LanguageSummary {
                    language: "Java".to_string(),
                    files: 1,
                    non_empty_lines: 100,
                },
            ]
        );
    }
}
