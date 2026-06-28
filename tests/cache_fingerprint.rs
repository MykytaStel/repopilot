use repopilot::scan::cache::{CACHE_SCHEMA_VERSION, config_fingerprint, stable_hash_hex};
use repopilot::scan::config::ScanConfig;

#[test]
fn stable_hash_hex_uses_sha256() {
    assert_eq!(
        stable_hash_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn config_fingerprint_changes_with_config_and_cache_schema_context() {
    assert_eq!(CACHE_SCHEMA_VERSION, 7);

    let default = config_fingerprint(&ScanConfig::default());
    let changed = config_fingerprint(&ScanConfig {
        max_file_bytes: 123,
        ..ScanConfig::default()
    });

    assert_eq!(default.len(), 64);
    assert_eq!(changed.len(), 64);
    assert_ne!(default, changed);
}
