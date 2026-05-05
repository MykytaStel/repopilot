use repopilot::config::loader::{load_optional_config, parse_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::output::OutputFormat;
use std::fs;
use tempfile::tempdir;

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
    assert_eq!(config.architecture.max_file_lines, 42);
    assert_eq!(config.architecture.huge_file_lines, 1000);
    assert_eq!(config.output.default_format, OutputFormat::Json);
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
