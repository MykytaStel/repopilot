use super::SkipReason;
use crate::audits::code_quality::complexity::count_branches;
use crate::graph::imports::extract_imports;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;
use crate::scan::language::detect_language;
use crate::scan::path_classification::is_low_signal_audit_path;
use std::fs;
use std::io;
use std::path::Path;

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
    let imports = extract_imports(&content, language.as_deref());

    Ok(LoadedFile::Analyzable {
        full_facts: FileFacts {
            path: path.to_path_buf(),
            language: language.clone(),
            non_empty_lines,
            branch_count,
            imports,
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
