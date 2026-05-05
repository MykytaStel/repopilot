use repopilot::scan::config::ScanConfig;

#[test]
fn default_scan_config_uses_expected_thresholds() {
    let config = ScanConfig::default();

    assert_eq!(config.large_file_loc_threshold, 300);
    assert_eq!(config.huge_file_loc_threshold, 1000);
}

#[test]
fn custom_large_file_threshold_updates_config() {
    let config = ScanConfig::default().with_large_file_loc_threshold(500);

    assert_eq!(config.large_file_loc_threshold, 500);
    assert_eq!(config.huge_file_loc_threshold, 1000);
}

#[test]
fn custom_threshold_keeps_huge_threshold_above_large_threshold() {
    let config = ScanConfig::default().with_large_file_loc_threshold(1200);

    assert_eq!(config.large_file_loc_threshold, 1200);
    assert!(config.huge_file_loc_threshold > config.large_file_loc_threshold);
}
