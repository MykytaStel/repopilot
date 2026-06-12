use repopilot::config::loader::{discover_config_path, load_optional_config, parse_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::output::OutputFormat;
use std::fs;
use tempfile::tempdir;

const CONFIG_FILE_NAME: &str = "repopilot.toml";

#[test]
fn missing_config_returns_defaults() {
    let temp = tempdir().expect("failed to create temp dir");
    let config = load_optional_config(&temp.path().join("missing.toml"))
        .expect("missing config should use defaults");

    assert_eq!(config, RepoPilotConfig::default());
}

#[test]
fn valid_config_is_parsed() {
    let config = parse_config(
        r#"
        [scan]
        ignore = ["vendor"]

        [architecture]
        max_file_lines = 42

        [output]
        default_format = "json"
        "#,
        None,
    )
    .expect("valid config should parse");

    assert_eq!(config.scan.ignore, vec!["vendor"]);
    assert_eq!(config.scan.max_file_bytes, 2 * 1024 * 1024);
    assert_eq!(config.architecture.max_file_lines, 42);
    assert_eq!(config.architecture.huge_file_lines, 1000);
    assert_eq!(config.output.default_format, OutputFormat::Json);
}

#[test]
fn architecture_coupling_thresholds_are_parsed() {
    let config = parse_config(
        r#"
        [architecture]
        max_fan_out = 9
        instability_hub_min_fan_in = 3
        instability_hub_min_instability_pct = 60
        "#,
        None,
    )
    .expect("valid config should parse");
    let scan_config = config.to_scan_config();

    assert_eq!(scan_config.max_fan_out, 9);
    assert_eq!(scan_config.instability_hub_min_fan_in, 3);
    assert_eq!(scan_config.instability_hub_min_instability_pct, 60);
}

#[test]
fn invalid_toml_returns_error() {
    let error = parse_config("[scan", None).expect_err("invalid TOML should fail");

    assert!(error.to_string().contains("invalid config"));
}

#[test]
fn explicit_config_path_is_loaded() {
    let temp = tempdir().expect("failed to create temp dir");
    let config_path = temp.path().join("custom.toml");
    fs::write(
        &config_path,
        r#"
        [scan]
        ignore = ["generated"]
        "#,
    )
    .expect("failed to write config");

    let config = load_optional_config(&config_path).expect("config should load");

    assert_eq!(config.scan.ignore, vec!["generated"]);
}

#[test]
fn scan_max_file_bytes_is_parsed() {
    let config = parse_config(
        r#"
        [scan]
        max_file_bytes = 12345
        "#,
        None,
    )
    .expect("valid config should parse");

    assert_eq!(config.scan.max_file_bytes, 12345);
    assert_eq!(config.to_scan_config().max_file_bytes, 12345);
}

#[test]
fn discover_finds_config_in_start_dir() {
    let temp = tempdir().expect("temp dir");
    let dir = temp.path();
    fs::write(dir.join(CONFIG_FILE_NAME), "[scan]\nignore = []\n").expect("write config");

    assert_eq!(
        discover_config_path(dir),
        Some(dir.join(CONFIG_FILE_NAME)),
        "config beside the start dir should be discovered"
    );
}

#[test]
fn discover_walks_up_to_an_ancestor_config() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    // `.git` bounds the upward walk so the test never escapes the temp tree.
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(root.join(CONFIG_FILE_NAME), "[scan]\nignore = []\n").expect("write config");
    let nested = root.join("packages/app/src");
    fs::create_dir_all(&nested).expect("nested dirs");

    assert_eq!(
        discover_config_path(&nested),
        Some(root.join(CONFIG_FILE_NAME)),
        "a config at the repo root should be found from a nested subdir"
    );
}

#[test]
fn discover_stops_at_git_root_and_does_not_escape_the_repo() {
    let temp = tempdir().expect("temp dir");
    let outer = temp.path();
    // Config sits *above* the git root and must stay invisible.
    fs::write(outer.join(CONFIG_FILE_NAME), "[scan]\nignore = []\n").expect("write outer config");
    let repo = outer.join("repo");
    fs::create_dir_all(repo.join(".git")).expect("git marker");
    let nested = repo.join("src");
    fs::create_dir_all(&nested).expect("nested dirs");

    assert_eq!(
        discover_config_path(&nested),
        None,
        "discovery must stop at the git root, ignoring configs outside the repo"
    );
}

#[test]
fn discover_returns_none_when_no_config_exists() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    fs::create_dir_all(root.join(".git")).expect("git marker");
    let nested = root.join("a/b");
    fs::create_dir_all(&nested).expect("nested dirs");

    assert_eq!(discover_config_path(&nested), None);
}

#[test]
fn taint_review_signals_can_be_disabled() {
    let config = parse_config(
        r#"
        [taint]
        enabled = false
        "#,
        None,
    )
    .expect("valid config should parse");

    assert!(!config.taint.enabled);
}
