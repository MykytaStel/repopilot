use super::file::{SkipReason, audit_file_inline, collect_file_facts, process_file};
use super::summary::build_language_summary;
use super::walker::collect_paths;
use crate::audits::traits::FileAudit;
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io;
use std::path::Path;

pub(super) fn collect_and_audit_inline(
    path: &Path,
    config: &ScanConfig,
    file_audits: &[Box<dyn FileAudit>],
) -> io::Result<(ScanFacts, Vec<Finding>)> {
    ensure_path_exists(path)?;

    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        ..ScanFacts::default()
    };

    if path.is_file() {
        facts.files_discovered = 1;
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut findings: Vec<Finding> = Vec::new();
        audit_file_inline(
            path,
            &mut facts,
            &mut languages,
            file_audits,
            config,
            &mut findings,
        )?;
        facts.languages = build_language_summary(languages);
        return Ok((facts, findings));
    }

    let collected = collect_paths(path, config)?;
    let mut file_paths = collected.file_paths;

    facts.files_discovered = file_paths.len();
    facts.files_skipped_repopilotignore = collected.files_skipped_repopilotignore;
    facts.repopilotignore_path = collected.repopilotignore_path;
    facts.directories_count = collected.directories_count;

    apply_max_files_limit(&mut file_paths, &mut facts, config);

    let results: Vec<io::Result<_>> = file_paths
        .par_iter()
        .map(|p| process_file(p, file_audits, config))
        .collect();

    let mut languages: HashMap<String, usize> = HashMap::new();
    let mut findings: Vec<Finding> = Vec::new();

    for result in results {
        let per_file = result?;
        if per_file.skip_reason == SkipReason::None {
            facts.files_count += 1;
            if let Some(ref lang) = per_file.language {
                *languages.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        facts.lines_of_code += per_file.file_facts.lines_of_code;
        match per_file.skip_reason {
            SkipReason::LargeFile => {
                facts.skipped_files_count += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::Binary => {
                facts.binary_files_skipped += 1;
                facts.skipped_bytes = facts.skipped_bytes.saturating_add(per_file.skipped_bytes);
            }
            SkipReason::LowSignal => {
                facts.files_skipped_low_signal += 1;
            }
            SkipReason::None => {}
        }
        facts.files.push(per_file.file_facts);
        findings.extend(per_file.findings);
    }

    facts.languages = build_language_summary(languages);
    Ok((facts, findings))
}

/// Collects scan facts and retains file contents for every readable file.
///
/// The CLI scan path uses `scan_path_with_config`, which runs file audits inline
/// and drops each file's content after auditing. Prefer that path for large
/// repositories unless callers explicitly need the collected source text.
pub fn collect_scan_facts(path: &Path) -> io::Result<ScanFacts> {
    collect_scan_facts_with_config(path, &ScanConfig::default())
}

pub fn collect_scan_facts_with_config(path: &Path, config: &ScanConfig) -> io::Result<ScanFacts> {
    ensure_path_exists(path)?;

    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        ..ScanFacts::default()
    };

    let mut languages: HashMap<String, usize> = HashMap::new();

    if path.is_file() {
        facts.files_discovered = 1;
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
    let collected = collect_paths(path, config)?;
    let mut file_paths = collected.file_paths;

    facts.files_discovered = file_paths.len();
    facts.files_skipped_repopilotignore = collected.files_skipped_repopilotignore;
    facts.repopilotignore_path = collected.repopilotignore_path;
    facts.directories_count = collected.directories_count;

    apply_max_files_limit(&mut file_paths, facts, config);

    for entry_path in file_paths {
        collect_file_facts(&entry_path, facts, languages, config)?;
    }

    Ok(())
}

fn ensure_path_exists(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("path does not exist: {}", path.display()),
    ))
}

fn apply_max_files_limit(
    file_paths: &mut Vec<std::path::PathBuf>,
    facts: &mut ScanFacts,
    config: &ScanConfig,
) {
    let Some(max) = config.max_files else {
        return;
    };

    if file_paths.len() <= max {
        return;
    }

    facts.files_skipped_by_limit = file_paths.len().saturating_sub(max);
    file_paths.truncate(max);
}
