use crate::findings::contract::validate_findings_contract;
use crate::findings::types::{Confidence, Finding, Severity};
use crate::rules::{RuleLifecycle, SignalSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalQualitySummary {
    pub findings_total: usize,
    pub by_confidence: ConfidenceCounts,
    pub by_lifecycle: RuleLifecycleCounts,
    pub by_signal_source: SignalSourceCounts,
    pub evidence_coverage_percent: u8,
    pub recommendation_coverage_percent: u8,
    pub docs_coverage_for_high_risk_percent: u8,
    pub contract_violations: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfidenceCounts {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleLifecycleCounts {
    pub experimental: usize,
    pub preview: usize,
    pub stable: usize,
    pub deprecated: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalSourceCounts {
    pub text_heuristic: usize,
    pub ast: usize,
    pub config_file: usize,
    pub dependency_manifest: usize,
    pub import_graph: usize,
    pub framework_detector: usize,
    pub git_diff: usize,
    pub mixed: usize,
}

pub fn summarize_signal_quality(findings: &[Finding]) -> SignalQualitySummary {
    let contract_violations = validate_findings_contract(findings).violations.len();
    summarize_signal_quality_with_contract_violations(findings, contract_violations)
}

pub fn summarize_signal_quality_with_contract_violations(
    findings: &[Finding],
    contract_violations: usize,
) -> SignalQualitySummary {
    let findings_total = findings.len();
    let mut by_confidence = ConfidenceCounts::default();
    let mut by_lifecycle = RuleLifecycleCounts::default();
    let mut by_signal_source = SignalSourceCounts::default();
    let mut evidence_present = 0usize;
    let mut recommendation_present = 0usize;
    let mut high_risk_total = 0usize;
    let mut high_risk_docs_present = 0usize;

    for finding in findings {
        match finding.confidence {
            Confidence::Low => by_confidence.low += 1,
            Confidence::Medium => by_confidence.medium += 1,
            Confidence::High => by_confidence.high += 1,
        }
        match finding.provenance.rule_lifecycle {
            RuleLifecycle::Experimental => by_lifecycle.experimental += 1,
            RuleLifecycle::Preview => by_lifecycle.preview += 1,
            RuleLifecycle::Stable => by_lifecycle.stable += 1,
            RuleLifecycle::Deprecated => by_lifecycle.deprecated += 1,
        }
        match finding.provenance.signal_source {
            SignalSource::TextHeuristic => by_signal_source.text_heuristic += 1,
            SignalSource::Ast => by_signal_source.ast += 1,
            SignalSource::ConfigFile => by_signal_source.config_file += 1,
            SignalSource::DependencyManifest => by_signal_source.dependency_manifest += 1,
            SignalSource::ImportGraph => by_signal_source.import_graph += 1,
            SignalSource::FrameworkDetector => by_signal_source.framework_detector += 1,
            SignalSource::GitDiff => by_signal_source.git_diff += 1,
            SignalSource::Mixed => by_signal_source.mixed += 1,
        }
        if !finding.evidence.is_empty() {
            evidence_present += 1;
        }
        if !finding.recommendation.trim().is_empty() {
            recommendation_present += 1;
        }
        if matches!(finding.severity, Severity::High | Severity::Critical) {
            high_risk_total += 1;
            if finding
                .docs_url
                .as_deref()
                .is_some_and(|docs_url| !docs_url.trim().is_empty())
            {
                high_risk_docs_present += 1;
            }
        }
    }

    SignalQualitySummary {
        findings_total,
        by_confidence,
        by_lifecycle,
        by_signal_source,
        evidence_coverage_percent: coverage_percent(evidence_present, findings_total),
        recommendation_coverage_percent: coverage_percent(recommendation_present, findings_total),
        docs_coverage_for_high_risk_percent: coverage_percent_or_full(
            high_risk_docs_present,
            high_risk_total,
        ),
        contract_violations,
    }
}

fn coverage_percent(numerator: usize, denominator: usize) -> u8 {
    if denominator == 0 {
        return 100;
    }

    ((numerator * 100 + denominator / 2) / denominator).min(100) as u8
}

fn coverage_percent_or_full(numerator: usize, denominator: usize) -> u8 {
    if denominator == 0 {
        return 100;
    }
    coverage_percent(numerator, denominator)
}
