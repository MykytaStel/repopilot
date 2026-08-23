use repopilot::config::loader::{discover_config_path, load_optional_config, parse_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::output::OutputFormat;
use repopilot::verification::select_checks;
use std::fs;
use tempfile::tempdir;

const CONFIG_FILE_NAME: &str = "repopilot.toml";

#[test]
fn verification_checks_parse_with_bounded_defaults() {
    let config = parse_config(
        r#"
        [[verification.checks]]
        id = "unit"
        role = "test"
        program = "cargo"
        args = ["test", "--all"]
        paths = ["src/**", "tests/**"]
        "#,
        None,
    )
    .expect("verification config should parse");

    let check = &config.verification.checks[0];
    assert_eq!(check.id, "unit");
    assert_eq!(check.program, std::path::PathBuf::from("cargo"));
    assert_eq!(check.args, ["test", "--all"]);
    assert_eq!(check.working_directory, std::path::PathBuf::from("."));
    assert_eq!(check.timeout_seconds, 300);
    assert_eq!(check.max_output_bytes, 65_536);
    assert_eq!(check.paths, ["src/**", "tests/**"]);
}

#[test]
fn verification_cache_is_disabled_by_default_and_requires_explicit_opt_in() {
    let default = parse_config(
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\n",
        None,
    )
    .expect("default check");
    assert!(!default.verification.checks[0].cache.enabled);

    let enabled = parse_config(
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\n[verification.checks.cache]\nenabled = true\n",
        None,
    )
    .expect("cached check");
    assert!(enabled.verification.checks[0].cache.enabled);
}

#[test]
fn verification_cache_rejects_unknown_fields() {
    let error = parse_config(
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\n[verification.checks.cache]\nunknown = true\n",
        None,
    )
    .expect_err("unknown cache policy fields must fail closed");

    assert!(error.to_string().contains("unknown field `unknown`"));
}

#[test]
fn verification_check_rejects_unknown_fields() {
    let error = parse_config(
        r#"
        [[verification.checks]]
        id = "unit"
        role = "test"
        program = "cargo"
        shell = true
        "#,
        None,
    )
    .expect_err("unknown verification fields must fail closed");

    assert!(error.to_string().contains("unknown field `shell`"));
}

#[test]
fn verification_check_rejects_unsupported_role() {
    let error = parse_config(
        r#"
        [[verification.checks]]
        id = "unit"
        role = "deploy"
        program = "cargo"
        "#,
        None,
    )
    .expect_err("unsupported verification roles must fail closed");

    assert!(error.to_string().contains("unknown variant `deploy`"));
}

#[test]
fn verification_selection_is_deduplicated_and_sorted() {
    let temp = tempdir().expect("temp dir");
    let config = parse_config(
        r#"
        [[verification.checks]]
        id = "unit"
        role = "test"
        program = "cargo"

        [[verification.checks]]
        id = "lint"
        role = "lint"
        program = "cargo"
        "#,
        None,
    )
    .expect("valid config");

    let selected = select_checks(
        temp.path(),
        &config.verification.checks,
        &["unit".into(), "lint".into(), "unit".into()],
    )
    .expect("selection should be valid");

    assert_eq!(
        selected.iter().map(|check| check.id()).collect::<Vec<_>>(),
        ["lint", "unit"]
    );
}

#[test]
fn verification_selection_rejects_invalid_policy_before_execution() {
    let temp = tempdir().expect("temp dir");
    for (body, expected) in [
        (
            "id = \"Upper\"\nrole = \"test\"\nprogram = \"cargo\"",
            "invalid verification check id `Upper`",
        ),
        (
            "id = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\ntimeout_seconds = 0",
            "timeout_seconds must be between 1 and 1800",
        ),
        (
            "id = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\nmax_output_bytes = 1048577",
            "max_output_bytes must be between 1 and 1048576",
        ),
    ] {
        let config = parse_config(&format!("[[verification.checks]]\n{body}\n"), None)
            .expect("syntax should parse before semantic validation");
        let error = select_checks(temp.path(), &config.verification.checks, &["unit".into()])
            .expect_err("invalid verification policy must fail closed");
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn verification_selection_rejects_duplicate_ids_and_unknown_selection() {
    let temp = tempdir().expect("temp dir");
    let duplicate = parse_config(
        r#"
        [[verification.checks]]
        id = "unit"
        role = "test"
        program = "cargo"
        [[verification.checks]]
        id = "unit"
        role = "lint"
        program = "cargo"
        "#,
        None,
    )
    .expect("valid TOML");
    let error = select_checks(
        temp.path(),
        &duplicate.verification.checks,
        &["unit".into()],
    )
    .expect_err("duplicate IDs must fail closed");
    assert!(
        error
            .to_string()
            .contains("duplicate verification check id `unit`")
    );

    let valid = parse_config(
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\n",
        None,
    )
    .expect("valid TOML");
    let error = select_checks(temp.path(), &valid.verification.checks, &["missing".into()])
        .expect_err("unknown selected IDs must fail closed");
    assert!(
        error
            .to_string()
            .contains("unknown verification check id `missing`")
    );
}

#[test]
fn verification_paths_match_changed_or_impacted_files() {
    let temp = tempdir().expect("temp dir");
    let config = parse_config(
        r#"
        [[verification.checks]]
        id = "unit"
        role = "test"
        program = "cargo"
        paths = ["src/**", "Cargo.toml"]
        "#,
        None,
    )
    .expect("valid config");
    let checks = select_checks(temp.path(), &config.verification.checks, &["unit".into()])
        .expect("valid selection");

    assert!(checks[0].is_applicable([std::path::Path::new("src/deleted.rs")]));
    assert!(checks[0].is_applicable([std::path::Path::new("Cargo.toml")]));
    assert!(!checks[0].is_applicable([std::path::Path::new("docs/guide.md")]));
}

#[test]
fn verification_without_paths_is_always_applicable() {
    let temp = tempdir().expect("temp dir");
    let config = parse_config(
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\n",
        None,
    )
    .expect("valid config");
    let checks = select_checks(temp.path(), &config.verification.checks, &["unit".into()])
        .expect("valid selection");

    assert!(checks[0].is_applicable(std::iter::empty::<&std::path::Path>()));
}

#[test]
fn verification_paths_reject_repository_escape() {
    let temp = tempdir().expect("temp dir");
    let outside = tempdir().expect("outside dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), temp.path().join("escape")).expect("create symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(outside.path(), temp.path().join("escape"))
        .expect("create symlink");

    let config = parse_config(
        "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"cargo\"\nworking_directory = \"escape\"\n",
        None,
    )
    .expect("valid TOML");
    let error = select_checks(temp.path(), &config.verification.checks, &["unit".into()])
        .expect_err("symlink escape must fail closed");

    assert!(error.to_string().contains("inside the repository"));
}

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
