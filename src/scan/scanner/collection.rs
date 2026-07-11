use super::file::{SkipReason, collect_file_facts, collect_file_facts_with_cache, process_file};
use super::summary::build_language_summary;
use super::walker::collect_paths;
use crate::audits::pipeline::registered_file_audits;
use crate::findings::types::Finding;
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use crate::scan::parsed_cache::ParsedFactsCache;
use crate::scan::workspace::{PackageRoot, package_roots};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

pub(super) struct DiscoveredScanPaths {
    pub(super) facts: ScanFacts,
    pub(super) file_paths: Vec<PathBuf>,
}

pub(super) fn discover_scan_paths(
    path: &Path,
    config: &ScanConfig,
) -> io::Result<DiscoveredScanPaths> {
    ensure_path_exists(path)?;

    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        ..ScanFacts::default()
    };

    if path.is_file() {
        facts.files_discovered = 1;
        return Ok(DiscoveredScanPaths {
            facts,
            file_paths: vec![path.to_path_buf()],
        });
    }

    let collected = collect_paths(path, config)?;
    let mut file_paths = collected.file_paths;
    file_paths.sort();

    facts.files_discovered = file_paths.len();
    facts.files_skipped_repopilotignore = collected.files_skipped_repopilotignore;
    facts.repopilotignore_path = collected.repopilotignore_path;
    facts.directories_count = collected.directories_count;

    apply_max_files_limit(&mut file_paths, &mut facts, config);

    Ok(DiscoveredScanPaths { facts, file_paths })
}

