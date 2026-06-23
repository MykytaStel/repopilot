use crate::findings::types::Finding;
use crate::scan::cache::{FileRoleEntry, FindingsEntry, relative_cache_path};
use crate::scan::facts::{FileFacts, ScanFacts};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) enum CacheDecision {
    Hit {
        role_entry: Box<FileRoleEntry>,
        findings: Vec<Finding>,
    },
    Miss {
        reason: &'static str,
    },
}

pub(super) fn cache_decision(
    role_entry: Option<&FileRoleEntry>,
    findings_entry: Option<&FindingsEntry>,
    hash: &str,
    fingerprint: &str,
) -> CacheDecision {
    match (role_entry, findings_entry) {
        (Some(role_entry), Some(findings_entry))
            if role_entry.hash == hash
                && findings_entry.hash == hash
                && findings_entry.config_fingerprint == fingerprint =>
        {
            CacheDecision::Hit {
                role_entry: Box::new(role_entry.clone()),
                findings: findings_entry.findings.clone(),
            }
        }
        (None, None) => CacheDecision::Miss {
            reason: "missing-cache-entry",
        },
        (None, Some(_)) => CacheDecision::Miss {
            reason: "missing-file-role-cache",
        },
        (Some(_), None) => CacheDecision::Miss {
            reason: "missing-findings-cache",
        },
        (Some(role_entry), Some(findings_entry))
            if role_entry.hash != hash || findings_entry.hash != hash =>
        {
            CacheDecision::Miss {
                reason: "content-changed",
            }
        }
        (Some(_), Some(findings_entry)) if findings_entry.config_fingerprint != fingerprint => {
            CacheDecision::Miss {
                reason: "config-changed",
            }
        }
        (Some(_), Some(_)) => CacheDecision::Miss {
            reason: "cache-mismatch",
        },
    }
}

pub(super) fn record_cached_file(
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    entry: &FileRoleEntry,
) -> FileFacts {
    facts.files_analyzed += 1;
    facts.non_empty_lines += entry.non_empty_lines;
    if let Some(language) = &entry.language {
        *languages.entry(language.clone()).or_insert(0) += 1;
    }
    let file_facts = FileFacts {
        path: PathBuf::from(&entry.path),
        language: entry.language.clone(),
        non_empty_lines: entry.non_empty_lines,
        branch_count: 0,
        imports: entry.imports.clone(),
        content: None,
        has_inline_tests: entry.is_test,
        in_executable_package: false,
        deferred_imports: entry.deferred_imports.clone(),
    };
    facts.files.push(file_facts.clone());
    file_facts
}

pub(super) fn normalize_per_file_paths(
    path: &mut PathBuf,
    findings: &mut [Finding],
    repo_root: &Path,
) {
    *path = PathBuf::from(relative_cache_path(repo_root, path));
    for finding in findings {
        for evidence in &mut finding.evidence {
            evidence.path = PathBuf::from(relative_cache_path(repo_root, &evidence.path));
        }
    }
}
