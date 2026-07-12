//! Central redaction policy for evidence rendered into human-readable output.

use crate::findings::types::Finding;
use std::borrow::Cow;

pub const REDACTED_EVIDENCE: &str = "[sensitive evidence redacted]";

pub fn human_evidence_snippet<'a>(finding: &Finding, snippet: &'a str) -> &'a str {
    if is_sensitive_evidence_rule(&finding.rule_id) && !snippet.trim().is_empty() {
        REDACTED_EVIDENCE
    } else {
        snippet
    }
}

pub fn is_sensitive_evidence_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "security.secret-candidate" | "security.private-key-candidate"
    )
}

pub fn human_verification_step<'a>(finding: &Finding, step: &'a str) -> Cow<'a, str> {
    if !is_sensitive_evidence_rule(&finding.rule_id) {
        return Cow::Borrowed(step);
    }
    let mut redacted = step.to_string();
    for evidence in &finding.evidence {
        let snippet = evidence.snippet.trim();
        if !snippet.is_empty() {
            redacted = redacted.replace(snippet, REDACTED_EVIDENCE);
        }
    }
    Cow::Owned(redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_rules_without_changing_normal_evidence() {
        let mut finding = Finding {
            rule_id: "security.secret-candidate".to_string(),
            ..Finding::default()
        };
        assert_eq!(
            human_evidence_snippet(&finding, "TOKEN=live"),
            REDACTED_EVIDENCE
        );
        finding.evidence.push(crate::findings::types::Evidence {
            path: std::path::PathBuf::from("src/config.rs"),
            line_start: 1,
            line_end: None,
            snippet: "TOKEN=live".to_string(),
        });
        assert_eq!(
            human_verification_step(&finding, "Confirm `TOKEN=live`.").as_ref(),
            "Confirm `[sensitive evidence redacted]`."
        );
        finding.rule_id = "architecture.large-file".to_string();
        assert_eq!(human_evidence_snippet(&finding, "501 lines"), "501 lines");
    }
}
