use crate::analysis::parse::ParsedFile;
use crate::audits::code_quality::complexity::count_branches;
use crate::audits::context::classify_file;
use crate::audits::traits::FileAudit;
use crate::findings::types::Finding;
use crate::graph::imports::extract_imports_from;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::language::detect_language;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

pub(super) struct PerFileResult {
    pub(super) file_facts: FileFacts,
    pub(super) findings: Vec<Finding>,
    pub(super) language: Option<String>,
    pub(super) context: Option<PerFileContext>,
    pub(super) skip_reason: SkipReason,
    pub(super) skipped_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PerFileContext {
    pub(super) roles: Vec<String>,
    pub(super) frameworks: Vec<String>,
    pub(super) runtimes: Vec<String>,
    pub(super) paradigms: Vec<String>,
    pub(super) is_test: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SkipReason {
    None,
    LargeFile,
    Binary,
    LowSignal,
}

pub(super) enum LoadedFile {
    Analyzable {
        full_facts: FileFacts,
        language: Option<String>,
    },
    Skipped {
        language: Option<String>,
        reason: SkipReason,
        skipped_bytes: u64,
    },
}

pub(super) fn load_file(path: &Path, config: &ScanConfig) -> io::Result<LoadedFile> {
    let language = detect_language(path).map(str::to_string);

    if config.max_file_bytes > 0 {
        match fs::metadata(path) {
            Ok(metadata) => {
                let file_size = metadata.len();
                if file_size > config.max_file_bytes {
                    return Ok(LoadedFile::Skipped {
                        language,
                        reason: SkipReason::LargeFile,
                        skipped_bytes: file_size,
                    });
                }
            }
            Err(_) => {
                return Ok(LoadedFile::Skipped {
                    language,
                    reason: SkipReason::Binary,
                    skipped_bytes: 0,
                });
            }
        }
    }

    if !config.include_low_signal && is_low_signal_audit_path(path) {
        return Ok(LoadedFile::Skipped {
            language,
            reason: SkipReason::LowSignal,
            skipped_bytes: file_size(path),
        });
    }

    let Ok(content) = fs::read_to_string(path) else {
        return Ok(LoadedFile::Skipped {
            language,
            reason: SkipReason::Binary,
            skipped_bytes: file_size(path),
        });
    };

    let non_empty_lines = count_non_empty_lines(&content);
    let branch_count = count_branches(&content);
    let has_inline_tests = has_language_inline_tests(path, language.as_deref(), &content);

    // Imports are extracted later, from the shared `ParsedFile`, so a file's
    // syntax tree is produced once and reused by the per-file audits rather than
    // parsed separately here. See `process_file_inner` and `collect_file_facts`.
    Ok(LoadedFile::Analyzable {
        full_facts: FileFacts {
            path: path.to_path_buf(),
            language: language.clone(),
            non_empty_lines,
            branch_count,
            imports: Vec::new(),
            has_inline_tests,
            content: Some(content),
        },
        language,
    })
}

pub(super) fn without_content(file_facts: FileFacts) -> FileFacts {
    FileFacts {
        content: None,
        ..file_facts
    }
}

pub(super) fn empty_file_facts(path: &Path, language: Option<String>) -> FileFacts {
    FileFacts {
        path: path.to_path_buf(),
        language,
        non_empty_lines: 0,
        branch_count: 0,
        imports: Vec::new(),
        content: None,
        has_inline_tests: false,
    }
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

fn count_non_empty_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn has_language_inline_tests(path: &Path, language: Option<&str>, content: &str) -> bool {
    match language {
        Some("Rust") => content.contains("#[cfg(test)]") || content.contains("#[test]"),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => {
            contains_call(content, "describe")
                || contains_call(content, "it")
                || contains_call(content, "test")
        }
        Some("Python") => content.contains("def test_") || content.contains("unittest."),
        Some("Go") => content.contains("func Test") || content.contains("func Benchmark"),
        Some("Java") | Some("Kotlin") => content.contains("@Test"),
        _ => path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("_test.") || name.contains(".test.")),
    }
}

fn contains_call(content: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    content.match_indices(&needle).any(|(index, _)| {
        content[..index]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
    })
}

pub(super) fn process_file(
    path: &Path,
    file_audits: &[Box<dyn FileAudit>],
    config: &ScanConfig,
) -> io::Result<PerFileResult> {
    process_file_inner(path, file_audits, config, false)
}

pub(super) fn process_file_with_content(
    path: &Path,
    file_audits: &[Box<dyn FileAudit>],
    config: &ScanConfig,
) -> io::Result<PerFileResult> {
    process_file_inner(path, file_audits, config, true)
}

fn process_file_inner(
    path: &Path,
    file_audits: &[Box<dyn FileAudit>],
    config: &ScanConfig,
    retain_content: bool,
) -> io::Result<PerFileResult> {
    let loaded = load_file(path, config)?;
    let LoadedFile::Analyzable {
        mut full_facts,
        language,
    } = loaded
    else {
        return Ok(skipped_result_from_loaded(path, loaded));
    };

    let context = PerFileContext::from_file(&full_facts);
    let mut findings = Vec::new();
    let imports = {
        // Parse the file at most once and share the syntax tree across both
        // import extraction and every audit. Scoped so the borrow of
        // `full_facts` ends before `imports` is written back and it is moved.
        let parsed = ParsedFile::for_facts(&full_facts);
        let imports = extract_imports_from(&parsed, full_facts.language.as_deref());
        for audit in file_audits {
            findings.extend(audit.audit_parsed(&full_facts, &parsed, config));
        }
        imports
    };
    full_facts.imports = imports;

    Ok(PerFileResult {
        file_facts: if retain_content {
            full_facts
        } else {
            without_content(full_facts)
        },
        findings,
        language,
        context: Some(context),
        skip_reason: SkipReason::None,
        skipped_bytes: 0,
    })
}

pub(super) fn collect_file_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    config: &ScanConfig,
) -> io::Result<()> {
    let LoadedFile::Analyzable {
        mut full_facts, ..
    } = load_file_or_record_skip(path, facts, config)?
    else {
        return Ok(());
    };

    // No audits run on this path, so the parse view is built solely to extract
    // imports; it is still parsed at most once via the shared parsers. Bind the
    // result first so the view's borrow of `full_facts` ends before the write.
    let imports =
        extract_imports_from(&ParsedFile::for_facts(&full_facts), full_facts.language.as_deref());
    full_facts.imports = imports;

    record_analyzed_file(facts, languages, &full_facts);
    facts.files.push(full_facts);

    Ok(())
}

fn load_file_or_record_skip(
    path: &Path,
    facts: &mut ScanFacts,
    config: &ScanConfig,
) -> io::Result<LoadedFile> {
    let loaded = load_file(path, config)?;
    if let LoadedFile::Skipped {
        language,
        reason,
        skipped_bytes,
    } = &loaded
    {
        record_skip(path, facts, language.clone(), *reason, *skipped_bytes);
    }
    Ok(loaded)
}

fn skipped_result_from_loaded(path: &Path, loaded: LoadedFile) -> PerFileResult {
    match loaded {
        LoadedFile::Skipped {
            language,
            reason,
            skipped_bytes,
        } => skipped_result(path, language, reason, skipped_bytes),
        LoadedFile::Analyzable { .. } => unreachable!("analyzable files are handled by caller"),
    }
}

fn record_analyzed_file(
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    file_facts: &FileFacts,
) {
    facts.files_analyzed += 1;
    facts.non_empty_lines += file_facts.non_empty_lines;
    if let Some(language_name) = &file_facts.language {
        *languages.entry(language_name.clone()).or_insert(0) += 1;
    }
}

fn record_skip(
    path: &Path,
    facts: &mut ScanFacts,
    language: Option<String>,
    reason: SkipReason,
    skipped_bytes: u64,
) {
    match reason {
        SkipReason::None => {}
        SkipReason::LargeFile => track_skipped_file(facts, skipped_bytes),
        SkipReason::Binary => track_binary_file(facts, skipped_bytes),
        SkipReason::LowSignal => facts.files_skipped_low_signal += 1,
    }
    facts.files.push(empty_file_facts(path, language));
}

fn skipped_result(
    path: &Path,
    language: Option<String>,
    skip_reason: SkipReason,
    skipped_bytes: u64,
) -> PerFileResult {
    PerFileResult {
        file_facts: empty_file_facts(path, language.clone()),
        findings: Vec::new(),
        language,
        context: None,
        skip_reason,
        skipped_bytes,
    }
}

impl PerFileContext {
    fn from_file(file: &FileFacts) -> Self {
        let context = classify_file(file);
        Self {
            roles: context.role_ids().into_iter().map(str::to_string).collect(),
            frameworks: context
                .framework_ids()
                .into_iter()
                .map(str::to_string)
                .collect(),
            runtimes: context
                .runtime_ids()
                .into_iter()
                .map(str::to_string)
                .collect(),
            paradigms: context
                .paradigm_ids()
                .into_iter()
                .map(str::to_string)
                .collect(),
            is_test: context.is_test,
        }
    }
}

fn track_skipped_file(facts: &mut ScanFacts, skipped_bytes: u64) {
    facts.large_files_skipped += 1;
    facts.skipped_bytes = facts.skipped_bytes.saturating_add(skipped_bytes);
}

fn track_binary_file(facts: &mut ScanFacts, skipped_bytes: u64) {
    facts.binary_files_skipped += 1;
    facts.skipped_bytes = facts.skipped_bytes.saturating_add(skipped_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_skipped_instead_of_aborting_scan() {
        let loaded = load_file(Path::new("missing-after-walk.rs"), &ScanConfig::default())
            .expect("missing file should be classified as skipped");

        let LoadedFile::Skipped {
            language,
            reason,
            skipped_bytes,
        } = loaded
        else {
            panic!("missing file should not be analyzable");
        };

        assert_eq!(language.as_deref(), Some("Rust"));
        assert_eq!(reason, SkipReason::Binary);
        assert_eq!(skipped_bytes, 0);
    }
}
