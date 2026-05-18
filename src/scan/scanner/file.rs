mod read;

use crate::audits::context::classify_file;
use crate::audits::traits::FileAudit;
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use read::{LoadedFile, empty_file_facts, load_file, without_content};
use std::collections::HashMap;
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
        full_facts,
        language,
    } = loaded
    else {
        return Ok(skipped_result_from_loaded(path, loaded));
    };

    let context = PerFileContext::from_file(&full_facts);
    let mut findings = Vec::new();
    for audit in file_audits {
        findings.extend(audit.audit(&full_facts, config));
    }

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

pub(super) fn audit_file_inline(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    file_audits: &[Box<dyn FileAudit>],
    config: &ScanConfig,
    findings: &mut Vec<Finding>,
) -> io::Result<()> {
    let LoadedFile::Analyzable { full_facts, .. } = load_file_or_record_skip(path, facts, config)?
    else {
        return Ok(());
    };

    record_analyzed_file(facts, languages, &full_facts);

    for audit in file_audits {
        findings.extend(audit.audit(&full_facts, config));
    }

    facts.files.push(without_content(full_facts));

    Ok(())
}

pub(super) fn collect_file_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    config: &ScanConfig,
) -> io::Result<()> {
    let LoadedFile::Analyzable { full_facts, .. } = load_file_or_record_skip(path, facts, config)?
    else {
        return Ok(());
    };

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
    facts.files_count += 1;
    facts.lines_of_code += file_facts.lines_of_code;
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
    facts.skipped_files_count += 1;
    facts.skipped_bytes = facts.skipped_bytes.saturating_add(skipped_bytes);
}

fn track_binary_file(facts: &mut ScanFacts, skipped_bytes: u64) {
    facts.binary_files_skipped += 1;
    facts.skipped_bytes = facts.skipped_bytes.saturating_add(skipped_bytes);
}
