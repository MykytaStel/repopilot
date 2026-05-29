use crate::findings::types::Severity;
use crate::receipt::git::collect_git_receipt;
use crate::receipt::model::{
    AUDIT_RECEIPT_SCHEMA_VERSION, AuditReceipt, ReceiptDiagnostic, ReceiptFindings,
    ReceiptHiddenSuggestion, ReceiptLanguage, ReceiptLocalFeedback, ReceiptScope,
};
use crate::report::schema::ReportEnvelope;
use crate::scan::types::ScanSummary;
use chrono::Utc;

pub fn build_audit_receipt(summary: &ScanSummary) -> AuditReceipt {
    AuditReceipt {
        schema_version: AUDIT_RECEIPT_SCHEMA_VERSION,
        report: ReportEnvelope::receipt(AUDIT_RECEIPT_SCHEMA_VERSION),
        tool: "repopilot".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now().to_rfc3339(),
        root_path: summary.root_path.display().to_string(),
        git: collect_git_receipt(&summary.root_path),
        scope: build_scope(summary),
        findings: build_findings(summary),
        local_feedback: build_local_feedback(summary),
        languages: build_languages(summary),
        diagnostics: build_diagnostics(summary),
        health_score: summary.metrics.health_score,
    }
}

fn build_scope(summary: &ScanSummary) -> ReceiptScope {
    ReceiptScope {
        mode: summary.mode.label().to_string(),
        base_ref: summary.base_ref.clone(),
        changed_files_count: summary.metrics.changed_files_count,
        repo_level_rules_included: summary.repo_level_rules_included,
        files_discovered: summary.metrics.files_discovered,
        files_analyzed: summary.metrics.files_analyzed,
        directories_count: summary.metrics.directories_count,
        non_empty_lines: summary.metrics.non_empty_lines,
        files_skipped_low_signal: summary.metrics.files_skipped_low_signal,
        binary_files_skipped: summary.metrics.binary_files_skipped,
        large_files_skipped: summary.metrics.large_files_skipped,
        files_skipped_by_limit: summary.metrics.files_skipped_by_limit,
        files_skipped_repopilotignore: summary.metrics.files_skipped_repopilotignore,
        skipped_bytes: summary.metrics.skipped_bytes,
        repopilotignore_path: summary
            .repopilotignore_path
            .as_ref()
            .map(|path| path.display().to_string()),
    }
}

fn build_findings(summary: &ScanSummary) -> ReceiptFindings {
    let mut findings = ReceiptFindings {
        total: summary.artifacts.findings.len(),
        hidden_suggestions_count: summary.metrics.hidden_suggestions_count,
        hidden_suggestions: summary
            .artifacts
            .hidden_suggestions
            .iter()
            .map(|item| ReceiptHiddenSuggestion {
                intent: item.intent.clone(),
                rule_id: item.rule_id.clone(),
                category: item.category.clone(),
                reason: item.reason.clone(),
                count: item.count,
            })
            .collect(),
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
        info: 0,
    };

    for finding in &summary.artifacts.findings {
        match finding.severity {
            Severity::Critical => findings.critical += 1,
            Severity::High => findings.high += 1,
            Severity::Medium => findings.medium += 1,
            Severity::Low => findings.low += 1,
            Severity::Info => findings.info += 1,
        }
    }

    findings
}

fn build_local_feedback(summary: &ScanSummary) -> Option<ReceiptLocalFeedback> {
    summary
        .local_feedback
        .as_ref()
        .map(|feedback| ReceiptLocalFeedback {
            feedback_path: feedback
                .feedback_path
                .as_ref()
                .map(|path| path.display().to_string()),
            suppressions_loaded: feedback.suppressions_loaded,
            suppressed_findings_count: feedback.suppressed_findings_count,
            unmatched_suppressions_count: feedback.unmatched_suppressions_count,
            invalid_suppressions_count: feedback.invalid_suppressions_count,
            unmatched_suppressions: feedback.unmatched_suppressions.clone(),
            parse_error: feedback.parse_error.clone(),
        })
}

fn build_languages(summary: &ScanSummary) -> Vec<ReceiptLanguage> {
    summary
        .metrics
        .languages
        .iter()
        .map(|language| ReceiptLanguage {
            name: language.name.clone(),
            files_analyzed: language.files_analyzed,
        })
        .collect()
}

fn build_diagnostics(summary: &ScanSummary) -> Vec<ReceiptDiagnostic> {
    summary
        .artifacts
        .diagnostics
        .iter()
        .map(|diagnostic| ReceiptDiagnostic {
            code: diagnostic.code.clone(),
            severity: diagnostic.severity,
            message: diagnostic.message.clone(),
            path: diagnostic
                .path
                .as_ref()
                .map(|path| path.display().to_string()),
        })
        .collect()
}
