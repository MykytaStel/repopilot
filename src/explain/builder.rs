use crate::audits::context::classify_file_with_evidence;
use crate::explain::model::{
    ExplainContext, ExplainDecision, ExplainDecisionTraceStep, ExplainReport, ExplainRiskSignal,
    ExplainRoleEvidence, ExplainScope, ExplainSource, ExplainVisibility,
};
use crate::findings::types::{Confidence, Evidence, Finding, FindingCategory, Severity};
use crate::findings::visibility::classify_visibility;
use crate::knowledge::decision::decide_for_file_with_trace;
use crate::knowledge::language::{detect_language_for_path, profile_by_id};
use crate::knowledge::model::{RuleDecisionAction, SupportLevel};
use crate::rules::registry::lookup_rule_metadata;
use crate::scan::facts::FileFacts;
use crate::scan::workspace::{package_roots, path_in_executable_package};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn build_explain_report(
    path: &Path,
    rule_id: Option<&str>,
    signal: Option<&str>,
    base_severity: Severity,
) -> Result<ExplainReport, io::Error> {
    let root = infer_analysis_root(path);
    build_explain_report_with_root(&root, path, rule_id, signal, base_severity)
}

pub fn build_explain_report_with_root(
    root: &Path,
    path: &Path,
    rule_id: Option<&str>,
    signal: Option<&str>,
    base_severity: Severity,
) -> Result<ExplainReport, io::Error> {
    let content = fs::read_to_string(path)?;
    let language_name = detect_language_for_path(path).map(str::to_string);
    let non_empty_lines = count_non_empty_lines(&content);
    let has_inline_tests = has_language_inline_tests(path, language_name.as_deref(), &content);
    let executable_roots = package_roots(root);
    let in_executable_package = path_in_executable_package(path, &executable_roots);

    let file = FileFacts {
        path: path.to_path_buf(),
        language: language_name.clone(),
        non_empty_lines,
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content),
        has_inline_tests,
        in_executable_package,
        deferred_imports: Vec::new(),
    };

    let classified = classify_file_with_evidence(&file);
    let audit_context = &classified.context;
    let arch_context = crate::analysis::classify_file_architecture(
        &file,
        &crate::scan::config::ScanConfig::default(),
    );
    let language_id = audit_context.language_id();
    let language_support =
        profile_by_id(language_id).map(|profile| support_level_label(profile.support).to_string());

    let context = ExplainContext {
        language: language_id.to_string(),
        language_support,
        frameworks: audit_context
            .framework_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        roles: audit_context
            .role_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        role_evidence: classified
            .role_evidence
            .iter()
            .map(|evidence| ExplainRoleEvidence {
                role: evidence.role.as_id().to_string(),
                source: evidence.source.as_id().to_string(),
                reason: evidence.reason.to_string(),
            })
            .collect(),
        paradigms: audit_context
            .paradigm_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        runtimes: audit_context
            .runtime_ids()
            .into_iter()
            .map(str::to_string)
            .collect(),
        is_test: audit_context.is_test,
        is_production_code: audit_context.is_production_code(),
        architecture_role: Some(format!("{:?}", arch_context.file_role)),
        module_kind: Some(format!("{:?}", arch_context.module_kind)),
        language_family: Some(format!("{:?}", arch_context.language_family)),
    };

    let decision = rule_id.map(|rule_id| {
        let traced = decide_for_file_with_trace(rule_id, &file, base_severity, signal);
        let visibility = explain_visibility(path, rule_id, traced.decision.severity);
        let mut trace = traced
            .steps
            .into_iter()
            .enumerate()
            .map(|(index, step)| ExplainDecisionTraceStep {
                order: index + 1,
                stage: step.stage.as_id().to_string(),
                status: step.status.as_id().to_string(),
                label: step.label,
                criteria: step.criteria,
                action: step
                    .action
                    .map(|action| decision_action_label(action).to_string()),
                severity_before: step.severity_before,
                severity_after: step.severity_after,
                reason: step.reason,
                override_index: step.override_index,
            })
            .collect::<Vec<_>>();

        trace.push(ExplainDecisionTraceStep {
            order: trace.len() + 1,
            stage: "visibility".to_string(),
            status: if visibility.visible_by_default {
                "visible".to_string()
            } else {
                "hidden".to_string()
            },
            label: "default-profile".to_string(),
            criteria: vec![
                format!("profile={}", visibility.profile),
                format!("intent={}", visibility.intent),
            ],
            action: None,
            severity_before: traced.decision.severity,
            severity_after: traced.decision.severity,
            reason: visibility.reason.clone(),
            override_index: None,
        });

        ExplainDecision {
            rule_id: rule_id.to_string(),
            signal: signal.map(str::to_string),
            base_severity,
            action: decision_action_label(traced.decision.action).to_string(),
            final_severity: traced.decision.severity,
            reason: traced.decision.reason,
            risk_signal: traced.decision.risk_signal.map(|signal| ExplainRiskSignal {
                id: signal.id,
                label: signal.label,
                weight: signal.weight,
                reason: signal.reason,
            }),
            trace,
            visibility: Some(visibility),
        }
    });

    Ok(ExplainReport {
        path: path.display().to_string(),
        scope: ExplainScope {
            analysis_scope: "single-file".to_string(),
            decision_source: "bundled-knowledge-pack".to_string(),
            visibility_profile: "default".to_string(),
            repository_context_included: false,
            package_manifest_context_included: true,
            scan_configuration_included: false,
            local_feedback_included: false,
            baseline_included: false,
            note: "Explains local file classification, executable-package manifest context, and the bundled knowledge decision only; repository graph context, repopilot.toml rule overrides, local feedback, baseline state, and full scan filtering are not applied.".to_string(),
        },
        source: ExplainSource {
            language_name,
            non_empty_lines,
            has_inline_tests,
        },
        context,
        decision,
    })
}

