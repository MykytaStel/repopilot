use crate::analysis::{ParsedArtifact, SyntaxSummary};
use crate::scan::cache::{cache_dir, stable_hash_hex};
use crate::scan::facts::FileFacts;
use crate::scan::types::ParsedCacheTelemetry;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

const PARSED_FACTS_SCHEMA_VERSION: u32 = 2;
const PARSED_FACTS_ANALYSIS_VERSION: &str = "tree-sitter-imports-exports-v1";
const PARSED_FACTS_NAME: &str = "parsed_facts_v2.json";
const MAX_PARSED_FACTS_ENTRIES: usize = 16_384;

#[derive(Debug, Default, Clone)]
pub struct ParsedFactsCache {
    entries: BTreeMap<String, ParsedFactsEntry>,
    referenced_hashes: BTreeSet<String>,
    telemetry: ParsedCacheTelemetry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedFactsEntry {
    pub content_hash: String,
    pub language: Option<String>,
    pub non_empty_lines: usize,
    pub branch_count: usize,
    pub has_inline_tests: bool,
    pub imports: Vec<String>,
    pub deferred_imports: Vec<String>,
    pub exports: Vec<String>,
    pub syntax: CachedSyntaxSummary,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedSyntaxSummary {
    pub parsed: bool,
    pub root_kind: Option<String>,
    pub has_errors: bool,
    pub named_child_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ParsedFactsFile {
    schema_version: u32,
    analysis_version: String,
    repopilot_version: String,
    entries: Vec<ParsedFactsEntry>,
}

impl ParsedFactsCache {
    pub fn load(root: &Path) -> Self {
        let path = cache_dir(root).join(PARSED_FACTS_NAME);
        let Some(cache) = read_cache::<ParsedFactsFile>(&path) else {
            return Self {
                telemetry: ParsedCacheTelemetry {
                    corruptions: usize::from(path.exists()),
                    ..Default::default()
                },
                ..Default::default()
            };
        };

        let entries_count = cache.entries.len();
        if cache.schema_version != PARSED_FACTS_SCHEMA_VERSION
            || cache.analysis_version != PARSED_FACTS_ANALYSIS_VERSION
        {
            return Self {
                telemetry: ParsedCacheTelemetry {
                    invalidations: entries_count,
                    ..Default::default()
                },
                ..Default::default()
            };
        }

        let entries: BTreeMap<_, _> = cache
            .entries
            .into_iter()
            .map(|entry| (entry.cache_key(), entry))
            .collect();
        Self {
            telemetry: ParsedCacheTelemetry {
                entries_loaded: entries.len(),
                ..Default::default()
            },
            entries,
            ..Default::default()
        }
    }

    pub fn get(&self, content_hash: &str, language: Option<&str>) -> Option<&ParsedFactsEntry> {
        self.entries.get(&cache_key(content_hash, language))
    }

    pub fn lookup(
        &mut self,
        content_hash: &str,
        language: Option<&str>,
    ) -> Option<ParsedFactsEntry> {
        self.referenced_hashes.insert(content_hash.to_string());
        let entry = self
            .entries
            .get(&cache_key(content_hash, language))
            .cloned();
        if entry.is_some() {
            self.telemetry.hits += 1;
        } else {
            self.telemetry.misses += 1;
        }
        entry
    }

    pub fn insert(&mut self, entry: ParsedFactsEntry) {
        self.referenced_hashes.insert(entry.content_hash.clone());
        self.entries.insert(entry.cache_key(), entry);
    }

    pub fn retain_referenced_current_scan(&mut self) {
        if self.referenced_hashes.is_empty() {
            return;
        }
        let before = self.entries.len();
        self.entries
            .retain(|_, entry| self.referenced_hashes.contains(&entry.content_hash));
        self.telemetry.invalidations += before.saturating_sub(self.entries.len());
    }

    pub fn remove_content_hash(&mut self, content_hash: &str) {
        let before = self.entries.len();
        self.entries
            .retain(|_, entry| entry.content_hash != content_hash);
        self.telemetry.invalidations += before.saturating_sub(self.entries.len());
    }

    pub fn telemetry(&self) -> ParsedCacheTelemetry {
        self.telemetry
    }

    pub fn write(&mut self, root: &Path) -> io::Result<()> {
        let cache_root = cache_dir(root);
        fs::create_dir_all(&cache_root)?;
        self.enforce_entry_limit();
        let file = ParsedFactsFile {
            schema_version: PARSED_FACTS_SCHEMA_VERSION,
            analysis_version: PARSED_FACTS_ANALYSIS_VERSION.to_string(),
            repopilot_version: env!("CARGO_PKG_VERSION").to_string(),
            entries: self.entries.values().cloned().collect(),
        };
        self.telemetry.entries_written = file.entries.len();
        let rendered = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        fs::write(cache_root.join(PARSED_FACTS_NAME), rendered)
    }

    fn enforce_entry_limit(&mut self) {
        let overflow = self.entries.len().saturating_sub(MAX_PARSED_FACTS_ENTRIES);
        if overflow == 0 {
            return;
        }
        let stale_keys: Vec<String> = self.entries.keys().take(overflow).cloned().collect();
        for key in stale_keys {
            self.entries.remove(&key);
        }
        self.telemetry.invalidations += overflow;
    }
}

impl ParsedFactsEntry {
    pub fn from_artifact(
        content_hash: String,
        file: &FileFacts,
        artifact: &ParsedArtifact,
    ) -> Self {
        Self {
            content_hash,
            language: file.language.clone(),
            non_empty_lines: file.non_empty_lines,
            branch_count: file.branch_count,
            has_inline_tests: file.has_inline_tests,
            imports: file.imports.clone(),
            deferred_imports: file.deferred_imports.clone(),
            exports: artifact.exports.clone(),
            syntax: CachedSyntaxSummary::from(&artifact.syntax),
        }
    }

    fn cache_key(&self) -> String {
        cache_key(&self.content_hash, self.language.as_deref())
    }
}

impl From<&SyntaxSummary> for CachedSyntaxSummary {
    fn from(value: &SyntaxSummary) -> Self {
        Self {
            parsed: value.parsed,
            root_kind: value.root_kind.clone(),
            has_errors: value.has_errors,
            named_child_count: value.named_child_count,
        }
    }
}

impl From<&CachedSyntaxSummary> for SyntaxSummary {
    fn from(value: &CachedSyntaxSummary) -> Self {
        Self {
            parsed: value.parsed,
            root_kind: value.root_kind.clone(),
            has_errors: value.has_errors,
            named_child_count: value.named_child_count,
        }
    }
}

pub fn content_hash(content: &str) -> String {
    stable_hash_hex(content.as_bytes())
}

fn cache_key(content_hash: &str, language: Option<&str>) -> String {
    format!("{}:{}", content_hash, language.unwrap_or(""))
}

fn read_cache<T>(path: &Path) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{FileContextFacts, ParsedArtifact, SyntaxSummary};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn parsed_facts_cache_round_trips_by_content_hash_and_language() {
        let temp = tempdir().expect("temp dir");
        let mut cache = ParsedFactsCache::default();
        let file = FileFacts {
            path: PathBuf::from("src/lib.rs"),
            language: Some("Rust".to_string()),
            non_empty_lines: 1,
            branch_count: 0,
            imports: vec!["crate::api".to_string()],
            deferred_imports: Vec::new(),
            content: None,
            has_inline_tests: false,
            in_executable_package: false,
        };
        let artifact = ParsedArtifact::from_source(
            file.path.clone(),
            file.language.clone(),
            file.imports.clone(),
            Vec::new(),
            vec!["run".to_string()],
            FileContextFacts::default(),
            SyntaxSummary {
                parsed: true,
                root_kind: Some("source_file".to_string()),
                has_errors: false,
                named_child_count: 3,
            },
        );
        cache.insert(ParsedFactsEntry::from_artifact(
            "hash".to_string(),
            &file,
            &artifact,
        ));
        cache.write(temp.path()).expect("write parsed facts cache");

        let loaded = ParsedFactsCache::load(temp.path());
        let entry = loaded
            .get("hash", Some("Rust"))
            .expect("parsed facts cache hit");
        assert_eq!(entry.exports, vec!["run"]);
        assert_eq!(entry.syntax.root_kind.as_deref(), Some("source_file"));
        assert_eq!(loaded.telemetry().entries_loaded, 1);
    }

    #[test]
    fn corrupt_parsed_facts_cache_is_discarded() {
        let temp = tempdir().expect("temp dir");
        let cache_root = cache_dir(temp.path());
        fs::create_dir_all(&cache_root).expect("create cache dir");
        fs::write(cache_root.join(PARSED_FACTS_NAME), "{not json").expect("write corrupt cache");

        let loaded = ParsedFactsCache::load(temp.path());
        assert!(loaded.get("hash", Some("Rust")).is_none());
        assert_eq!(loaded.telemetry().corruptions, 1);
    }

    #[test]
    fn parsed_facts_cache_rejects_analysis_version_mismatch() {
        let temp = tempdir().expect("temp dir");
        let cache_root = cache_dir(temp.path());
        fs::create_dir_all(&cache_root).expect("create cache dir");
        let cache = ParsedFactsFile {
            schema_version: PARSED_FACTS_SCHEMA_VERSION,
            analysis_version: "old-imports".to_string(),
            repopilot_version: "test".to_string(),
            entries: vec![ParsedFactsEntry {
                content_hash: "hash".to_string(),
                language: Some("Rust".to_string()),
                non_empty_lines: 1,
                branch_count: 0,
                has_inline_tests: false,
                imports: Vec::new(),
                deferred_imports: Vec::new(),
                exports: Vec::new(),
                syntax: CachedSyntaxSummary::default(),
            }],
        };
        fs::write(
            cache_root.join(PARSED_FACTS_NAME),
            serde_json::to_string(&cache).expect("render parsed cache"),
        )
        .expect("write cache");

        let loaded = ParsedFactsCache::load(temp.path());
        assert!(loaded.get("hash", Some("Rust")).is_none());
        assert_eq!(loaded.telemetry().invalidations, 1);
    }

    #[test]
    fn retain_referenced_current_scan_prunes_unseen_hashes() {
        let temp = tempdir().expect("temp dir");
        let mut cache = ParsedFactsCache::default();
        cache.insert(entry("old"));
        cache.insert(entry("live"));
        cache.write(temp.path()).expect("write cache");

        let mut cache = ParsedFactsCache::load(temp.path());
        assert!(cache.lookup("live", Some("Rust")).is_some());
        cache.retain_referenced_current_scan();

        assert!(cache.get("live", Some("Rust")).is_some());
        assert!(cache.get("old", Some("Rust")).is_none());
        assert_eq!(cache.telemetry().invalidations, 1);
    }

    fn entry(hash: &str) -> ParsedFactsEntry {
        ParsedFactsEntry {
            content_hash: hash.to_string(),
            language: Some("Rust".to_string()),
            non_empty_lines: 1,
            branch_count: 0,
            has_inline_tests: false,
            imports: Vec::new(),
            deferred_imports: Vec::new(),
            exports: Vec::new(),
            syntax: CachedSyntaxSummary::default(),
        }
    }
}
