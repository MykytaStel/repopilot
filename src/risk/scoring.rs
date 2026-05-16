use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::scan::facts::{FileFacts, ScanFacts};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::context::add_file_context_signals;
use super::model::{RiskAssessment, RiskInputs, RiskSignal, clamp_score, push_adjustment, signal};
use super::overlays::baseline_signal;

pub fn assess_findings(findings: &mut [Finding], facts: &ScanFacts) {
    let files = file_index(facts);
    for finding in findings {
        let file = finding_file(finding, facts.root_path.as_path(), &files);
        finding.risk = assess_finding(finding, file, RiskInputs::default());
    }
}

pub fn assess_finding(
    finding: &Finding,
    file: Option<&FileFacts>,
    inputs: RiskInputs,
) -> RiskAssessment {
    let base = severity_base_score(finding.severity);
    let confidence_delta = confidence_delta(base, finding.confidence);
    let mut score = base as i16 + confidence_delta;
    let mut signals = vec![severity_signal(finding.severity, base)];

    if confidence_delta != 0 {
        signals.push(confidence_signal(finding.confidence, confidence_delta));
    }

    if finding.category == FindingCategory::Security {
        push_adjustment(
            &mut score,
            &mut signals,
            "category.security",
            "security finding",
            12,
            "security findings usually have higher remediation priority",
        );
    }

    if let Some(file) = file {
        add_file_context_signals(file, &mut score, &mut signals);
    }

    if let Some(status) = inputs.baseline_status {
        let signal = baseline_signal(status);
        super::model::push_signal(&mut score, &mut signals, signal);
    }

    if inputs.in_diff {
        push_adjustment(
            &mut score,
            &mut signals,
            "review.in-diff",
            "changed lines",
            12,
            "finding touches changed diff lines",
        );
    }

    if inputs.workspace_hotspot {
        push_adjustment(
            &mut score,
            &mut signals,
            "workspace.hotspot",
            "workspace hotspot",
            5,
            "workspace package has multiple high-risk findings",
        );
    }

    RiskAssessment::new(clamp_score(score), signals)
}

fn severity_base_score(severity: Severity) -> u8 {
    match severity {
        Severity::Critical => 95,
        Severity::High => 75,
        Severity::Medium => 45,
        Severity::Low => 20,
        Severity::Info => 5,
    }
}

fn severity_signal(severity: Severity, score: u8) -> RiskSignal {
    signal(
        &format!("severity.{}", severity.lowercase_label()),
        &format!("{} severity", severity.label()),
        score as i16,
        "base score from rule severity",
    )
}

fn confidence_delta(base: u8, confidence: Confidence) -> i16 {
    let multiplier = match confidence {
        Confidence::High => 1.10,
        Confidence::Medium => 1.00,
        Confidence::Low => 0.80,
    };
    ((base as f64 * multiplier).round() as i16) - base as i16
}

fn confidence_signal(confidence: Confidence, weight: i16) -> RiskSignal {
    signal(
        &format!("confidence.{}", confidence.lowercase_label()),
        &format!("{} confidence", confidence.label()),
        weight,
        "confidence adjusts certainty without changing rule severity",
    )
}

fn file_index(facts: &ScanFacts) -> HashMap<PathBuf, &FileFacts> {
    let mut files = HashMap::new();
    for file in &facts.files {
        files.insert(file.path.clone(), file);
        if let Ok(relative) = file.path.strip_prefix(&facts.root_path) {
            files.insert(relative.to_path_buf(), file);
        }
    }
    files
}

fn finding_file<'a>(
    finding: &Finding,
    root: &Path,
    files: &'a HashMap<PathBuf, &'a FileFacts>,
) -> Option<&'a FileFacts> {
    let evidence = finding.evidence.first()?;
    files.get(&evidence.path).copied().or_else(|| {
        evidence
            .path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| files.get(relative).copied())
    })
}
