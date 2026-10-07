use super::elicitation::{ElicitationBroker, ElicitationFailure};
use crate::commands::review_verification::VerificationApproval;
use repopilot::config::loader::{discover_config_path, load_optional_config};
use repopilot::config::model::RepoPilotConfig;
use repopilot::verification::CancellationToken;
use repopilot::verification::ValidatedCheck;
use repopilot::verification::select_checks;
use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub(super) enum EffectiveConfigSource {
    Explicit(PathBuf),
    Discovered(PathBuf),
    Defaults,
}

impl EffectiveConfigSource {
    pub(super) fn resolve(explicit: Option<&Path>, root: &Path) -> Self {
        if let Some(path) = explicit {
            return Self::Explicit(path.to_path_buf());
        }
        discover_config_path(root).map_or(Self::Defaults, Self::Discovered)
    }

    fn load(&self) -> Result<RepoPilotConfig, String> {
        match self {
            Self::Explicit(path) | Self::Discovered(path) => {
                load_optional_config(path).map_err(|error| error.to_string())
            }
            Self::Defaults => Ok(RepoPilotConfig::default()),
        }
    }
}

pub(super) fn load_initial_config(
    source: &EffectiveConfigSource,
) -> Result<RepoPilotConfig, String> {
    source.load()
}

pub(super) fn reload_check(
    root: &Path,
    source: &EffectiveConfigSource,
    check_id: &str,
) -> Result<ValidatedCheck, String> {
    let config = source.load()?;
    select_checks(root, &config.verification.checks, &[check_id.to_string()])
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "selected verification check disappeared".to_string())
}

pub(super) fn request_approval<W: Write>(
    check: &ValidatedCheck,
    supported: bool,
    broker: &ElicitationBroker,
    writer: &Arc<Mutex<&mut W>>,
    cancellation: &CancellationToken,
) -> VerificationApproval {
    if !supported {
        return skipped("client does not support approval", false);
    }
    if !check.has_stable_executable_identity() {
        return skipped("configured executable could not be resolved", false);
    }
    let params = json!({
        "mode": "form",
        "message": check.approval_details(),
        "requestedSchema": {
            "type": "object",
            "properties": {
                "approve": {
                    "type": "boolean",
                    "title": "Allow this verification check to run?"
                }
            },
            "required": ["approve"],
            "additionalProperties": false
        }
    });
    match broker.request_form(writer, params, cancellation, APPROVAL_TIMEOUT) {
        Ok(result) => response_decision(&result),
        Err(ElicitationFailure::Cancelled) => VerificationApproval::ToolCallCancelled,
        Err(ElicitationFailure::TimedOut) => skipped("approval timed out", true),
        Err(ElicitationFailure::InvalidResponse | ElicitationFailure::Transport) => {
            skipped("invalid approval response", true)
        }
    }
}

fn response_decision(result: &Value) -> VerificationApproval {
    match result.get("action").and_then(Value::as_str) {
        Some("accept") if accepted_content(result.get("content")) => VerificationApproval::Accepted,
        Some("decline") => skipped("user declined", false),
        Some("cancel") => skipped("user cancelled", true),
        _ => skipped("invalid approval response", true),
    }
}

fn accepted_content(content: Option<&Value>) -> bool {
    let Some(content) = content.and_then(Value::as_object) else {
        return false;
    };
    content.len() == 1 && content.get("approve").and_then(Value::as_bool) == Some(true)
}

fn skipped(limitation: &str, stop_following: bool) -> VerificationApproval {
    VerificationApproval::Skip {
        limitation: limitation.to_string(),
        stop_following,
    }
}

#[cfg(test)]
mod tests {
    use super::response_decision;
    use crate::commands::review_verification::VerificationApproval;
    use serde_json::json;

    #[test]
    fn only_explicit_accept_true_authorizes_execution() {
        assert_eq!(
            response_decision(&json!({ "action": "accept", "content": { "approve": true } })),
            VerificationApproval::Accepted
        );
        assert_eq!(
            response_decision(&json!({
                "_meta": { "trace": "supported extension" },
                "action": "accept",
                "content": { "approve": true }
            })),
            VerificationApproval::Accepted
        );
        for invalid in [
            json!({ "action": "accept", "content": { "approve": false } }),
            json!({ "action": "accept", "content": { "approve": true, "extra": 1 } }),
            json!({ "action": "accept" }),
            json!({ "action": "unknown" }),
        ] {
            assert_eq!(
                response_decision(&invalid),
                VerificationApproval::Skip {
                    limitation: "invalid approval response".to_string(),
                    stop_following: true,
                }
            );
        }
        assert_eq!(
            response_decision(&json!({ "action": "decline" })),
            VerificationApproval::Skip {
                limitation: "user declined".to_string(),
                stop_following: false,
            }
        );
    }
}
