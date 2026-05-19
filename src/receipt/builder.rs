use crate::findings::types::Severity;
use crate::receipt::git::collect_git_receipt;
use crate::receipt::model::{
    AUDIT_RECEIPT_SCHEMA_VERSION, AuditReceipt, ReceiptDiagnostic, ReceiptFindings,
    ReceiptHiddenSuggestion, ReceiptLanguage, ReceiptScope,
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
        languages: build_languages(summary),
        diagnostics: build_diagnostics(summary),
        health_score: summary.health_score,
    }
}

fn build_scope(summary: &ScanSummary) -> ReceiptScope {
    ReceiptScope {
        mode: summary.mode.label().to_string(),
        base_ref: summary.base_ref.clone(),
        changed_files_count: summary.changed_files_count,
        repo_level_rules_included: summary.repo_level_rules_included,
        files_discovered: summary.files_discovered,
        files_analyzed: summary.files_analyzed,
        directories_count: summary.directories_count,
        non_empty_lines: summary.non_empty_lines,
        files_skipped_low_signal: summary.files_skipped_low_signal,
        binary_files_skipped: summary.binary_files_skipped,
        large_files_skipped: summary.large_files_skipped,
        files_skipped_by_limit: summary.files_skipped_by_limit,
        files_skipped_repopilotignore: summary.files_skipped_repopilotignore,
        skipped_bytes: summary.skipped_bytes,
        repopilotignore_path: summary
            .repopilotignore_path
            .as_ref()
            .map(|path| path.display().to_string()),
    }
}

fn build_findings(summary: &ScanSummary) -> ReceiptFindings {
    let mut findings = ReceiptFindings {
        total: summary.findings.len(),
        hidden_suggestions_count: summary.hidden_suggestions_count,
        hidden_suggestions: summary
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

    for finding in &summary.findings {
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

fn build_languages(summary: &ScanSummary) -> Vec<ReceiptLanguage> {
    summary
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
