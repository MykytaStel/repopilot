use super::classify::classify_boundary;
use super::composites::{any_test_changed, enrich_blast_radius, missing_test_for_code_boundary};
use super::{BoundaryCategory, BoundarySignal, detect_boundary_signals};
use crate::config::model::SecurityBoundarySection;
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn changed(path: &str, status: ChangeStatus) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }
}

fn classify(path: &str) -> Option<BoundaryCategory> {
    classify_boundary(path, None)
}

fn signal(category: BoundaryCategory, path: &str) -> BoundarySignal {
    BoundarySignal {
        category,
        path: path.to_string(),
        status: ChangeStatus::Modified,
        blast_radius: 0,
    }
}

#[test]
fn access_control_matches_auth_paths_and_filenames() {
    assert_eq!(
        classify("src/middleware/auth.ts"),
        Some(BoundaryCategory::AccessControl)
    );
    assert_eq!(
        classify("src/authMiddleware.ts"),
        Some(BoundaryCategory::AccessControl)
    );
    assert_eq!(
        classify("internal/session/store.go"),
        Some(BoundaryCategory::AccessControl)
    );
    assert_eq!(
        classify("src/auth/jwt_verify.rs"),
        Some(BoundaryCategory::AccessControl)
    );
    assert_eq!(
        classify("app/Http/Middleware/Authenticate.php"),
        Some(BoundaryCategory::AccessControl)
    );
}

#[test]
fn access_control_does_not_match_lookalike_tokens() {
    // `authors`, design `tokens`, and privacy `policy` are common non-boundary files.
    assert_eq!(classify("src/components/authors.tsx"), None);
    assert_eq!(classify("src/theme/tokens.ts"), None);
    assert_eq!(classify("docs/privacy-policy.md"), None);
    assert_eq!(classify("src/utils/format.ts"), None);
}

#[test]
fn request_trust_matches_cors_and_helmet() {
    assert_eq!(
        classify("src/server/cors.ts"),
        Some(BoundaryCategory::RequestTrust)
    );
    assert_eq!(
        classify("config/helmet.js"),
        Some(BoundaryCategory::RequestTrust)
    );
    assert_eq!(
        classify("src/corsConfig.ts"),
        Some(BoundaryCategory::RequestTrust)
    );
}

#[test]
fn deploy_surface_matches_ci_docker_and_iac() {
    assert_eq!(
        classify(".github/workflows/deploy.yml"),
        Some(BoundaryCategory::DeploySurface)
    );
    assert_eq!(
        classify("Dockerfile"),
        Some(BoundaryCategory::DeploySurface)
    );
    assert_eq!(
        classify("Dockerfile.prod"),
        Some(BoundaryCategory::DeploySurface)
    );
    assert_eq!(
        classify("infra/main.tf"),
        Some(BoundaryCategory::DeploySurface)
    );
    assert_eq!(
        classify(".gitlab-ci.yml"),
        Some(BoundaryCategory::DeploySurface)
    );
}

#[test]
fn deploy_surface_wins_over_access_control_for_workflow_named_auth() {
    assert_eq!(
        classify(".github/workflows/auth.yml"),
        Some(BoundaryCategory::DeploySurface)
    );
}

#[test]
fn supply_chain_matches_manifests_and_lockfiles() {
    assert_eq!(
        classify("package.json"),
        Some(BoundaryCategory::SupplyChain)
    );
    assert_eq!(
        classify("frontend/package-lock.json"),
        Some(BoundaryCategory::SupplyChain)
    );
    assert_eq!(classify("Cargo.toml"), Some(BoundaryCategory::SupplyChain));
    assert_eq!(classify("go.sum"), Some(BoundaryCategory::SupplyChain));
}

#[test]
fn secret_config_matches_env_files() {
    assert_eq!(classify(".env"), Some(BoundaryCategory::SecretConfig));
    assert_eq!(
        classify(".env.production"),
        Some(BoundaryCategory::SecretConfig)
    );
    assert_eq!(classify("src/config/settings.ts"), None);
}

#[test]
fn disabled_config_yields_no_signals() {
    let config = SecurityBoundarySection {
        enabled: false,
        extra_patterns: Vec::new(),
    };
    let files = vec![changed("src/auth/login.ts", ChangeStatus::Modified)];
    assert!(detect_boundary_signals(&files, &config).is_empty());
}

