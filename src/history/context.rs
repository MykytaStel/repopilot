use crate::findings::types::Finding;
use crate::history::diagnostic::HistoryDiagnostic;
use crate::history::model::{
    AnalysisScope, ComparisonIdentity, ComparisonResult, FindingReceipt, RunReceipt,
};
use crate::history::storage::{HistoryLimits, HistoryStore};
use crate::report::schema::SCAN_REPORT_SCHEMA_VERSION;
use crate::scan::cache::{config_fingerprint, stable_hash_hex};
use crate::scan::session::AnalysisSession;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ReceiptContext {
    pub scope: AnalysisScope,
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub selection_fingerprint: String,
}

#[derive(Debug)]
pub struct HistoryRecordOutcome {
    pub receipt: RunReceipt,
    pub comparison: Option<ComparisonResult>,
    pub diagnostics: Vec<HistoryDiagnostic>,
    pub recorded: bool,
}

pub fn record_session(
    session: &AnalysisSession,
    context: ReceiptContext,
    findings: &[Finding],
) -> HistoryRecordOutcome {
    let receipt = build_receipt(session, context, findings);
    let limits = HistoryLimits {
        max_runs: session.repo_config().history.max_runs,
        max_bytes: session.repo_config().history.max_bytes,
    };
    let store = HistoryStore::new(session.workspace_root(), limits);
    let loaded = store.load();
    let compatible = loaded
        .receipts
        .iter()
        .rev()
        .find(|prior| prior.comparison == receipt.comparison);
    let prior = compatible.or_else(|| loaded.receipts.last());
    let comparison = prior.map(|prior| crate::history::delta::compare(&receipt, prior));
    let mut diagnostics = loaded.diagnostics;
    let recorded = match store.record(&receipt) {
        Ok(()) => true,
        Err(error) => {
            diagnostics.push(HistoryDiagnostic::write_failed(error.to_string()));
            false
        }
    };
    HistoryRecordOutcome {
        receipt,
        comparison,
        diagnostics,
        recorded,
    }
}

pub fn build_receipt(
    session: &AnalysisSession,
    context: ReceiptContext,
    findings: &[Finding],
) -> RunReceipt {
    let workspace = normalized_path(session.workspace_root());
    let analysis_target = relative_analysis_target(session);
    let comparison = ComparisonIdentity {
        workspace,
        analysis_target,
        scope: context.scope,
        base_revision: resolve_optional_ref(session.workspace_root(), context.base_ref),
        head_revision: resolve_optional_ref(session.workspace_root(), context.head_ref),
        profile: session.visibility_profile().label().to_string(),
        config_fingerprint: config_fingerprint(session.scan_config()),
        selection_fingerprint: context.selection_fingerprint,
        overlay_fingerprint: overlay_fingerprint(session.workspace_root()),
        analysis_schema: SCAN_REPORT_SCHEMA_VERSION.to_string(),
    };
    let receipts = findings
        .iter()
        .map(|finding| FindingReceipt::from_finding(finding, session.workspace_root()));
    RunReceipt::new(
        comparison,
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        session.revision().id().to_string(),
        receipts,
    )
}

fn relative_analysis_target(session: &AnalysisSession) -> String {
    let absolute = absolute_path(session.analysis_path());
    absolute
        .strip_prefix(session.workspace_root())
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .map(normalized_path)
        .unwrap_or_else(|| ".".to_string())
}

fn absolute_path(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    fs::canonicalize(&path).unwrap_or(path)
}

fn resolve_optional_ref(root: &Path, reference: Option<String>) -> Option<String> {
    reference.map(|reference| {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "--verify", &reference])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|resolved| !resolved.is_empty())
            .unwrap_or(reference)
    })
}

fn overlay_fingerprint(root: &Path) -> String {
    let path = root.join(".repopilot/overlay.toml");
    match fs::read(path) {
        Ok(bytes) => stable_hash_hex(&bytes),
        Err(_) => stable_hash_hex(b"overlay:missing"),
    }
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