pub(super) fn analyze_discovered_files(
    discovered: DiscoveredScanPaths,
    config: &ScanConfig,
) -> io::Result<(ScanFacts, Vec<Finding>)> {
    let DiscoveredScanPaths {
        mut facts,
        file_paths,
    } = discovered;
    let file_audits = registered_file_audits(config);

    // Package roots are resolved once from the scan root, then passed down so each
    // file's `in_executable_package` flag (set from its nearest package's manifest)
    // is known before its audits run — host-exit severity is decided at finding time.
    let roots = package_roots(&facts.root_path);

    let results: Vec<io::Result<_>> = file_paths
        .par_iter()
        .map(|p| process_file(p, &file_audits, config, &roots))
        .collect();

    let mut languages: HashMap<String, usize> = HashMap::new();
    let mut findings: Vec<Finding> = Vec::new();

    for result in results {
        let per_file = result?;
        if per_file.skip_reason == SkipReason::None {
            facts.files_analyzed += 1;
            if let Some(ref lang) = per_file.language {
                *languages.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        facts.non_empty_lines += per_file.file_facts.non_empty_lines;
        match per_file.skip_reason {
            SkipReason::LargeFile => {
                facts.large_files_skipped += 1;
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
        if let Some(artifact) = per_file.artifact {
            facts.insert_artifact(artifact);
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
    let roots = package_roots(&facts.root_path);

    if path.is_file() {
        facts.files_discovered = 1;
        collect_file_facts(path, &mut facts, &mut languages, config, &roots)?;
    } else {
        collect_directory_facts(path, &mut facts, &mut languages, config, &roots)?;
    }

    facts.languages = build_language_summary(languages);

    Ok(facts)
}

#[cfg(test)]
fn collect_scan_facts_without_content(path: &Path, config: &ScanConfig) -> io::Result<ScanFacts> {
    ensure_path_exists(path)?;
    let cache_root = parsed_cache_root(path);
    let mut parsed_cache = ParsedFactsCache::load(&cache_root);
    let facts =
        collect_scan_facts_without_content_with_parsed_cache(path, config, &mut parsed_cache)?;
    parsed_cache.retain_referenced_current_scan();
    parsed_cache.write(&cache_root)?;
    Ok(facts)
}

pub(super) fn collect_scan_facts_without_content_with_parsed_cache(
    path: &Path,
    config: &ScanConfig,
    parsed_cache: &mut ParsedFactsCache,
) -> io::Result<ScanFacts> {
    ensure_path_exists(path)?;

    let mut facts = ScanFacts {
        root_path: path.to_path_buf(),
        ..ScanFacts::default()
    };
    let mut languages: HashMap<String, usize> = HashMap::new();
    let roots = package_roots(&facts.root_path);

    if path.is_file() {
        facts.files_discovered = 1;
        collect_file_facts_with_cache(
            path,
            &mut facts,
            &mut languages,
            config,
            &roots,
            parsed_cache,
        )?;
    } else {
        collect_directory_facts_with_cache(
            path,
            &mut facts,
            &mut languages,
            config,
            &roots,
            parsed_cache,
        )?;
    }

    facts.languages = build_language_summary(languages);

    Ok(facts)
}

fn collect_directory_facts(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    config: &ScanConfig,
    roots: &[PackageRoot],
) -> io::Result<()> {
    let collected = collect_paths(path, config)?;
    let mut file_paths = collected.file_paths;
    file_paths.sort();

    facts.files_discovered = file_paths.len();
    facts.files_skipped_repopilotignore = collected.files_skipped_repopilotignore;
    facts.repopilotignore_path = collected.repopilotignore_path;
    facts.directories_count = collected.directories_count;

    apply_max_files_limit(&mut file_paths, facts, config);

    for entry_path in file_paths {
        collect_file_facts(&entry_path, facts, languages, config, roots)?;
    }

    Ok(())
}

fn collect_directory_facts_with_cache(
    path: &Path,
    facts: &mut ScanFacts,
    languages: &mut HashMap<String, usize>,
    config: &ScanConfig,
    roots: &[PackageRoot],
    parsed_cache: &mut ParsedFactsCache,
) -> io::Result<()> {
    let collected = collect_paths(path, config)?;
    let mut file_paths = collected.file_paths;
    file_paths.sort();

    facts.files_discovered = file_paths.len();
    facts.files_skipped_repopilotignore = collected.files_skipped_repopilotignore;
    facts.repopilotignore_path = collected.repopilotignore_path;
    facts.directories_count = collected.directories_count;

    apply_max_files_limit(&mut file_paths, facts, config);

    for entry_path in file_paths {
        collect_file_facts_with_cache(&entry_path, facts, languages, config, roots, parsed_cache)?;
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

#[cfg(test)]
fn parsed_cache_root(path: &Path) -> PathBuf {
    if path.is_file() {
        return path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn collect_without_content_reuses_cached_parsed_facts_on_warm_run() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src");
        fs::create_dir_all(&src).expect("create src");
        fs::write(
            src.join("lib.rs"),
            "use crate::api::run;\nmod api;\npub fn main() { run(); }\n",
        )
        .expect("write source");
        fs::write(src.join("api.rs"), "pub fn run() {}\n").expect("write api");
        let original_api_hash = crate::scan::parsed_cache::content_hash("pub fn run() {}\n");

        let mut cold_cache = ParsedFactsCache::load(temp.path());
        let first = collect_scan_facts_without_content_with_parsed_cache(
            temp.path(),
            &ScanConfig::default(),
            &mut cold_cache,
        )
        .expect("cold facts collection");
        let cold_telemetry = cold_cache.telemetry();

        assert_eq!(first.files_analyzed, 2);
        assert_eq!(cold_telemetry.hits, 0);
        assert_eq!(cold_telemetry.misses, 2);
        cold_cache.retain_referenced_current_scan();
        cold_cache.write(temp.path()).expect("write cold cache");

        let mut warm_cache = ParsedFactsCache::load(temp.path());
        let second = collect_scan_facts_without_content_with_parsed_cache(
            temp.path(),
            &ScanConfig::default(),
            &mut warm_cache,
        )
        .expect("warm facts collection");
        let warm_telemetry = warm_cache.telemetry();

        assert_eq!(second.files_analyzed, 2);
        assert_eq!(warm_telemetry.entries_loaded, 2);
        assert_eq!(warm_telemetry.hits, 2, "warm run should reuse parsed facts");
        assert_eq!(
            warm_telemetry.misses, 0,
            "warm run should not reparse files"
        );
        warm_cache.retain_referenced_current_scan();
        warm_cache.write(temp.path()).expect("write warm cache");

        fs::write(src.join("api.rs"), "pub fn run() { if true { return; } }\n")
            .expect("modify api");

        let mut changed_cache = ParsedFactsCache::load(temp.path());
        let changed = collect_scan_facts_without_content_with_parsed_cache(
            temp.path(),
            &ScanConfig::default(),
            &mut changed_cache,
        )
        .expect("changed facts collection");
        let changed_telemetry = changed_cache.telemetry();

        assert_eq!(changed.files_analyzed, 2);
        assert_eq!(changed_telemetry.hits, 1);
        assert_eq!(changed_telemetry.misses, 1);
        changed_cache.retain_referenced_current_scan();
        changed_cache
            .write(temp.path())
            .expect("write changed cache");
        assert!(!parsed_cache_hashes(temp.path()).contains(&original_api_hash));

        fs::write(src.join("extra.rs"), "pub fn extra() {}\n").expect("add extra");
        let extra_hash = crate::scan::parsed_cache::content_hash("pub fn extra() {}\n");

        let mut expanded_cache = ParsedFactsCache::load(temp.path());
        let expanded = collect_scan_facts_without_content_with_parsed_cache(
            temp.path(),
            &ScanConfig::default(),
            &mut expanded_cache,
        )
        .expect("expanded facts collection");
        let expanded_telemetry = expanded_cache.telemetry();

        assert_eq!(expanded.files_analyzed, 3);
        assert_eq!(expanded_telemetry.hits, 2);
        assert_eq!(expanded_telemetry.misses, 1);
        expanded_cache.retain_referenced_current_scan();
        expanded_cache
            .write(temp.path())
            .expect("write expanded cache");
        assert!(parsed_cache_hashes(temp.path()).contains(&extra_hash));
    }

    #[test]
    fn single_file_without_content_uses_parent_as_cache_root() {
        let temp = tempdir().expect("temp dir");
        let file = temp.path().join("single.rs");
        fs::write(&file, "pub fn single() {}\n").expect("write single file");

        let facts = collect_scan_facts_without_content(&file, &ScanConfig::default())
            .expect("single-file facts collection");

        assert_eq!(facts.files_analyzed, 1);
        assert!(
            temp.path()
                .join(".repopilot/cache/parsed_facts_v2.json")
                .is_file()
        );
        assert!(!file.join(".repopilot/cache/parsed_facts_v2.json").exists());
    }

    fn parsed_cache_hashes(root: &Path) -> Vec<String> {
        let path = root.join(".repopilot/cache/parsed_facts_v2.json");
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).expect("read parsed cache"))
                .expect("parse parsed cache");
        value["entries"]
            .as_array()
            .expect("entries should be array")
            .iter()
            .filter_map(|entry| entry["content_hash"].as_str().map(str::to_string))
            .collect()
    }
}