#[test]
fn extra_patterns_flag_custom_boundaries() {
    let config = SecurityBoundarySection {
        enabled: true,
        extra_patterns: vec!["**/secrets/**".to_string()],
    };
    let files = vec![
        changed("ops/secrets/keys.yaml", ChangeStatus::Added),
        changed("src/utils/format.ts", ChangeStatus::Modified),
    ];
    let signals = detect_boundary_signals(&files, &config);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].category, BoundaryCategory::Custom);
    assert_eq!(signals[0].path, "ops/secrets/keys.yaml");
}

#[test]
fn detect_skips_test_files_even_when_path_looks_like_a_boundary() {
    let config = SecurityBoundarySection::default();
    let files = vec![
        changed("src/auth/login.ts", ChangeStatus::Modified),
        changed("src/auth/login.test.ts", ChangeStatus::Modified),
        changed("tests/auth_service_test.py", ChangeStatus::Modified),
        changed("src/__tests__/cors.spec.ts", ChangeStatus::Modified),
    ];
    let signals = detect_boundary_signals(&files, &config);
    // Only the production auth file is a boundary; the three test files are not.
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].path, "src/auth/login.ts");
    assert_eq!(signals[0].category, BoundaryCategory::AccessControl);
}

#[test]
fn detect_sorts_by_category_then_path() {
    let config = SecurityBoundarySection::default();
    let files = vec![
        changed("package.json", ChangeStatus::Modified),
        changed("src/auth/login.ts", ChangeStatus::Modified),
        changed(".github/workflows/ci.yml", ChangeStatus::Modified),
    ];
    let signals = detect_boundary_signals(&files, &config);
    let categories: Vec<_> = signals.iter().map(|signal| signal.category).collect();
    assert_eq!(
        categories,
        vec![
            BoundaryCategory::AccessControl,
            BoundaryCategory::DeploySurface,
            BoundaryCategory::SupplyChain,
        ]
    );
}

#[test]
fn any_test_changed_detects_test_files_in_diff() {
    let with_test = vec![
        changed("src/auth/login.ts", ChangeStatus::Modified),
        changed("src/auth/login.test.ts", ChangeStatus::Modified),
    ];
    let without_test = vec![changed("src/auth/login.ts", ChangeStatus::Modified)];
    assert!(any_test_changed(&with_test));
    assert!(!any_test_changed(&without_test));
}

#[test]
fn missing_test_fires_only_for_code_boundaries_without_a_test() {
    let code_signal = vec![signal(BoundaryCategory::AccessControl, "src/auth/login.ts")];
    let config_signal = vec![signal(BoundaryCategory::SupplyChain, "package.json")];

    // Code boundary changed, no test moved → fire.
    let no_test = vec![changed("src/auth/login.ts", ChangeStatus::Modified)];
    assert!(missing_test_for_code_boundary(&code_signal, &no_test));

    // Code boundary changed but a test moved too → silent.
    let with_test = vec![
        changed("src/auth/login.ts", ChangeStatus::Modified),
        changed("tests/auth.test.ts", ChangeStatus::Modified),
    ];
    assert!(!missing_test_for_code_boundary(&code_signal, &with_test));

    // Only a config/supply-chain boundary changed → not a code boundary, silent.
    assert!(!missing_test_for_code_boundary(&config_signal, &no_test));
}

#[test]
fn enrich_blast_radius_counts_importers_from_graph() {
    let repo_root = Path::new("/repo");
    let mut edges: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    edges.insert(
        PathBuf::from("src/app.ts"),
        BTreeSet::from([PathBuf::from("src/auth/session.ts")]),
    );
    edges.insert(
        PathBuf::from("src/admin.ts"),
        BTreeSet::from([PathBuf::from("src/auth/session.ts")]),
    );
    let graph = CouplingGraph {
        edges,
        nodes: BTreeSet::new(),
    };

    let mut signals = vec![
        signal(BoundaryCategory::AccessControl, "src/auth/session.ts"),
        signal(BoundaryCategory::SupplyChain, "package.json"),
    ];
    enrich_blast_radius(&mut signals, Some(&graph), repo_root);

    assert_eq!(signals[0].blast_radius, 2);
    assert_eq!(signals[1].blast_radius, 0);
}

#[test]
fn enrich_blast_radius_is_noop_without_graph() {
    let mut signals = vec![signal(
        BoundaryCategory::AccessControl,
        "src/auth/session.ts",
    )];
    enrich_blast_radius(&mut signals, None, Path::new("/repo"));
    assert_eq!(signals[0].blast_radius, 0);
}
