use crate::scan::language::detect_language;
use crate::scan::markers::detect_markers;
use crate::scan::types::{LanguageSummary, ScanSummary};

use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

pub fn scan_path(path: &Path) -> io::Result<ScanSummary> {
    let mut summary = ScanSummary {
        root_path: path.to_path_buf(),
        ..ScanSummary::default()
    };

    let mut languages: HashMap<String, usize> = HashMap::new();

    if path.is_file() {
        scan_file(path, &mut summary, &mut languages)?;
    } else {
        scan_directory(path, &mut summary, &mut languages)?;
    }

    summary.languages = build_language_summary(languages);

    Ok(summary)
}

fn scan_directory(
    path: &Path,
    summary: &mut ScanSummary,
    languages: &mut HashMap<String, usize>,
) -> io::Result<()> {
    let walker = WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for result in walker {
        let entry = result.map_err(io::Error::other)?;
        let entry_path = entry.path();

        if entry_path == path {
            continue;
        }

        if entry_path.is_dir() {
            summary.directories_count += 1;
            continue;
        }

        if entry_path.is_file() {
            scan_file(entry_path, summary, languages)?;
        }
    }

    Ok(())
}

fn scan_file(
    path: &Path,
    summary: &mut ScanSummary,
    languages: &mut HashMap<String, usize>,
) -> io::Result<()> {
    summary.files_count += 1;

    if let Some(language) = detect_language(path) {
        *languages.entry(language.to_string()).or_insert(0) += 1;
    }

    let Ok(content) = fs::read_to_string(path) else {
        return Ok(());
    };

    summary.lines_of_code += count_lines_of_code(&content);
    summary.markers.extend(detect_markers(path, &content));

    Ok(())
}

fn count_lines_of_code(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn build_language_summary(languages: HashMap<String, usize>) -> Vec<LanguageSummary> {
    let mut summary: Vec<LanguageSummary> = languages
        .into_iter()
        .map(|(name, files_count)| LanguageSummary { name, files_count })
        .collect();

    summary.sort_by(|left, right| right.files_count.cmp(&left.files_count));

    summary
}
