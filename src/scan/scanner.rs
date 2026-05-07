use crate::audits::architecture::import_coupling::ImportCouplingAudit;
use crate::audits::code_quality::complexity::count_branches;
use crate::audits::pipeline::{build_file_audits, run_project_audits};
use crate::audits::traits::FileAudit;
use crate::baseline::key::stable_finding_key;
use crate::findings::types::Finding;
use crate::graph::imports::extract_imports;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::language::detect_language;
use crate::scan::types::{LanguageSummary, ScanSummary};

use ignore::WalkBuilder;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

pub fn scan_path(path: &Path) -> io::Result<ScanSummary> {
    scan_path_with_config(path, &ScanConfig::default())
}

pub fn scan_path_with_config(path: &Path, config: &ScanConfig) -> io::Result<ScanSummary> {
    let file_audits = build_file_audits();
    let (facts, mut findings) = collect_and_audit_inline(path, config, &file_audits)?;
    findings.extend(run_project_audits(&facts, config));
    let (coupling_findings, coupling_graph) =
        ImportCouplingAudit.audit_with_graph(&facts, config, path);
    findings.extend(coupling_findings);

    for finding in &mut findings {
        finding.id = stable_finding_key(finding, path);
    }

    Ok(ScanSummary {
        root_path: facts.root_path,
        files_count: facts.files_count,
        directories_count: facts.directories_count,
        lines_of_code: facts.lines_of_code,
        skipped_files_count: facts.skipped_files_count,
        skipped_bytes: facts.skipped_bytes,
        languages: facts.languages,
        findings,
        coupling_graph: Some(coupling_graph),
    })
}

/// Collects scan facts while running file audits inline — content is dropped after
/// each file's audits complete, so only one file's content lives in memory at a time.
fn collect_and_audit_inline(
    path: &Path,
    config: &ScanConfig,
    file_audits: &[Box<dyn FileAudit>],
) -> io::Result<(ScanFacts, Vec<Finding>)> {
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
    let mut findings: Vec<Finding> = Vec::new();

    if path.is_file() {
        audit_file_inline(
            path,
            &mut facts,
            &mut languages,
            file_audits,
            config,
            &mut findings,
        )?;
    } else {
        let walker = build_walker(path, config);

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
            } else if file_type.is_file() {
                audit_file_inline(
                    entry_path,
                    &mut facts,
                    &mut languages,
                    file_audits,
                    config,
                    &mut findings,
                )?;
            }
        }
    }

    facts.languages = build_language_summary(languages);
    Ok((facts, findings))
}

/// Reads one file, runs all file audits with the content, then stores FileFacts
/// without content so memory is freed before moving to the next file.
fn audit_file_inline(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    file_audits: &[Box<dyn FileAudit>],
    config: &ScanConfig,
    findings: &mut Vec<Finding>,
) -> io::Result<()> {
    facts.files_count += 1;

    let language = detect_language(path).map(str::to_string);
    if let Some(language_name) = &language {
        *languages.entry(language_name.clone()).or_insert(0) += 1;
    }

    if skip_oversized_file(path, facts, &language, config)? {
        return Ok(());
    }

    let Ok(content) = fs::read_to_string(path) else {
        track_skipped_file(path, facts, 0);
        facts.files.push(FileFacts {
            path: path.to_path_buf(),
            language,
            lines_of_code: 0,
            branch_count: 0,
            imports: Vec::new(),
            content: String::new(),
        });
        return Ok(());
    };

    let lines_of_code = count_lines_of_code(&content);
    let branch_count = count_branches(&content);
    let imports = extract_imports(&content, language.as_deref());
    facts.lines_of_code += lines_of_code;

    let file_facts = FileFacts {
        path: path.to_path_buf(),
        language,
        lines_of_code,
        branch_count,
        imports,
        content,
    };

    for audit in file_audits {
        findings.extend(audit.audit(&file_facts, config));
    }

    // Push without content — project audits only need path and line counts
    facts.files.push(FileFacts {
        path: file_facts.path,
        language: file_facts.language,
        lines_of_code: file_facts.lines_of_code,
        branch_count: file_facts.branch_count,
        imports: file_facts.imports,
        content: String::new(),
    });

    Ok(())
}

