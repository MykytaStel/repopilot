use crate::findings::types::Severity;
use crate::receipt::git::collect_git_receipt;
use crate::receipt::model::{
    AUDIT_RECEIPT_SCHEMA_VERSION, AuditReceipt, ReceiptFindings, ReceiptLanguage, ReceiptScope,
};
use crate::scan::types::ScanSummary;
use chrono::Utc;

pub fn build_audit_receipt(summary: &ScanSummary) -> AuditReceipt {
    AuditReceipt {
        schema_version: AUDIT_RECEIPT_SCHEMA_VERSION,
        tool: "repopilot".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now().to_rfc3339(),
        root_path: summary.root_path.display().to_string(),
        git: collect_git_receipt(&summary.root_path),
        scope: build_scope(summary),
        findings: build_findings(summary),
        languages: build_languages(summary),
        health_score: summary.health_score,
    }
}

fn build_scope(summary: &ScanSummary) -> ReceiptScope {
    ReceiptScope {
        files_discovered: summary.files_discovered,
        files_analyzed: summary.files_count,
        directories_count: summary.directories_count,
        lines_of_code: summary.lines_of_code,
        files_skipped_low_signal: summary.files_skipped_low_signal,
        binary_files_skipped: summary.binary_files_skipped,
        large_files_skipped: summary.skipped_files_count,
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
            files_count: language.files_count,
        })
        .collect()
}
