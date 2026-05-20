use crate::cli::CompareOutputFormatArg;
use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::render;
use repopilot::report::schema::parse_scan_summary_json;
use repopilot::report::writer::write_report;
use repopilot::scan::types::ScanSummary;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn run(
    before: PathBuf,
    after: PathBuf,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before_summary = read_summary(&before)?;
    let after_summary = read_summary(&after)?;

    let diff = diff_summaries(&before_summary, &after_summary);
    let rendered = render(&diff, format.into())?;

    write_report(&rendered, output.as_deref())?;

    Ok(())
}

fn read_summary(path: &Path) -> Result<ScanSummary, CompareInputError> {
    let content = fs::read_to_string(path).map_err(|source| CompareInputError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    parse_scan_summary_json(&content).map_err(|source| CompareInputError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Debug)]
enum CompareInputError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl std::fmt::Display for CompareInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, .. } => {
                write!(f, "Failed to read scan summary {}", path.display())
            }
            Self::Parse { path, source } => {
                write!(
                    f,
                    "Failed to parse scan summary {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl Error for CompareInputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::read_summary;
    use repopilot::output::json;
    use repopilot::scan::types::ScanSummary;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn read_summary_accepts_versioned_json_report() {
        let summary = ScanSummary {
            hidden_suggestions: Vec::new(),
            root_path: PathBuf::from("."),
            files_analyzed: 7,
            non_empty_lines: 42,
            ..ScanSummary::default()
        };
        let rendered = json::render(&summary).expect("json render should succeed");
        let dir = tempdir().expect("tempdir should be created");
        let path = dir.path().join("report.json");
        fs::write(&path, rendered).expect("report should be written");

        let parsed = read_summary(&path).expect("versioned report should parse");

        assert_eq!(parsed.files_analyzed, 7);
        assert_eq!(parsed.non_empty_lines, 42);
    }

    #[test]
    fn read_summary_rejects_legacy_scan_summary_json() {
        let summary = ScanSummary {
            hidden_suggestions: Vec::new(),
            root_path: PathBuf::from("."),
            files_analyzed: 3,
            non_empty_lines: 21,
            ..ScanSummary::default()
        };
        let rendered =
            serde_json::to_string_pretty(&summary).expect("legacy json render should succeed");
        let dir = tempdir().expect("tempdir should be created");
        let path = dir.path().join("legacy-report.json");
        fs::write(&path, rendered).expect("report should be written");

        let error = read_summary(&path).expect_err("legacy report should be rejected");

        assert!(error.to_string().contains("Failed to parse scan summary"));
    }
}
