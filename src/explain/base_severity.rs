use crate::findings::types::Severity;
use crate::rules::registry::lookup_rule_metadata;

pub fn base_severity_for_explain(rule_id: Option<&str>, signal: Option<&str>) -> Severity {
    signal_base_severity(rule_id, signal)
        .or_else(|| {
            rule_id
                .and_then(lookup_rule_metadata)
                .map(|metadata| metadata.default_severity)
        })
        .unwrap_or(Severity::Info)
}

fn signal_base_severity(rule_id: Option<&str>, signal: Option<&str>) -> Option<Severity> {
    match (rule_id?, signal?) {
        ("language.javascript.runtime-exit-risk", "js.process-exit")
        | ("language.python.exception-risk", "python.not-implemented")
        | ("language.managed.fatal-exception-risk", "managed.not-implemented")
        | ("language.rust.panic-risk", "rust.todo" | "rust.unimplemented") => Some(Severity::High),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_signal_specific_detector_severity_when_known() {
        assert_eq!(
            base_severity_for_explain(
                Some("language.javascript.runtime-exit-risk"),
                Some("js.process-exit")
            ),
            Severity::High
        );
    }

    #[test]
    fn falls_back_to_rule_registry_default() {
        assert_eq!(
            base_severity_for_explain(Some("language.javascript.runtime-exit-risk"), None),
            Severity::Medium
        );
    }
}
