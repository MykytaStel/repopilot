use super::{ProgressMode, ProgressReporter, mode_for_tool_call};
use crate::commands::review_verification::ReviewVerificationEvent;
use repopilot::verification::{CancellationToken, VerificationStatus};
use serde_json::{Value, json};
use std::io;

#[test]
fn review_progress_is_monotonic_and_check_aware() {
    let cancellation = CancellationToken::new();
    let notifications = capture(
        ProgressMode::Review { checks: 2 },
        &cancellation,
        |reporter| {
            reporter.analysis_started();
            reporter.verification(ReviewVerificationEvent::Started {
                check_id: "lint".into(),
                index: 1,
                total: 2,
            });
            reporter.verification(ReviewVerificationEvent::Completed {
                check_id: "lint".into(),
                index: 1,
                total: 2,
                status: VerificationStatus::Passed,
            });
            reporter.verification(ReviewVerificationEvent::Started {
                check_id: "unit".into(),
                index: 2,
                total: 2,
            });
            reporter.verification(ReviewVerificationEvent::Completed {
                check_id: "unit".into(),
                index: 2,
                total: 2,
                status: VerificationStatus::Failed,
            });
            reporter.finish_success();
        },
    );

    assert_eq!(
        progress_messages(&notifications),
        vec![
            (0, 4, "analysis started"),
            (1, 4, "analysis complete"),
            (1, 4, "verification lint started"),
            (2, 4, "verification lint passed"),
            (2, 4, "verification unit started"),
            (3, 4, "verification unit failed"),
            (4, 4, "review complete"),
        ]
    );
}

#[test]
fn review_without_checks_has_analysis_and_finalization_units() {
    let cancellation = CancellationToken::new();
    let notifications = capture(
        ProgressMode::Review { checks: 0 },
        &cancellation,
        |reporter| {
            reporter.analysis_started();
            reporter.finish_success();
        },
    );

    assert_eq!(
        progress_messages(&notifications),
        vec![
            (0, 2, "analysis started"),
            (1, 2, "analysis complete"),
            (2, 2, "review complete"),
        ]
    );
}

#[test]
fn non_review_tool_keeps_existing_two_notification_contract() {
    let cancellation = CancellationToken::new();
    let notifications = capture(ProgressMode::Tool, &cancellation, |reporter| {
        reporter.analysis_started();
        reporter.finish_success();
    });

    assert_eq!(
        progress_messages(&notifications),
        vec![(0, 1, "analysis started"), (1, 1, "analysis complete")]
    );
}

#[test]
fn cancellation_suppresses_all_later_notifications() {
    let cancellation = CancellationToken::new();
    let observer_token = cancellation.clone();
    let notifications = capture(
        ProgressMode::Review { checks: 1 },
        &cancellation,
        |reporter| {
            reporter.analysis_started();
            observer_token.cancel();
            reporter.verification(ReviewVerificationEvent::Started {
                check_id: "unit".into(),
                index: 1,
                total: 1,
            });
            reporter.finish_success();
        },
    );

    assert_eq!(
        progress_messages(&notifications),
        vec![(0, 3, "analysis started")]
    );
}

#[test]
fn review_mode_counts_unique_string_selectors() {
    assert_eq!(
        mode_for_tool_call(&json!({
            "name": "repopilot_review_change",
            "arguments": { "verify": ["unit", "lint", "unit"] }
        })),
        ProgressMode::Review { checks: 2 }
    );
    assert_eq!(
        mode_for_tool_call(&json!({
            "name": "repopilot_scan",
            "arguments": { "verify": ["ignored"] }
        })),
        ProgressMode::Tool
    );
}

fn capture(
    mode: ProgressMode,
    cancellation: &CancellationToken,
    run: impl FnOnce(&mut ProgressReporter<'_>),
) -> Vec<Value> {
    let mut notifications = Vec::new();
    {
        let mut sink = |value: Value| -> io::Result<()> {
            notifications.push(value);
            Ok(())
        };
        let mut reporter =
            ProgressReporter::new(Some(json!("token")), mode, cancellation, &mut sink);
        run(&mut reporter);
        reporter.into_result().expect("progress sink");
    }
    notifications
}

fn progress_messages(notifications: &[Value]) -> Vec<(u64, u64, &str)> {
    notifications
        .iter()
        .map(|notification| {
            let params = &notification["params"];
            (
                params["progress"].as_u64().expect("progress"),
                params["total"].as_u64().expect("total"),
                params["message"].as_str().expect("message"),
            )
        })
        .collect()
}
