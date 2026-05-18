use repopilot::baseline::diff::{BaselineStatus, FindingBaselineStatus};
use repopilot::baseline::key::stable_finding_key;
use repopilot::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use repopilot::graph::CouplingGraph;
use repopilot::risk::{
    FORMULA_VERSION, RiskInputs, RiskPriority, apply_baseline_overlay, apply_blast_radius_overlay,
    apply_cluster_overlay, apply_graph_overlay, apply_review_overlay,
    apply_workspace_hotspot_overlay, assess_finding,
};
use repopilot::scan::facts::FileFacts;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[test]
fn file_role_context_changes_risk_without_changing_rule_severity() {
    let finding = finding(
        "code-quality.complex-file",
        "src/domain/user.rs",
        Severity::Medium,
    );
    let production = file("src/domain/user.rs", Some("Rust"));
    let generated = file("src/generated/user.rs", Some("Rust"));
    let config = file("Cargo.toml", Some("TOML"));

    let production_risk = assess_finding(&finding, Some(&production), RiskInputs::default());
    let generated_risk = assess_finding(&finding, Some(&generated), RiskInputs::default());
    let config_risk = assess_finding(&finding, Some(&config), RiskInputs::default());

    assert!(production_risk.score > generated_risk.score);
    assert!(production_risk.score > config_risk.score);
    assert_eq!(finding.severity, Severity::Medium);
    assert!(
        generated_risk
            .signals
            .iter()
            .any(|signal| signal.id == "role.generated")
    );
    assert!(
        config_risk
            .signals
            .iter()
            .any(|signal| signal.id == "role.config")
    );
}

#[test]
fn baseline_overlay_boosts_new_findings_and_downweights_existing_findings() {
    let root = Path::new("/repo");
    let mut findings = vec![
        assessed_finding("security.secret-candidate", "src/new.rs", Severity::High),
        assessed_finding(
            "security.secret-candidate",
            "src/existing.rs",
            Severity::High,
        ),
    ];
    let statuses = vec![
        FindingBaselineStatus {
            key: stable_finding_key(&findings[0], root),
            status: BaselineStatus::New,
        },
        FindingBaselineStatus {
            key: stable_finding_key(&findings[1], root),
            status: BaselineStatus::Existing,
        },
    ];

    apply_baseline_overlay(&mut findings, &statuses, root);

    assert!(findings[0].risk.score > findings[1].risk.score);
    assert!(
        findings[0]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "baseline.new")
    );
    assert!(
        findings[1]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "baseline.existing")
    );
}

#[test]
fn review_overlay_boosts_in_diff_findings() {
    let mut findings = vec![
        assessed_finding(
            "architecture.large-file",
            "src/changed.rs",
            Severity::Medium,
        ),
        assessed_finding(
            "architecture.large-file",
            "src/unchanged.rs",
            Severity::Medium,
        ),
    ];

    apply_review_overlay(&mut findings, &[true, false]);

    assert!(findings[0].risk.score > findings[1].risk.score);
    assert!(
        findings[0]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "review.in-diff")
    );
}

#[test]
fn workspace_hotspot_overlay_adds_stable_learning_signal() {
    let mut findings = vec![
        assessed_finding(
            "security.secret-candidate",
            "packages/web/a.rs",
            Severity::High,
        ),
        assessed_finding(
            "security.secret-candidate",
            "packages/web/b.rs",
            Severity::High,
        ),
        assessed_finding(
            "security.secret-candidate",
            "packages/api/a.rs",
            Severity::High,
        ),
    ];
    findings[0].workspace_package = Some("web".to_string());
    findings[1].workspace_package = Some("web".to_string());
    findings[2].workspace_package = Some("api".to_string());

    apply_workspace_hotspot_overlay(&mut findings);

    assert!(
        findings[0]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "workspace.hotspot")
    );
    assert!(
        findings[1]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "workspace.hotspot")
    );
    assert!(
        findings[2]
            .risk
            .signals
            .iter()
            .all(|signal| signal.id != "workspace.hotspot")
    );
}

#[test]
fn priority_labels_follow_score_thresholds() {
    let critical = assessed_finding(
        "security.private-key-candidate",
        "src/key.rs",
        Severity::Critical,
    );
    let low = assessed_finding("code-marker.todo", "src/lib.rs", Severity::Low);

    assert_eq!(critical.risk.priority, RiskPriority::P0);
    assert_eq!(low.risk.priority, RiskPriority::P3);
    assert_eq!(critical.risk.formula_version, FORMULA_VERSION);
}

