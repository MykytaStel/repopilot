use repopilot::review::diff::{ChangeStatus, ChangedFile, ChangedRange, DiffHunk};
use repopilot::review::model::ReviewReport;
use repopilot::review::render::render_review_html;
use repopilot::scan::types::{ScanMetadata, ScanMetrics, ScanMode, ScanSummary};
use repopilot::verification::{VerificationOutcome, VerificationRole, VerificationStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn review_html_renders_proof_card_change_map_and_escaped_scope() {
    let report = ReviewReport {
        summary: ScanSummary {
            metadata: ScanMetadata {
                mode: ScanMode::Changed,
                root_path: PathBuf::from("<repo>"),
                ..Default::default()
            },
            metrics: ScanMetrics {
                files_discovered: 2,
                files_analyzed: 1,
                large_files_skipped: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        repo_root: PathBuf::from("<repo>"),
        baseline_path: None,
        changed_files: vec![ChangedFile {
            path: PathBuf::from("src/<auth>.rs"),
            status: ChangeStatus::Modified,
            ranges: vec![ChangedRange { start: 4, end: 6 }],
            hunks: vec![DiffHunk {
                header: Some("auth policy".to_string()),
                new_range: Some(ChangedRange { start: 4, end: 4 }),
                old_range: Some(ChangedRange { start: 4, end: 4 }),
                added_lines: vec!["after <change>".to_string()],
                removed_lines: vec!["before <change>".to_string()],
            }],
        }],
        blast_radius: vec![PathBuf::from("src/consumer.rs")],
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    };

    let html = render_review_html(&report, None, None);

    assert!(html.contains("RepoPilot Review Report"));
    assert!(html.contains("class=\"proof-card verdict-review\""));
    assert!(html.contains("Change proof"));
    assert!(html.contains("REVIEW"));
    assert!(html.contains("1/1 file(s) analyzed"));
    assert!(html.contains("none selected; no verification evidence"));
    assert!(html.contains("1 excluded, 0 unsupported file(s)"));
    assert!(html.contains("<h2>Change Map</h2>"));
    assert!(html.contains("Before"));
    assert!(html.contains("After"));
    assert!(html.contains("after &lt;change&gt;"));
    assert!(!html.contains("after <change>"));
    assert!(html.contains("src/&lt;auth&gt;.rs"));
    assert!(!html.contains("src/<auth>.rs"));
    assert!(html.contains("src/consumer.rs"));
}

#[test]
fn review_cli_writes_html_contract_consumer_map() {
    let temp = TempDir::new().expect("temp dir");
    init_repo(temp.path());
    write(
        temp.path(),
        "src/api.ts",
        "export const policy = () => true;\n",
    );
    write(
        temp.path(),
        "src/app.ts",
        "import { policy } from \"./api\";\nexport const app = () => policy();\n",
    );
    git(temp.path(), &["add", "."]);
    git(temp.path(), &["commit", "-m", "before"]);

    write(temp.path(), "src/api.ts", "const policy = () => true;\n");
    let output_path = temp.path().join("review.html");
    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .args([
            "review",
            ".",
            "--format",
            "html",
            "--output",
            output_path.to_str().expect("output path"),
            "--no-progress",
        ])
        .current_dir(temp.path())
        .output()
        .expect("run HTML review");

    assert!(
        output.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = fs::read_to_string(output_path).expect("read HTML review");
    assert!(html.contains("Contract / consumer map"));
    assert!(html.contains("src/api.ts"));
    assert!(html.contains("src/app.ts"));
    assert!(html.contains("RemovedExport"));
}

#[test]
fn review_html_renders_verification_outcomes_and_revision_state() {
    let mut report = ReviewReport {
        summary: ScanSummary {
            metadata: ScanMetadata {
                mode: ScanMode::Changed,
                ..Default::default()
            },
            metrics: ScanMetrics {
                files_discovered: 1,
                files_analyzed: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        repo_root: PathBuf::from("/repo"),
        baseline_path: None,
        changed_files: vec![ChangedFile {
            path: PathBuf::from("src/lib.rs"),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }],
        blast_radius: Vec::new(),
        impact_paths: Default::default(),
        ownership: Default::default(),
        ownership_diagnostics: Vec::new(),
        boundary_signals: Vec::new(),
        boundary_missing_test: false,
        tiered_signals: Default::default(),
        timings: Default::default(),
        verification: Vec::new(),
        findings: Vec::new(),
    };
    report.verification.push(VerificationOutcome {
        check_id: "unit".to_string(),
        role: VerificationRole::Test,
        status: VerificationStatus::Passed,
        duration_ms: 12,
        exit_code: Some(0),
        working_directory: ".".to_string(),
        stdout_excerpt: String::new(),
        stderr_excerpt: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        revision_before: "same".to_string(),
        revision_after: "same".to_string(),
        revision_compatible: true,
        limitations: Vec::new(),
        reused: true,
    });

    let html = render_review_html(&report, None, None);

    assert!(html.contains("<h2>Verification</h2>"));
    assert!(html.contains("<code>unit</code>"));
    assert!(html.contains("Passed"));
    assert!(html.contains("cached"));
    assert!(html.contains("compatible"));
    assert!(html.contains("1 passed, 0 failed"));
}

fn init_repo(root: &Path) {
    git(root, &["init"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "RepoPilot Test"]);
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write fixture");
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}
