use repopilot::config::loader::load_optional_config;
use repopilot::verification::VerificationRole;
use std::path::Path;

#[test]
fn repository_checks_are_explicit_offline_and_scoped() {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("repopilot.toml");
    let config = load_optional_config(&config_path).expect("repository config");
    let checks = config.verification.checks;

    let ids = checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["rust-format", "rust-clippy", "rust-tests"]);

    let expected = [
        (
            VerificationRole::Lint,
            600,
            ["--offline", "--locked", "fmt", "--all", "--", "--check"].as_slice(),
        ),
        (
            VerificationRole::Lint,
            600,
            [
                "--offline",
                "--locked",
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ]
            .as_slice(),
        ),
        (
            VerificationRole::Test,
            600,
            ["--offline", "--locked", "test", "--all"].as_slice(),
        ),
    ];

    for (check, (role, timeout_seconds, args)) in checks.iter().zip(expected) {
        assert_eq!(check.role, role, "{}", check.id);
        assert_eq!(check.program, Path::new("cargo"), "{}", check.id);
        assert_eq!(check.args, args, "{}", check.id);
        assert_eq!(check.working_directory, Path::new("."), "{}", check.id);
        assert_eq!(check.timeout_seconds, timeout_seconds, "{}", check.id);
        assert!(!check.cache.enabled, "{}", check.id);
        for path in [
            "src/**",
            "tests/**",
            "Cargo.toml",
            "Cargo.lock",
            "build.rs",
            "rust-toolchain.toml",
            "repopilot.toml",
        ] {
            assert!(
                check.paths.iter().any(|configured| configured == path),
                "{} misses {path}",
                check.id
            );
        }
    }
}