#[test]
fn graph_overlay_boosts_hub_findings_above_leaf_findings() {
    let mut findings = vec![
        assessed_finding("code-quality.complex-file", "src/core.rs", Severity::Medium),
        assessed_finding("code-quality.complex-file", "src/leaf.rs", Severity::Medium),
    ];
    let graph = graph(&[
        ("src/a.rs", "src/core.rs"),
        ("src/b.rs", "src/core.rs"),
        ("src/c.rs", "src/core.rs"),
        ("src/core.rs", "src/leaf.rs"),
    ]);

    apply_graph_overlay(&mut findings, &graph);

    assert!(findings[0].risk.score > findings[1].risk.score);
    assert!(
        findings[0]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "graph.hub")
    );
}

#[test]
fn blast_radius_overlay_boosts_impacted_files() {
    let mut findings = vec![
        assessed_finding(
            "architecture.large-file",
            "src/impacted.rs",
            Severity::Medium,
        ),
        assessed_finding("architecture.large-file", "src/other.rs", Severity::Medium),
    ];

    apply_blast_radius_overlay(
        &mut findings,
        Path::new("/repo"),
        &[PathBuf::from("src/impacted.rs")],
    );

    assert!(findings[0].risk.score > findings[1].risk.score);
    assert!(
        findings[0]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "review.blast-radius")
    );
}

#[test]
fn cluster_overlay_marks_repeated_rule_scope_patterns() {
    let mut findings = vec![
        assessed_finding(
            "language.rust.panic-risk",
            "src/output/a.rs",
            Severity::Medium,
        ),
        assessed_finding(
            "language.rust.panic-risk",
            "src/output/b.rs",
            Severity::Medium,
        ),
        assessed_finding(
            "language.rust.panic-risk",
            "src/output/c.rs",
            Severity::Medium,
        ),
        assessed_finding(
            "language.rust.panic-risk",
            "src/core/d.rs",
            Severity::Medium,
        ),
    ];

    apply_cluster_overlay(&mut findings);

    assert!(
        findings[0]
            .risk
            .signals
            .iter()
            .any(|signal| signal.id == "cluster.repeated")
    );
    assert!(
        findings[3]
            .risk
            .signals
            .iter()
            .all(|signal| signal.id != "cluster.repeated")
    );
}

#[test]
fn knowledge_rule_adjustment_participates_in_risk_scoring() {
    let finding = finding("code-marker.todo", "src/lib.rs", Severity::Low);
    let file = file("src/lib.rs", Some("Rust"));

    let risk = assess_finding(&finding, Some(&file), RiskInputs::default());

    assert!(
        risk.signals
            .iter()
            .any(|signal| signal.id == "knowledge.todo-backlog")
    );
}

fn assessed_finding(rule_id: &str, path: &str, severity: Severity) -> Finding {
    let mut finding = finding(rule_id, path, severity);
    finding.risk = assess_finding(&finding, None, RiskInputs::default());
    finding
}

fn finding(rule_id: &str, path: &str, severity: Severity) -> Finding {
    Finding {
        id: String::new(),
        rule_id: rule_id.to_string(),
        title: "test finding".to_string(),
        description: "test finding".to_string(),
        recommendation: Finding::recommendation_for_rule_id(rule_id),
        category: if rule_id.starts_with("security.") {
            FindingCategory::Security
        } else {
            FindingCategory::CodeQuality
        },
        severity,
        confidence: Confidence::Medium,
        evidence: vec![Evidence {
            path: PathBuf::from(path),
            line_start: 1,
            line_end: None,
            snippet: String::new(),
        }],
        workspace_package: None,
        docs_url: None,
        risk: Default::default(),
    }
}

fn file(path: &str, language: Option<&str>) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: language.map(str::to_string),
        non_empty_lines: 10,
        branch_count: 0,
        imports: Vec::new(),
        content: None,
        has_inline_tests: false,
    }
}

fn graph(edges: &[(&str, &str)]) -> CouplingGraph {
    let mut edge_map: BTreeMap<PathBuf, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut nodes = BTreeSet::new();

    for (source, target) in edges {
        let source = PathBuf::from(source);
        let target = PathBuf::from(target);
        nodes.insert(source.clone());
        nodes.insert(target.clone());
        edge_map.entry(source).or_default().insert(target);
    }

    CouplingGraph {
        edges: edge_map,
        nodes,
    }
}