// ── Legacy path: collect facts only, keeps content (used by library consumers) ──

pub fn collect_scan_facts(path: &Path) -> io::Result<ScanFacts> {
    collect_scan_facts_with_config(path, &ScanConfig::default())
}

pub fn collect_scan_facts_with_config(path: &Path, config: &ScanConfig) -> io::Result<ScanFacts> {
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
        collect_file_facts(path, &mut facts, &mut languages, config)?;
    } else {
        collect_directory_facts(path, &mut facts, &mut languages, config)?;
    }

    facts.languages = build_language_summary(languages);

    Ok(facts)
}

fn collect_directory_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    config: &ScanConfig,
) -> io::Result<()> {
    let walker = build_walker(path, config);

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
            collect_file_facts(entry_path, facts, languages, config)?;
        }
    }

    Ok(())
}

fn build_walker(path: &Path, config: &ScanConfig) -> ignore::Walk {
    let root = path.to_path_buf();
    let ignored_paths = config.ignored_paths.clone();
    WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(move |entry| !is_ignored_path(entry.path(), &root, &ignored_paths))
        .build()
}

fn is_ignored_path(path: &Path, root: &Path, ignored_paths: &[String]) -> bool {
    if path == root {
        return false;
    }

    ignored_paths.iter().any(|ignored_path| {
        let ignored_path = ignored_path.trim_matches('/');

        if ignored_path.is_empty() {
            return false;
        }

        path.strip_prefix(root)
            .ok()
            .and_then(|relative_path| relative_path.to_str())
            .map(|relative_path| relative_path == ignored_path)
            .unwrap_or(false)
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == ignored_path)
                .unwrap_or(false)
    })
}

fn collect_file_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    config: &ScanConfig,
) -> io::Result<()> {
    facts.files_count += 1;

    let language = detect_language(path).map(str::to_string);

    if let Some(language_name) = &language {
        *languages.entry(language_name.clone()).or_insert(0) += 1;
    }

    if skip_oversized_file(path, facts, &language, config)? {
        return Ok(());
    }

    let Ok(content) = fs::read_to_string(path) else {
        track_skipped_file(path, facts, 0);
        facts.files.push(FileFacts {
            path: path.to_path_buf(),
            language,
            lines_of_code: 0,
            branch_count: 0,
            imports: Vec::new(),
            content: String::new(),
        });
        return Ok(());
    };

    let lines_of_code = count_lines_of_code(&content);
    facts.lines_of_code += lines_of_code;

    facts.files.push(FileFacts {
        path: path.to_path_buf(),
        imports: extract_imports(&content, language.as_deref()),
        branch_count: count_branches(&content),
        language,
        lines_of_code,
        content,
    });

    Ok(())
}

fn skip_oversized_file(
    path: &Path,
    facts: &mut ScanFacts,
    language: &Option<String>,
    config: &ScanConfig,
) -> io::Result<bool> {
    if config.max_file_bytes == 0 {
        return Ok(false);
    }

    let metadata = fs::metadata(path)?;
    let file_size = metadata.len();

    if file_size <= config.max_file_bytes {
        return Ok(false);
    }

    track_skipped_file(path, facts, file_size);
    facts.files.push(FileFacts {
        path: path.to_path_buf(),
        language: language.clone(),
        lines_of_code: 0,
        branch_count: 0,
        imports: Vec::new(),
        content: String::new(),
    });

    Ok(true)
}

fn track_skipped_file(path: &Path, facts: &mut ScanFacts, skipped_bytes: u64) {
    facts.skipped_files_count += 1;
    facts.skipped_bytes = facts.skipped_bytes.saturating_add(skipped_bytes);

    if skipped_bytes > 0 {
        return;
    }

    if let Ok(metadata) = fs::metadata(path) {
        facts.skipped_bytes = facts.skipped_bytes.saturating_add(metadata.len());
    }
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
