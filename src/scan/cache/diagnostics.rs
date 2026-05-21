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
