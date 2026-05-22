use std::fs;
use std::path::{Path, PathBuf};

// This is a repository safety test, not a secret detector test.
//
// Rule fixtures may intentionally contain secret-like variable names so RepoPilot
// can test its detectors. They must not contain provider-looking fake secrets
// because GitHub Push Protection can block the branch before CI runs.

const FIXTURE_ROOT: &str = "tests/fixtures/rules";

const FORBIDDEN_PROVIDER_SECRET_MARKERS: &[&str] = &[
    "sk_live_",
    "sk_test_",
    "rk_live_",
    "rk_test_",
    "pk_live_",
    "pk_test_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "github_pat_",
    "xoxb-",
    "xoxp-",
    "ya29.",
    "AKIA",
    "ASIA",
    "AIza",
];

#[test]
fn given_rule_fixtures_when_scanning_repository_tests_then_no_provider_looking_fake_secrets_exist()
{
    // Given
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT);
    let fixture_files = collect_files(&fixture_root);

    // When / Then
    for file in fixture_files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };

        for marker in FORBIDDEN_PROVIDER_SECRET_MARKERS {
            assert!(
                !content.contains(marker),
                "{} contains provider-looking fake secret marker `{}`. Use fixture-only values instead.",
                file.display(),
                marker
            );
        }
    }
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_into(root, &mut files);
    files
}

fn collect_files_into(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    if metadata.is_file() {
        files.push(path.to_path_buf());
        return;
    }

    if !metadata.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_files_into(&entry.path(), files);
    }
}
