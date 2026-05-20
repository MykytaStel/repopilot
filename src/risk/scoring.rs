use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::knowledge::decision::decide_for_file;
use crate::scan::facts::{FileFacts, ScanFacts};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::context::add_file_context_signals;
use super::model::{
    GraphImpact, RiskAssessment, RiskFormula, RiskInputs, RiskSignal, clamp_score, push_adjustment,
    signal,
};
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
            "security",
            RiskFormula::CURRENT.security_category_weight,
            "security findings are prioritized",
        );
    }

    if let Some(file) = file {
        add_knowledge_rule_signal(finding, file, &mut score, &mut signals);
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
            "review diff",
            RiskFormula::CURRENT.review_in_diff_weight,
            "finding touches changed diff lines",
        );
    }

    if inputs.workspace_hotspot {
        push_adjustment(
            &mut score,
            &mut signals,
            "workspace.hotspot",
            "workspace",
            RiskFormula::CURRENT.workspace_hotspot_weight,
            "workspace package has multiple high-risk findings",
        );
    }

    if let Some(impact) = inputs.graph_impact {
        add_graph_signal(impact, &mut score, &mut signals);
    }

    if inputs.blast_radius {
        push_adjustment(
            &mut score,
            &mut signals,
            "review.blast-radius",
            "review diff",
            RiskFormula::CURRENT.blast_radius_weight,
            "finding is in a file impacted by changed import dependencies",
        );
    }

    if inputs.cluster_size >= 3 {
        let weight = cluster_weight(inputs.cluster_size);
        push_adjustment(
            &mut score,
            &mut signals,
            "cluster.repeated",
            "cluster",
            weight,
            "same rule appears repeatedly in the same repository area",
        );
    }

    RiskAssessment::new(clamp_score(score), signals)
}

fn add_knowledge_rule_signal(
    finding: &Finding,
    file: &FileFacts,
    score: &mut i16,
    signals: &mut Vec<RiskSignal>,
) {
    let decision = decide_for_file(&finding.rule_id, file, finding.severity, None);
    let Some(rule_signal) = decision.risk_signal else {
        return;
    };

    push_adjustment(
        score,
        signals,
        rule_signal.id.as_str(),
        rule_signal.label.as_str(),
        rule_signal.weight,
        rule_signal.reason.as_str(),
    );
}

fn add_graph_signal(impact: GraphImpact, score: &mut i16, signals: &mut Vec<RiskSignal>) {
    match impact {
        GraphImpact::Hub => push_adjustment(
            score,
            signals,
            "graph.hub",
            "graph",
            RiskFormula::CURRENT.graph_hub_weight,
            "file is an import hub",
        ),
        GraphImpact::Dependency => push_adjustment(
            score,
            signals,
            "graph.dependency",
            "graph",
            RiskFormula::CURRENT.graph_dependency_weight,
            "file is a shared dependency",
        ),
    }
}

fn cluster_weight(size: usize) -> i16 {
    match size {
        0..=2 => 0,
        3..=5 => RiskFormula::CURRENT.cluster_small_weight,
        6..=15 => RiskFormula::CURRENT.cluster_medium_weight,
        _ => RiskFormula::CURRENT.cluster_large_weight,
    }
}

fn severity_base_score(severity: Severity) -> u8 {
    match severity {
        Severity::Critical => RiskFormula::CURRENT.severity_critical,
        Severity::High => RiskFormula::CURRENT.severity_high,
        Severity::Medium => RiskFormula::CURRENT.severity_medium,
        Severity::Low => RiskFormula::CURRENT.severity_low,
        Severity::Info => RiskFormula::CURRENT.severity_info,
    }
}

fn severity_signal(severity: Severity, score: u8) -> RiskSignal {
    signal(
        &format!("severity.{}", severity.lowercase_label()),
        "severity",
        score as i16,
        &format!("{} severity finding", severity.lowercase_label()),
    )
}

fn confidence_delta(base: u8, confidence: Confidence) -> i16 {
    let percent = match confidence {
        Confidence::High => RiskFormula::CURRENT.confidence_high_percent,
        Confidence::Medium => RiskFormula::CURRENT.confidence_medium_percent,
        Confidence::Low => RiskFormula::CURRENT.confidence_low_percent,
    };
    (((base as u32 * percent as u32) + 50) / 100) as i16 - base as i16
}

fn confidence_signal(confidence: Confidence, weight: i16) -> RiskSignal {
    signal(
        &format!("confidence.{}", confidence.lowercase_label()),
        "confidence",
        weight,
        match confidence {
            Confidence::Low => "low confidence heuristic signal",
            Confidence::Medium => "medium confidence signal",
            Confidence::High => "high confidence signal",
        },
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
