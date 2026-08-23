use crate::commands::review_verification::ReviewVerificationEvent;
use repopilot::verification::{CancellationToken, VerificationStatus};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProgressMode {
    Tool,
    Review { checks: usize },
}

pub(super) fn mode_for_tool_call(params: &Value) -> ProgressMode {
    if params.get("name").and_then(Value::as_str) != Some(super::review_change::TOOL_NAME) {
        return ProgressMode::Tool;
    }
    let checks = params
        .get("arguments")
        .and_then(|arguments| arguments.get("verify"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>()
        .len();
    ProgressMode::Review { checks }
}

pub(super) struct ProgressReporter<'a> {
    token: Option<Value>,
    mode: ProgressMode,
    cancellation: &'a CancellationToken,
    sink: &'a mut dyn FnMut(Value) -> io::Result<()>,
    analysis_complete: bool,
    error: Option<io::Error>,
}

impl<'a> ProgressReporter<'a> {
    pub(super) fn new(
        token: Option<Value>,
        mode: ProgressMode,
        cancellation: &'a CancellationToken,
        sink: &'a mut dyn FnMut(Value) -> io::Result<()>,
    ) -> Self {
        Self {
            token,
            mode,
            cancellation,
            sink,
            analysis_complete: false,
            error: None,
        }
    }

    pub(super) fn analysis_started(&mut self) {
        self.emit(0, self.total(), "analysis started".to_string());
    }

    pub(super) fn verification(&mut self, event: ReviewVerificationEvent) {
        self.ensure_analysis_complete();
        match event {
            ReviewVerificationEvent::Started {
                check_id, index, ..
            } => self.emit(
                index,
                self.total(),
                format!("verification {check_id} started"),
            ),
            ReviewVerificationEvent::Completed {
                check_id,
                index,
                status,
                ..
            } => self.emit(
                index + 1,
                self.total(),
                format!("verification {check_id} {}", status_label(status)),
            ),
        }
    }

    pub(super) fn finish_success(&mut self) {
        match self.mode {
            ProgressMode::Tool => {
                self.analysis_complete = true;
                self.emit(1, 1, "analysis complete".to_string());
            }
            ProgressMode::Review { checks } => {
                self.ensure_analysis_complete();
                self.emit(checks + 2, checks + 2, "review complete".to_string());
            }
        }
    }

    pub(super) fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub(super) fn into_result(self) -> io::Result<()> {
        self.error.map_or(Ok(()), Err)
    }

    fn ensure_analysis_complete(&mut self) {
        if self.analysis_complete {
            return;
        }
        self.analysis_complete = true;
        self.emit(1, self.total(), "analysis complete".to_string());
    }

    fn total(&self) -> usize {
        match self.mode {
            ProgressMode::Tool => 1,
            ProgressMode::Review { checks } => checks + 2,
        }
    }

    fn emit(&mut self, progress: usize, total: usize, message: String) {
        if self.token.is_none() || self.cancellation.is_cancelled() || self.error.is_some() {
            return;
        }
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": self.token.as_ref().expect("checked progress token"),
                "progress": progress,
                "total": total,
                "message": message
            }
        });
        if let Err(error) = (self.sink)(notification) {
            self.error = Some(error);
        }
    }
}

fn status_label(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Passed => "passed",
        VerificationStatus::Failed => "failed",
        VerificationStatus::TimedOut => "timed out",
        VerificationStatus::Unavailable => "unavailable",
        VerificationStatus::Cancelled => "cancelled",
        VerificationStatus::Skipped => "skipped",
    }
}

#[cfg(test)]
mod tests;
