use super::{FindingIntent, FindingVisibilityDecision};
use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::risk::RiskPriority;
use crate::rules::{RuleLifecycle, SignalSource, lookup_rule_metadata};
use std::path::Path;

pub fn classify_visibility(finding: &Finding) -> FindingVisibilityDecision {
    let intent = classify_intent(finding);

    if is_validated_secret_leak(finding) {
        return FindingVisibilityDecision::visible(
            FindingIntent::SecurityRisk,
            "validated secret or private key candidate",
        );
    }

    if finding.confidence == Confidence::Low {
        return FindingVisibilityDecision::hidden(
            intent,
            "low-confidence findings are strict-mode suggestions by default",
        );
    }

    if finding.severity <= Severity::Low {
        return FindingVisibilityDecision::hidden(
            intent,
            "low-severity findings are strict-mode suggestions by default",
        );
    }

    if is_manifest_backed_package_boundary(finding) {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "manifest-backed package boundary violation",
        );
    }

    match rule_lifecycle(finding) {
        RuleLifecycle::Experimental => {
            return FindingVisibilityDecision::hidden(
                intent,
                "experimental rules are strict-mode suggestions by default",
            );
        }
        RuleLifecycle::Deprecated => {
            return FindingVisibilityDecision::hidden(
                intent,
                "deprecated rules are hidden in the default profile",
            );
        }
        RuleLifecycle::Preview
            if finding.confidence == Confidence::Medium
                && !is_evidence_backed_actionable(finding, intent) =>
        {
            return FindingVisibilityDecision::hidden(
                intent,
                "medium-confidence preview finding lacks direct actionable evidence",
            );
        }
        RuleLifecycle::Preview | RuleLifecycle::Stable => {}
    }

    match intent {
        FindingIntent::SecurityRisk => security_visibility(finding),
        FindingIntent::RuntimeRisk => runtime_visibility(finding),
        FindingIntent::ActionableRisk => actionable_visibility(finding),
        FindingIntent::Maintainability => maintainability_visibility(finding),
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
    if is_import_graph_rule(&finding.rule_id) && finding.risk.priority.is_at_least(RiskPriority::P2)
    {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "stable import-graph architecture risk",
        );
    }

    if is_evidence_backed_actionable(finding, FindingIntent::ActionableRisk)
        && finding.severity >= Severity::Medium
    {
        return FindingVisibilityDecision::visible(
            FindingIntent::ActionableRisk,
            "evidence-backed actionable risk",
        );
    }

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

fn maintainability_visibility(finding: &Finding) -> FindingVisibilityDecision {
    if is_strict_only_heuristic_rule(&finding.rule_id) {
        return FindingVisibilityDecision::hidden(
            FindingIntent::Maintainability,
            "broad maintainability heuristics are strict-mode suggestions",
        );
    }

    if is_high_priority(finding.risk.priority) && finding.confidence != Confidence::Low {
        return FindingVisibilityDecision::visible(
            FindingIntent::Maintainability,
            "high-priority maintainability risk",
        );
    }

    FindingVisibilityDecision::hidden(
        FindingIntent::Maintainability,
        "maintainability signal is below the default priority threshold",
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

    if is_import_graph_rule(&finding.rule_id) {
        return FindingIntent::ActionableRisk;
    }

    if finding.category == FindingCategory::Framework {
        if is_framework_style_rule(&finding.rule_id) {
            return FindingIntent::Maintainability;
        }

        if is_direct_evidence_source(signal_source(finding)) {
            return FindingIntent::ActionableRisk;
        }
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
            | "architecture.deep-directory-nesting"
            | "architecture.too-many-modules"
            | "architecture.large-file"
            | "architecture.barrel-file-risk"
            | "architecture.deep-relative-imports"
            | "code-quality.long-function"
            | "code-quality.complex-file"
            | "code-quality.complex-function"
            | "code-quality.cyclomatic-complexity"
            | "code-quality.deep-control-flow"
    )
}

fn is_import_graph_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "architecture.circular-dependency"
            | "architecture.excessive-fan-out"
            | "architecture.high-instability-hub"
    )
}

fn is_framework_style_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "framework.js.var-declaration"
            | "framework.js.console-log"
            | "framework.react.class-component"
            | "framework.react.prop-types"
            | "framework.react-native.inline-style"
            | "framework.react-native.flatlist-missing-key"
            | "framework.react-native.old-architecture"
            | "framework.react-native.hermes-disabled"
    )
}

fn is_strict_only_heuristic_rule(rule_id: &str) -> bool {
    matches!(
        rule_id,
        "testing.source-without-test"
            | "testing.missing-test-folder"
            | "code-marker.todo"
            | "code-marker.fixme"
            | "code-marker.hack"
            | "architecture.deep-directory-nesting"
            | "architecture.too-many-modules"
            | "architecture.large-file"
            | "architecture.barrel-file-risk"
            | "architecture.deep-relative-imports"
            | "code-quality.long-function"
            | "code-quality.complex-file"
            | "code-quality.complex-function"
            | "code-quality.cyclomatic-complexity"
            | "code-quality.deep-control-flow"
    )
}

fn is_evidence_backed_actionable(finding: &Finding, intent: FindingIntent) -> bool {
    matches!(
        intent,
        FindingIntent::SecurityRisk | FindingIntent::RuntimeRisk | FindingIntent::ActionableRisk
    ) && is_direct_evidence_source(signal_source(finding))
}

fn is_direct_evidence_source(source: SignalSource) -> bool {
    matches!(
        source,
        SignalSource::Ast
            | SignalSource::ConfigFile
            | SignalSource::DependencyManifest
            | SignalSource::ImportGraph
            | SignalSource::FrameworkDetector
            | SignalSource::GitDiff
    )
}

fn rule_lifecycle(finding: &Finding) -> RuleLifecycle {
    lookup_rule_metadata(&finding.rule_id)
        .map(|metadata| metadata.lifecycle)
        .unwrap_or(finding.provenance.rule_lifecycle)
}

fn signal_source(finding: &Finding) -> SignalSource {
    if finding.provenance.detector != "unknown" {
        return finding.provenance.signal_source;
    }

    lookup_rule_metadata(&finding.rule_id)
        .map(|metadata| metadata.signal_source)
        .unwrap_or(finding.provenance.signal_source)
}

fn is_manifest_backed_package_boundary(finding: &Finding) -> bool {
    finding.rule_id == "architecture.package-boundary-violation"
        && finding.confidence == Confidence::High
        && signal_source(finding) == SignalSource::ImportGraph
        && !finding.evidence.is_empty()
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
