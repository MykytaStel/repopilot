use super::{FindingIntent, FindingVisibilityDecision};
use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::risk::RiskPriority;
use std::path::Path;

pub fn classify_visibility(finding: &Finding) -> FindingVisibilityDecision {
    let intent = classify_intent(finding);

    if is_validated_secret_leak(finding) {
        return FindingVisibilityDecision::visible(
            FindingIntent::SecurityRisk,
            "validated secret or private key candidate",
        );
    }

    if finding.severity <= Severity::Low {
        return FindingVisibilityDecision::hidden(
            intent,
            "low-severity findings are strict-mode suggestions by default",
        );
    }

    match intent {
        FindingIntent::SecurityRisk => security_visibility(finding),
        FindingIntent::RuntimeRisk => runtime_visibility(finding),
        FindingIntent::ActionableRisk => actionable_visibility(finding),
        FindingIntent::Maintainability => FindingVisibilityDecision::hidden(
            intent,
            "maintainability signals are hidden in the default profile",
        ),
        FindingIntent::TestingGap => FindingVisibilityDecision::hidden(
            intent,
            "testing gaps are hidden in the default profile",
        ),
        FindingIntent::Informational => FindingVisibilityDecision::hidden(
            intent,
            "informational findings are hidden in the default profile",
        ),
    }
}

fn security_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if finding.severity >= Severity::High && finding.confidence != Confidence::Low {
        return FindingVisibilityDecision::visible(
            FindingIntent::SecurityRisk,
            "high-confidence security risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::SecurityRisk,
        "security signal is below the default confidence/severity threshold",
    )
}

fn runtime_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if is_script_boundary_runtime_exit(finding) {
        return FindingVisibilityDecision::hidden(
            FindingIntent::RuntimeRisk,
            "process exit in script/tooling boundary is a strict-mode suggestion",
        );
    }

    if is_high_priority(finding.risk.priority) {
        return FindingVisibilityDecision::visible(
            FindingIntent::RuntimeRisk,
            "high-priority runtime risk",
        );
    }

    if finding.severity >= Severity::High && finding.confidence != Confidence::Low {
        return FindingVisibilityDecision::visible(
            FindingIntent::RuntimeRisk,
            "high-severity runtime risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::RuntimeRisk,
        "runtime signal is not actionable enough for the default profile",
    )
}

fn actionable_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if is_high_priority(finding.risk.priority) {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "high-priority actionable risk",
        );
    }

    if finding.severity >= Severity::High && finding.confidence == Confidence::High {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "high-severity high-confidence actionable risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::ActionableRisk,
        "actionable signal is below the default visibility threshold",
    )
}

fn classify_intent(finding: &Finding) -> FindingIntent {
    if finding.category == FindingCategory::Testing {
        return FindingIntent::TestingGap;
    }

    if finding.category == FindingCategory::Security {
        return FindingIntent::SecurityRisk;
    }

    if is_runtime_rule(&finding.rule_id) {
        return FindingIntent::RuntimeRisk;
    }

    if is_maintainability_rule(&finding.rule_id) {
        return FindingIntent::Maintainability;
    }

    if finding.severity >= Severity::High {
        return FindingIntent::ActionableRisk;
    }

    FindingIntent::Informational
}

fn is_runtime_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "language.rust.panic-risk"
            | "language.javascript.runtime-exit-risk"
            | "language.go.panic-exit-risk"
            | "language.python.exception-risk"
            | "language.jvm.exception-risk"
            | "language.csharp.exception-risk"
    )
}

fn is_maintainability_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "testing.source-without-test"
            | "code-marker.todo"
            | "architecture.deep-nesting"
            | "architecture.too-many-modules"
            | "architecture.large-file"
            | "architecture.barrel-file-risk"
            | "architecture.deep-relative-imports"
            | "code-quality.long-function"
            | "code-quality.complex-file"
            | "code-quality.cyclomatic-complexity"
    )
}

fn is_validated_secret_leak(finding: &Finding) -> bool {
    matches!(
        finding.rule_id.as_str(),
        "security.secret-candidate" | "security.private-key-candidate"
    ) && finding.severity >= Severity::High
        && finding.confidence != Confidence::Low
}

fn is_high_priority(priority: RiskPriority) -> bool {
    matches!(priority, RiskPriority::P0 | RiskPriority::P1)
}

fn is_script_boundary_runtime_exit(finding: &Finding) -> bool {
    if finding.rule_id != "language.javascript.runtime-exit-risk" {
        return false;
    }

    let Some(evidence) = finding.evidence.first() else {
        return false;
    };

    is_script_or_tooling_path(&evidence.path)
}

fn is_script_or_tooling_path(path: &Path) -> bool {
    let path_text = path.to_string_lossy().replace('\\', "/").to_lowercase();

    if path_text.contains("/src/") || path_text.starts_with("src/") {
        return false;
    }

    path_text.contains("/scripts/")
        || path_text.starts_with("scripts/")
        || path_text.contains("/tools/")
        || path_text.starts_with("tools/")
        || path_text.contains("/bin/")
        || path_text.starts_with("bin/")
        || path_text.contains("/ci/")
        || path_text.starts_with("ci/")
        || path_text.contains("/.github/")
        || path_text.starts_with(".github/")
        || path_text.contains("guard")
        || path_text.contains("check")
        || path_text.contains("lint")
        || path_text.contains("verify")
}