fn infer_analysis_root(path: &Path) -> PathBuf {
    let start = path.parent().unwrap_or_else(|| Path::new("."));

    start
        .ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .or_else(|| {
            start.ancestors().find(|candidate| {
                candidate.join("Cargo.toml").is_file() || candidate.join("package.json").is_file()
            })
        })
        .unwrap_or(start)
        .to_path_buf()
}

fn count_non_empty_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn has_language_inline_tests(path: &Path, language: Option<&str>, content: &str) -> bool {
    match language {
        Some("Rust") => content.contains("#[cfg(test)]") || content.contains("#[test]"),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => {
            contains_call(content, "describe")
                || contains_call(content, "it")
                || contains_call(content, "test")
        }
        Some("Python") => content.contains("def test_") || content.contains("unittest."),
        Some("Go") => content.contains("func Test") || content.contains("func Benchmark"),
        Some("Java") | Some("Kotlin") => content.contains("@Test"),
        _ => path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("_test.") || name.contains(".test.")),
    }
}

fn contains_call(content: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    content.match_indices(&needle).any(|(index, _)| {
        content[..index]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '.')
    })
}

fn support_level_label(level: SupportLevel) -> &'static str {
    match level {
        SupportLevel::DetectOnly => "detect-only",
        SupportLevel::ImportAware => "import-aware",
        SupportLevel::ContextAware => "context-aware",
        SupportLevel::RuleAware => "rule-aware",
    }
}

fn decision_action_label(action: RuleDecisionAction) -> &'static str {
    match action {
        RuleDecisionAction::Apply => "apply",
        RuleDecisionAction::Suppress => "suppress",
        RuleDecisionAction::Downgrade => "downgrade",
        RuleDecisionAction::Upgrade => "upgrade",
    }
}

fn explain_visibility(path: &Path, rule_id: &str, severity: Severity) -> ExplainVisibility {
    let category = lookup_rule_metadata(rule_id)
        .map(|metadata| metadata.category.clone())
        .unwrap_or(FindingCategory::CodeQuality);
    let finding = Finding {
        rule_id: rule_id.to_string(),
        category,
        severity,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: 1,
            line_end: None,
            snippet: "synthetic finding for visibility explanation".to_string(),
        }],
        ..Default::default()
    };
    let decision = classify_visibility(&finding);
    ExplainVisibility {
        profile: "default".to_string(),
        intent: decision.intent.label().to_string(),
        visible_by_default: decision.visible_by_default,
        reason: decision.reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explain::render::render_explain_report;
    use crate::output::OutputFormat;

    #[test]
    fn report_contains_role_evidence_scope_and_ordered_trace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("src/models/value.rs");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create fixture dirs");
        std::fs::write(&path, "pub fn value() -> usize { 1 }\n").expect("write fixture");
        let report = build_explain_report(
            &path,
            Some("code-quality.long-function"),
            None,
            Severity::Medium,
        )
        .expect("build explain report");

        assert_eq!(report.scope.analysis_scope, "single-file");
        assert!(!report.scope.repository_context_included);
        assert!(
            report
                .context
                .role_evidence
                .iter()
                .any(|evidence| { evidence.role == "domain" && evidence.source == "path" })
        );

        let decision = report.decision.as_ref().expect("rule decision");
        assert!(!decision.trace.is_empty());
        assert_eq!(
            decision.trace.last().expect("visibility step").stage,
            "visibility"
        );
        assert!(
            decision
                .trace
                .windows(2)
                .all(|pair| pair[0].order + 1 == pair[1].order)
        );

        let console = render_explain_report(&report, OutputFormat::Console).expect("console");
        let markdown = render_explain_report(&report, OutputFormat::Markdown).expect("markdown");
        let json = render_explain_report(&report, OutputFormat::Json).expect("json");
        assert!(console.contains("Decision trace:"));
        assert!(markdown.contains("### Role evidence"));
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse JSON");
        assert!(value["context"]["role_evidence"].is_array());
        assert!(value["decision"]["trace"].is_array());
        assert_eq!(value["scope"]["analysis_scope"], "single-file");
    }

    #[test]
    fn executable_manifest_context_matches_scanner_decision() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("package.json"),
            r#"{ "name": "demo-cli", "bin": { "demo": "./src/index.js" } }"#,
        )
        .expect("write package manifest");

        let path = temp.path().join("src/commands/stop.ts");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create command dir");
        std::fs::write(&path, "export function stop() { process.exit(1); }\n")
            .expect("write command");

        let report = build_explain_report_with_root(
            temp.path(),
            &path,
            Some("language.javascript.runtime-exit-risk"),
            Some("js.process-exit"),
            Severity::High,
        )
        .expect("build manifest-aware explain report");

        assert!(report.scope.package_manifest_context_included);
        assert!(report.context.role_evidence.iter().any(|evidence| {
            evidence.role == "cli-executable"
                && evidence.source == "mixed"
                && evidence.reason.contains("executable package manifest")
        }));

        let decision = report.decision.as_ref().expect("decision");
        assert_eq!(decision.action, "downgrade");
        assert_eq!(decision.final_severity, Severity::Low);
        assert!(decision.trace.iter().any(|step| {
            step.stage == "override"
                && step.status == "applied"
                && step
                    .criteria
                    .iter()
                    .any(|criterion| criterion == "role=cli-executable")
        }));
    }
}
