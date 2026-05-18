use crate::audits::pipeline::{registered_file_audits, registered_project_audits};
use crate::findings::types::Finding;
use crate::findings::types::{FindingCategory, Severity};
use crate::rules::registry::all_rule_metadata;
use crate::scan::config::ScanConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const CACHE_SCHEMA_VERSION: u32 = 2;
pub const CACHE_DIR: &str = ".repopilot/cache";
const FILE_HASHES_NAME: &str = "file_hashes.json";
const FILE_ROLES_NAME: &str = "file_roles.json";
const FINDINGS_NAME: &str = "findings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileHashEntry {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub modified_unix_seconds: u64,
    pub cache_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileRoleEntry {
    pub path: String,
    pub hash: String,
    pub language: Option<String>,
    pub non_empty_lines: usize,
    pub roles: Vec<String>,
    pub frameworks: Vec<String>,
    pub runtimes: Vec<String>,
    pub paradigms: Vec<String>,
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingsEntry {
    pub path: String,
    pub hash: String,
    pub config_fingerprint: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileHashesCache {
    pub schema_version: u32,
    pub repopilot_version: String,
    pub entries: Vec<FileHashEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileRolesCache {
    pub schema_version: u32,
    pub repopilot_version: String,
    pub entries: Vec<FileRoleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingsCache {
    pub schema_version: u32,
    pub repopilot_version: String,
    pub entries: Vec<FindingsEntry>,
}

#[derive(Debug, Default)]
pub struct ScanCache {
    pub file_hashes: BTreeMap<String, FileHashEntry>,
    pub file_roles: BTreeMap<String, FileRoleEntry>,
    pub findings: BTreeMap<String, FindingsEntry>,
}

impl ScanCache {
    pub fn load(root: &Path) -> Self {
        let cache_root = cache_dir(root);
        Self {
            file_hashes: read_cache::<FileHashesCache>(&cache_root.join(FILE_HASHES_NAME))
                .filter(valid_file_hashes_cache)
                .map(|cache| entries_by_path(cache.entries))
                .unwrap_or_default(),
            file_roles: read_cache::<FileRolesCache>(&cache_root.join(FILE_ROLES_NAME))
                .filter(valid_file_roles_cache)
                .map(|cache| entries_by_path(cache.entries))
                .unwrap_or_default(),
            findings: read_cache::<FindingsCache>(&cache_root.join(FINDINGS_NAME))
                .filter(valid_findings_cache)
                .map(|cache| entries_by_path(cache.entries))
                .unwrap_or_default(),
        }
    }

    pub fn write(&self, root: &Path) -> io::Result<()> {
        let cache_root = cache_dir(root);
        fs::create_dir_all(&cache_root)?;

        write_cache(
            &cache_root.join(FILE_HASHES_NAME),
            &FileHashesCache {
                schema_version: CACHE_SCHEMA_VERSION,
                repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
                entries: self.file_hashes.values().cloned().collect(),
            },
        )?;
        write_cache(
            &cache_root.join(FILE_ROLES_NAME),
            &FileRolesCache {
                schema_version: CACHE_SCHEMA_VERSION,
                repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
                entries: self.file_roles.values().cloned().collect(),
            },
        )?;
        write_cache(
            &cache_root.join(FINDINGS_NAME),
            &FindingsCache {
                schema_version: CACHE_SCHEMA_VERSION,
                repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
                entries: self.findings.values().cloned().collect(),
            },
        )?;

        Ok(())
    }
}

pub fn clear_cache(root: &Path) -> io::Result<()> {
    let cache_root = cache_dir(root);
    match fs::remove_dir_all(cache_root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(CACHE_DIR)
}

pub fn file_hash_entry(root: &Path, path: &Path) -> io::Result<FileHashEntry> {
    let bytes = fs::read(path)?;
    let metadata = fs::metadata(path)?;
    let relative = relative_cache_path(root, path);
    let hash = stable_hash_hex(&bytes);
    let modified_unix_seconds = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let cache_key = stable_hash_hex(format!("{relative}:{hash}:{}", metadata.len()).as_bytes());

    Ok(FileHashEntry {
        path: relative,
        hash,
        size: metadata.len(),
        modified_unix_seconds,
        cache_key,
    })
}

pub fn config_fingerprint(config: &ScanConfig) -> String {
    let input = CacheFingerprintInput {
        cache_schema_version: CACHE_SCHEMA_VERSION,
        repopilot_version: env!("CARGO_PKG_VERSION"),
        scan_config: config,
        file_audits: registered_file_audits(config)
            .into_iter()
            .map(|registration| AuditFingerprint {
                audit_id: registration.metadata.audit_id,
                kind: registration.metadata.kind.label(),
                category: registration.metadata.category,
                rule_ids: registration.metadata.rule_ids.to_vec(),
            })
            .collect(),
        project_audits: registered_project_audits(config)
            .into_iter()
            .map(|registration| AuditFingerprint {
                audit_id: registration.metadata.audit_id,
                kind: registration.metadata.kind.label(),
                category: registration.metadata.category,
                rule_ids: registration.metadata.rule_ids.to_vec(),
            })
            .collect(),
        rules: all_rule_metadata()
            .map(|rule| RuleFingerprint {
                rule_id: rule.rule_id,
                title: rule.title,
                category: rule.category.clone(),
                default_severity: rule.default_severity,
                docs_url: rule.docs_url,
                description: rule.description,
                recommendation: rule.recommendation,
            })
            .collect(),
    };

    match serde_json::to_vec(&input) {
        Ok(bytes) => stable_hash_hex(&bytes),
        Err(_) => stable_hash_hex(format!("{config:?}:{}", CACHE_SCHEMA_VERSION).as_bytes()),
    }
}

pub fn stable_hash_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Serialize)]
struct CacheFingerprintInput<'a> {
    cache_schema_version: u32,
    repopilot_version: &'static str,
    scan_config: &'a ScanConfig,
    file_audits: Vec<AuditFingerprint>,
    project_audits: Vec<AuditFingerprint>,
    rules: Vec<RuleFingerprint>,
}

#[derive(Serialize)]
struct AuditFingerprint {
    audit_id: &'static str,
    kind: &'static str,
    category: FindingCategory,
    rule_ids: Vec<&'static str>,
}

#[derive(Serialize)]
struct RuleFingerprint {
    rule_id: &'static str,
    title: &'static str,
    category: FindingCategory,
    default_severity: Severity,
    docs_url: Option<&'static str>,
    description: &'static str,
    recommendation: Option<&'static str>,
}

pub fn relative_cache_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn read_cache<T>(path: &Path) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_cache<T>(path: &Path, value: &T) -> io::Result<()>
where
    T: Serialize,
{
    let rendered = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    fs::write(path, rendered)
}

fn entries_by_path<T>(entries: Vec<T>) -> BTreeMap<String, T>
where
    T: CachePath,
{
    entries
        .into_iter()
        .map(|entry| (entry.cache_path().to_string(), entry))
        .collect()
}

trait CachePath {
    fn cache_path(&self) -> &str;
}

impl CachePath for FileHashEntry {
    fn cache_path(&self) -> &str {
        &self.path
    }
}

impl CachePath for FileRoleEntry {
    fn cache_path(&self) -> &str {
        &self.path
    }
}

impl CachePath for FindingsEntry {
    fn cache_path(&self) -> &str {
        &self.path
    }
}

fn valid_file_hashes_cache(cache: &FileHashesCache) -> bool {
    cache.schema_version == CACHE_SCHEMA_VERSION
        && cache.repopilot_version == env!("CARGO_PKG_VERSION")
}

fn valid_file_roles_cache(cache: &FileRolesCache) -> bool {
    cache.schema_version == CACHE_SCHEMA_VERSION
        && cache.repopilot_version == env!("CARGO_PKG_VERSION")
}

fn valid_findings_cache(cache: &FindingsCache) -> bool {
    cache.schema_version == CACHE_SCHEMA_VERSION
        && cache.repopilot_version == env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheDiagnostics {
    pub cache_dir: PathBuf,
    pub exists: bool,
    pub schema_version: u32,
    pub repopilot_version: String,
    pub approximate_size_bytes: u64,
    pub file_hashes_count: usize,
    pub file_roles_count: usize,
    pub findings_count: usize,
    pub stale_entries_count: usize,
}

pub fn inspect_cache(root: &Path) -> CacheDiagnostics {
    let cache_root = cache_dir(root);
    let exists = cache_root.is_dir();
    let cache = ScanCache::load(root);

    CacheDiagnostics {
        cache_dir: cache_root.clone(),
        exists,
        schema_version: CACHE_SCHEMA_VERSION,
        repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
        approximate_size_bytes: directory_size_bytes(&cache_root),
        file_hashes_count: cache.file_hashes.len(),
        file_roles_count: cache.file_roles.len(),
        findings_count: cache.findings.len(),
        stale_entries_count: stale_cache_entries_count(&cache),
    }
}

fn stale_cache_entries_count(cache: &ScanCache) -> usize {
    let mut count = 0;

    for key in cache.file_hashes.keys() {
        if !cache.file_roles.contains_key(key) || !cache.findings.contains_key(key) {
            count += 1;
        }
    }

    for key in cache.file_roles.keys() {
        if !cache.file_hashes.contains_key(key) {
            count += 1;
        }
    }

    for key in cache.findings.keys() {
        if !cache.file_hashes.contains_key(key) {
            count += 1;
        }
    }

    count
}

fn directory_size_bytes(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let entry_path = entry.path();
            match entry.metadata() {
                Ok(metadata) if metadata.is_file() => metadata.len(),
                Ok(metadata) if metadata.is_dir() => directory_size_bytes(&entry_path),
                _ => 0,
            }
        })
        .sum()
}
