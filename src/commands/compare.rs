use crate::cli::CompareOutputFormatArg;
use repopilot::compare::diff::diff_summaries;
use repopilot::compare::render::render;
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

    serde_json::from_str(&content).map_err(|source| CompareInputError::Parse {
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
            Self::Parse { path, .. } => {
                write!(f, "Failed to parse scan summary {}", path.display())
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
