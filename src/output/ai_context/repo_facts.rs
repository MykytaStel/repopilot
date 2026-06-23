use crate::facts::RepoFactsSummary;
use std::fmt::Write as FmtWrite;

const MAX_LANGUAGES: usize = 5;

pub(super) fn render_summary(out: &mut String, summary: &RepoFactsSummary) {
    let _ = writeln!(out, "## Repository Facts Summary\n");

    if summary.total_files == 0 {
        let _ = writeln!(out, "- No repository file facts were collected.");
        let _ = writeln!(out, "- Fact diagnostics: {}\n", summary.diagnostics_count);
        return;
    }

    let _ = writeln!(out, "- Files: {}", summary.total_files);
    let _ = writeln!(
        out,
        "- Files with detected language: {}",
        summary.files_with_language
    );
    let _ = writeln!(out, "- Non-empty lines: {}", summary.total_non_empty_lines);
    let _ = writeln!(out, "- Fact diagnostics: {}", summary.diagnostics_count);

    if !summary.languages.is_empty() {
        let _ = writeln!(out, "\nTop languages:");
        for language in summary.languages.iter().take(MAX_LANGUAGES) {
            let file_label = if language.files == 1 { "file" } else { "files" };
            let _ = writeln!(
                out,
                "- {}: {} {}, {} non-empty lines",
                language.language, language.files, file_label, language.non_empty_lines
            );
        }
    }

    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{
        FactConfidence, FactDiagnostic, FactSource, FileFact, RepoFacts, repo_facts_from_scan,
        summarize_repo_facts,
    };
    use crate::scan::facts::{FileFacts, ScanFacts};
    use std::path::PathBuf;

    #[test]
    fn renders_empty_state_with_diagnostic_count() {
        let mut output = String::new();
        render_summary(
            &mut output,
            &RepoFactsSummary {
                diagnostics_count: 2,
                ..RepoFactsSummary::default()
            },
        );

        assert!(output.contains("## Repository Facts Summary"));
        assert!(output.contains("- No repository file facts were collected."));
        assert!(output.contains("- Fact diagnostics: 2"));
    }

    #[test]
    fn renders_aggregate_facts_without_raw_paths() {
        let raw_path = PathBuf::from("/private/repo/src/lib.rs");
        let scan = ScanFacts {
            root_path: PathBuf::from("/private/repo"),
            files: vec![
                FileFacts {
                    path: raw_path.clone(),
                    language: Some("Rust".to_string()),
                    non_empty_lines: 42,
                    branch_count: 0,
                    imports: Vec::new(),
                    content: None,
                    has_inline_tests: false,
                    in_executable_package: false,
                },
                FileFacts {
                    path: PathBuf::from("/private/repo/README"),
                    language: None,
                    non_empty_lines: 8,
                    branch_count: 0,
                    imports: Vec::new(),
                    content: None,
                    has_inline_tests: false,
                    in_executable_package: false,
                },
            ],
            ..ScanFacts::default()
        };
        let facts = repo_facts_from_scan(&scan);
        let mut summary = summarize_repo_facts(&facts);
        summary.diagnostics_count = 1;

        let mut output = String::new();
        render_summary(&mut output, &summary);

        assert!(output.contains("## Repository Facts Summary"));
        assert!(output.contains("- Files: 2"));
        assert!(output.contains("- Files with detected language: 1"));
        assert!(output.contains("- Non-empty lines: 50"));
        assert!(output.contains("- Fact diagnostics: 1"));
        assert!(output.contains("- Rust: 1 file, 42 non-empty lines"));
        assert!(!output.contains(&raw_path.display().to_string()));
    }

    #[test]
    fn limits_language_output_to_five_entries() {
        let facts = RepoFacts {
            files: (0..6)
                .map(|index| FileFact {
                    path: PathBuf::from(format!("file-{index}")),
                    language: Some(format!("Language-{index}")),
                    non_empty_lines: 10 - index,
                    source: FactSource::Mixed,
                    confidence: FactConfidence::High,
                })
                .collect(),
            diagnostics: vec![FactDiagnostic {
                code: "facts.example".to_string(),
                message: "example".to_string(),
                path: None,
            }],
            ..RepoFacts::default()
        };
        let summary = summarize_repo_facts(&facts);

        let mut output = String::new();
        render_summary(&mut output, &summary);

        assert_eq!(output.matches("- Language-").count(), MAX_LANGUAGES);
        assert!(!output.contains("- Language-5:"));
    }
}
