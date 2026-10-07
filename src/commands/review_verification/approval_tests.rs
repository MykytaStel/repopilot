use super::super::{ReviewVerificationEvent, VerificationApproval};
use super::{ApprovalHooks, run_selected_with_approval};
use crate::commands::review_verification::tests::empty_report;
use repopilot::config::loader::parse_config;
use repopilot::findings::visibility::FindingVisibilityProfile;
use repopilot::review::diff::OwnedDiffTarget;
use repopilot::scan::config::ScanConfig;
use repopilot::scan::session::AnalysisSession;
use repopilot::verification::{CancellationToken, VerificationStatus, select_checks};
use std::fs;
use tempfile::{TempDir, tempdir};

#[cfg(unix)]
#[test]
fn accepted_check_executes_declined_check_is_skipped_and_order_is_deterministic() {
    let (root, session, config) =
        setup(&[("beta", "printf b >> runs"), ("alpha", "printf a >> runs")]);
    let mut report = empty_report(root.path());
    let mut prompted = Vec::new();
    let mut events = Vec::new();
    let mut reload = |id: &str| reload(root.path(), &config, id);
    let mut approve = |check: &repopilot::verification::ValidatedCheck| {
        prompted.push(check.id().to_string());
        if check.id() == "alpha" {
            VerificationApproval::Skip {
                limitation: "user declined".to_string(),
                stop_following: false,
            }
        } else {
            VerificationApproval::Accepted
        }
    };

    run_selected_with_approval(
        &["beta".into(), "alpha".into()],
        &session,
        &OwnedDiffTarget::WorkingTree,
        &mut report,
        &CancellationToken::new(),
        &mut |event| events.push(event),
        ApprovalHooks {
            reload_check: &mut reload,
            approve: &mut approve,
        },
    )
    .expect("review verification");

    assert_eq!(prompted, ["alpha", "beta"]);
    assert_eq!(report.verification[0].status, VerificationStatus::Skipped);
    assert_eq!(report.verification[0].limitations, ["user declined"]);
    assert_eq!(report.verification[1].status, VerificationStatus::Passed);
    assert_eq!(
        fs::read_to_string(root.path().join("runs")).expect("marker"),
        "b"
    );
    assert!(matches!(
        events[0],
        ReviewVerificationEvent::Started {
            index: 1,
            total: 2,
            ..
        }
    ));
    assert!(matches!(
        events[2],
        ReviewVerificationEvent::Started {
            index: 2,
            total: 2,
            ..
        }
    ));
}

#[test]
fn decline_continues_but_user_cancel_stops_later_prompts() {
    let (root, session, config) = setup(&[
        ("alpha", "printf a >> runs"),
        ("beta", "printf b >> runs"),
        ("gamma", "printf c >> runs"),
    ]);
    let mut report = empty_report(root.path());
    let mut prompted = Vec::new();
    let mut reload = |id: &str| reload(root.path(), &config, id);
    let mut approve = |check: &repopilot::verification::ValidatedCheck| {
        prompted.push(check.id().to_string());
        VerificationApproval::Skip {
            limitation: if check.id() == "alpha" {
                "user declined"
            } else {
                "user cancelled"
            }
            .into(),
            stop_following: check.id() == "beta",
        }
    };

    run_selected_with_approval(
        &["gamma".into(), "beta".into(), "alpha".into()],
        &session,
        &OwnedDiffTarget::WorkingTree,
        &mut report,
        &CancellationToken::new(),
        &mut |_| {},
        ApprovalHooks {
            reload_check: &mut reload,
            approve: &mut approve,
        },
    )
    .expect("review verification");

    assert_eq!(prompted, ["alpha", "beta"]);
    assert_eq!(report.verification.len(), 3);
    assert_eq!(
        report.verification[2].limitations[0],
        "approval stopped after previous check"
    );
    assert!(!root.path().join("runs").exists());
}

#[cfg(unix)]
#[test]
fn workspace_or_policy_drift_after_acceptance_skips_check() {
    for (mode, expected_limitation) in [
        ("workspace", "workspace changed before execution"),
        ("policy", "check definition changed before execution"),
    ] {
        let original = "[[verification.checks]]\nid = \"unit\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"printf x >> runs\"]\n";
        let changed = if mode == "policy" {
            original.replace("printf x", "printf changed")
        } else {
            original.to_string()
        };
        let root = tempdir().expect("root");
        fs::write(root.path().join("source.txt"), "initial").expect("source");
        let config = parse_config(original, None).expect("config");
        let session = AnalysisSession::new(
            root.path().to_path_buf(),
            config,
            ScanConfig::default(),
            FindingVisibilityProfile::Default,
        );
        let mut report = empty_report(root.path());
        let mut reload_check = |id: &str| reload(root.path(), &changed, id);
        let mut approve = |_check: &repopilot::verification::ValidatedCheck| {
            if mode == "workspace" {
                fs::write(root.path().join("source.txt"), "changed").expect("drift");
            }
            VerificationApproval::Accepted
        };

        run_selected_with_approval(
            &["unit".into()],
            &session,
            &OwnedDiffTarget::WorkingTree,
            &mut report,
            &CancellationToken::new(),
            &mut |_| {},
            ApprovalHooks {
                reload_check: &mut reload_check,
                approve: &mut approve,
            },
        )
        .expect("review verification");

        assert_eq!(report.verification[0].status, VerificationStatus::Skipped);
        assert_eq!(report.verification[0].limitations[0], expected_limitation);
        assert!(!root.path().join("runs").exists());
    }
}

fn setup(checks: &[(&str, &str)]) -> (TempDir, AnalysisSession, String) {
    let root = tempdir().expect("root");
    let config = checks
        .iter()
        .map(|(id, command)| {
            format!(
                "[[verification.checks]]\nid = \"{id}\"\nrole = \"test\"\nprogram = \"sh\"\nargs = [\"-c\", \"{command}\"]\n"
            )
        })
        .collect::<String>();
    let parsed = parse_config(&config, None).expect("config");
    let session = AnalysisSession::new(
        root.path().to_path_buf(),
        parsed,
        ScanConfig::default(),
        FindingVisibilityProfile::Default,
    );
    (root, session, config)
}

fn reload(
    root: &std::path::Path,
    config: &str,
    id: &str,
) -> Result<repopilot::verification::ValidatedCheck, String> {
    let parsed = parse_config(config, None).map_err(|error| error.to_string())?;
    select_checks(root, &parsed.verification.checks, &[id.to_string()])
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "check missing".to_string())
}
