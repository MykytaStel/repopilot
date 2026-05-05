use crate::audits::pipeline::run_audits;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::language::detect_language;
use crate::scan::types::{LanguageSummary, ScanSummary};

use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

const EXCLUDED_DIRECTORY_NAMES: &[&str] =
    &[".git", ".github", "target", "node_modules", "dist", "build"];

pub fn scan_path(path: &Path) -> io::Result<ScanSummary> {
    scan_path_with_config(path, &ScanConfig::default())
}

pub fn scan_path_with_config(path: &Path, config: &ScanConfig) -> io::Result<ScanSummary> {
    let facts = collect_scan_facts(path)?;
    let findings = run_audits(&facts, config);

    Ok(ScanSummary {
        root_path: facts.root_path,
        files_count: facts.files_count,
        directories_count: facts.directories_count,
        lines_of_code: facts.lines_of_code,
        languages: facts.languages,
        findings,
    })
}

pub fn collect_scan_facts(path: &Path) -> io::Result<ScanFacts> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("path does not exist: {}", path.display()),
        ));
    }

    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        ..ScanFacts::default()
    };

    let mut languages: HashMap<String, usize> = HashMap::new();

    if path.is_file() {
        collect_file_facts(path, &mut facts, &mut languages)?;
    } else {
        collect_directory_facts(path, &mut facts, &mut languages)?;
    }

    facts.languages = build_language_summary(languages);

    Ok(facts)
}

fn collect_directory_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
) -> io::Result<()> {
    let walker = WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| !is_excluded_directory(entry.path()))
        .build();

    for result in walker {
        let entry = result.map_err(io::Error::other)?;
        let entry_path = entry.path();

        if entry_path == path {
            continue;
        }

        let Some(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            facts.directories_count += 1;
            continue;
        }

        if file_type.is_file() {
            collect_file_facts(entry_path, facts, languages)?;
        }
    }

    Ok(())
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| EXCLUDED_DIRECTORY_NAMES.contains(&name))
        .unwrap_or(false)
}

fn collect_file_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
) -> io::Result<()> {
    facts.files_count += 1;

    let language = detect_language(path).map(str::to_string);

    if let Some(language_name) = &language {
        *languages.entry(language_name.clone()).or_insert(0) += 1;
    }

    let Ok(content) = fs::read_to_string(path) else {
        return Ok(());
    };

    let lines_of_code = count_lines_of_code(&content);
    facts.lines_of_code += lines_of_code;

    facts.files.push(FileFacts {
        path: path.to_path_buf(),
        language,
        lines_of_code,
        content,
    });

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

    summary.sort_by(|left, right| {
        right
            .files_count
            .cmp(&left.files_count)
            .then_with(|| left.name.cmp(&right.name))
    });

    summary
}
