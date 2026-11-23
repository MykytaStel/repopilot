use super::{SinkKind, SourceKind, detect_taint};
use crate::review::diff::{ChangeStatus, ChangedFile, ChangedRange};
use crate::review::signals::content::ReviewSource;
use std::path::PathBuf;

/// Run taint detection over `code`, treating every line as changed so the sink is
/// always in-diff. `path` drives language detection; `label` drives parsing.
pub(super) fn run(path: &str, label: &str, code: &str) -> Vec<super::TaintSignal> {
    run_with_changed_lines(path, label, code, 1, 100_000)
}

pub(super) fn run_with_changed_lines(
    path: &str,
    label: &str,
    code: &str,
    start: usize,
    end: usize,
) -> Vec<super::TaintSignal> {
    let file = ChangedFile {
        path: PathBuf::from(path),
        status: ChangeStatus::Modified,
        ranges: vec![ChangedRange { start, end }],
        hunks: Vec::new(),
    };
    let source = ReviewSource::new(code.to_string(), Some(label.to_string()));
    detect_taint(&file, Some(&source))
}

mod coercions;
mod csharp;
mod go;
mod helper_regressions;
mod helper_shell;
mod java;
mod javascript;
mod javascript_helpers;
mod python;
mod scope_guards;
