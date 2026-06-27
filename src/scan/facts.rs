use crate::frameworks::DetectedFramework;
use crate::frameworks::FrameworkProject;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::scan::types::LanguageSummary;
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ScanFacts {
    pub root_path: PathBuf,
    pub files_discovered: usize,
    pub files_analyzed: usize,
    pub directories_count: usize,
    pub non_empty_lines: usize,
    pub large_files_skipped: usize,
    pub files_skipped_low_signal: usize,
    pub binary_files_skipped: usize,
    pub skipped_bytes: u64,
    pub languages: Vec<LanguageSummary>,
    pub files: Vec<FileFacts>,
    pub detected_frameworks: Vec<DetectedFramework>,
    pub framework_projects: Vec<FrameworkProject>,
    pub react_native: Option<ReactNativeArchitectureProfile>,
    pub files_skipped_by_limit: usize,
    pub files_skipped_repopilotignore: usize,
    pub repopilotignore_path: Option<PathBuf>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub path: PathBuf,
    pub language: Option<String>,
    pub non_empty_lines: usize,
    pub branch_count: usize,
    pub imports: Vec<String>,
    /// Imports that are full coupling edges but not module-load/runtime edges for
    /// cycle detection: Python imports present only inside a function body, plus
    /// TypeScript/JavaScript type-only imports/exports erased by the compiler. A
    /// subset of `imports`; the coupling graph keeps them as edges but cycle
    /// detection subtracts them. Empty for languages without such non-runtime edges.
    pub deferred_imports: Vec<String>,
    /// Source text while file audits are running. `None` means the file was skipped,
    /// binary/unreadable, or retained only as summary metadata after auditing.
    pub content: Option<String>,
    /// True if the file contains a `#[cfg(test)]` block (Rust inline unit tests).
    /// Computed while content is available; preserved after content is dropped.
    pub has_inline_tests: bool,
    /// True if the file lives inside a package that declares an executable
    /// entrypoint (npm `package.json#bin`, Cargo `[[bin]]`/`src/bin`). Such a
    /// package is a CLI tool, so the knowledge engine treats `process.exit` and
    /// similar host-termination calls as an intended boundary (downgraded, not a
    /// hazard). Set by a post-collection pass once workspace layout is known.
    pub in_executable_package: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileContentProvider;

impl FileContentProvider {
    pub fn content<'a>(&self, file: &'a FileFacts) -> Option<Cow<'a, str>> {
        if let Some(content) = file.content.as_deref() {
            return Some(Cow::Borrowed(content));
        }

        fs::read_to_string(&file.path).ok().map(Cow::Owned)
    }
}
