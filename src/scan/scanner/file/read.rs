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
        let file_size = fs::metadata(path)?.len();
        if file_size > config.max_file_bytes {
            return Ok(LoadedFile::Skipped {
                language,
                reason: SkipReason::LargeFile,
                skipped_bytes: file_size,
            });
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

    let lines_of_code = count_lines_of_code(&content);
    let branch_count = count_branches(&content);
    let has_inline_tests = content.contains("#[cfg(test)]");
    let imports = extract_imports(&content, language.as_deref());

    Ok(LoadedFile::Analyzable {
        full_facts: FileFacts {
            path: path.to_path_buf(),
            language: language.clone(),
            lines_of_code,
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
        lines_of_code: 0,
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

fn count_lines_of_code(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}
