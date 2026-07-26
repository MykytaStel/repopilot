use repopilot::review::diff::{ChangeStatus, ChangedFile};
use repopilot::review::{FileImpact, ImpactPaths, OwnershipIndex, OwnershipSummary};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn codeowners_last_matching_rule_wins_and_deduplicates_owners() {
    let index = OwnershipIndex::from_codeowners(
        "* @all\n/src/ @backend\n/src/auth/ @security @backend @security\n",
        PathBuf::from("CODEOWNERS"),
    )
    .unwrap();

    assert_eq!(
        values(index.owners_for(Path::new("src/auth/session.rs"))),
        vec!["@security", "@backend"]
    );
    assert_eq!(
        values(index.owners_for(Path::new("src/lib.rs"))),
        vec!["@backend"]
    );
}

#[test]
fn discovery_prefers_dot_github_then_root_then_docs() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join(".github")).unwrap();
    fs::create_dir_all(root.path().join("docs")).unwrap();
    fs::write(root.path().join("CODEOWNERS"), "* @root\n").unwrap();
    fs::write(root.path().join("docs/CODEOWNERS"), "* @docs\n").unwrap();
    fs::write(root.path().join(".github/CODEOWNERS"), "* @github\n").unwrap();

    let discovery = OwnershipIndex::discover(root.path());
    assert!(discovery.diagnostics.is_empty());
    assert_eq!(
        discovery.index.source(),
        Some(Path::new(".github/CODEOWNERS"))
    );
    assert_eq!(
        values(discovery.index.owners_for(Path::new("src/lib.rs"))),
        vec!["@github"]
    );
}

#[test]
fn missing_codeowners_reports_stable_boundaries_without_inventing_people() {
    let index = OwnershipIndex::empty();
    let summary = OwnershipSummary::for_paths(
        [
            PathBuf::from("src/auth/session.rs"),
            PathBuf::from("src/api.rs"),
            PathBuf::from("README.md"),
        ],
        &index,
    );

    assert!(summary.suggested_owners.is_empty());
    assert_eq!(summary.unowned_paths.len(), 3);
    assert_eq!(summary.fallback_boundaries, vec![".", "src"]);
}

#[test]
fn ownership_summary_covers_changed_and_transitively_impacted_paths() {
    let index = OwnershipIndex::from_codeowners(
        "/src/auth/ @security\n/src/api/ @platform\n",
        PathBuf::from("CODEOWNERS"),
    )
    .unwrap();
    let changed = vec![ChangedFile {
        path: PathBuf::from("src/auth/session.rs"),
        status: ChangeStatus::Modified,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }];
    let impact = ImpactPaths {
        files: vec![FileImpact {
            path: PathBuf::from("src/auth/session.rs"),
            direct_dependents: vec![PathBuf::from("src/api/routes.rs")],
            transitive_dependents: vec![PathBuf::from("web/app.ts")],
        }],
        ..ImpactPaths::default()
    };

    let summary = OwnershipSummary::for_impact(&changed, &impact, &index);
    assert_eq!(
        values(summary.suggested_owners),
        vec!["@platform", "@security"]
    );
    assert_eq!(summary.unowned_paths, vec!["web/app.ts"]);
}

fn values(owners: Vec<repopilot::review::Owner>) -> Vec<String> {
    owners.into_iter().map(|owner| owner.value).collect()
}
