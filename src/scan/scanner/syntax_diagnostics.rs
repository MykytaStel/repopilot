use crate::scan::facts::ScanFacts;
use crate::scan::types::ScanDiagnostic;

pub(super) fn python_syntax_diagnostics(facts: &ScanFacts) -> Vec<ScanDiagnostic> {
    facts
        .artifacts()
        .filter_map(|artifact| {
            if artifact.language.as_deref() != Some("Python")
                || !artifact.syntax.has_errors
            {
                return None;
            }
            let line = artifact.syntax.first_error_line?;
            Some(
                ScanDiagnostic::warning(
                    "python.syntax-error",
                    format!(
                        "Python parser detected a syntax error at line {line}; verify with the project's compiler or test command."
                    ),
                )
                .with_path(artifact.path.clone())
                .with_line(line),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{FileContextFacts, ParsedArtifact, SyntaxSummary};
    use std::path::PathBuf;

    #[test]
    fn emits_a_stable_python_diagnostic_with_location() {
        let facts = ScanFacts {
            artifacts: std::collections::BTreeMap::from([(
                PathBuf::from("pkg/bad.py"),
                ParsedArtifact::from_source(
                    PathBuf::from("pkg/bad.py"),
                    Some("Python".to_string()),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    FileContextFacts::default(),
                    SyntaxSummary {
                        parsed: true,
                        root_kind: Some("module".to_string()),
                        has_errors: true,
                        first_error_line: Some(7),
                        named_child_count: 1,
                    },
                ),
            )]),
            ..ScanFacts::default()
        };

        let diagnostics = python_syntax_diagnostics(&facts);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "python.syntax-error");
        assert_eq!(diagnostics[0].path, Some(PathBuf::from("pkg/bad.py")));
        assert_eq!(diagnostics[0].line, Some(7));
    }
}
