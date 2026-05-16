use crate::audits::context::{FileRole, classify_file};
use crate::baseline::diff::{BaselineStatus, FindingBaselineStatus};
use crate::findings::types::{Confidence, Finding, FindingCategory, Severity};
use crate::scan::facts::{FileFacts, ScanFacts};
use crate::scan::path_classification::is_low_signal_audit_path;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const FORMULA_VERSION: &str = "risk-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskAssessment {
    pub score: u8,
    pub priority: RiskPriority,
    pub signals: Vec<RiskSignal>,
    pub formula_version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskPriority {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskSignal {
    pub id: String,
    pub label: String,
    pub weight: i16,
    pub reason: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RiskInputs {
    pub baseline_status: Option<BaselineStatus>,
    pub in_diff: bool,
    pub workspace_hotspot: bool,
}

impl Default for RiskAssessment {
    fn default() -> Self {
        Self {
            score: 0,
            priority: RiskPriority::P3,
            signals: Vec::new(),
            formula_version: FORMULA_VERSION.to_string(),
        }
    }
}

impl RiskAssessment {
    fn new(score: u8, signals: Vec<RiskSignal>) -> Self {
        Self {
            score,
            priority: priority_for_score(score),
            signals,
            formula_version: FORMULA_VERSION.to_string(),
        }
    }
}

pub fn priority_for_score(score: u8) -> RiskPriority {
    match score {
        90..=100 => RiskPriority::P0,
        70..=89 => RiskPriority::P1,
        40..=69 => RiskPriority::P2,
        _ => RiskPriority::P3,
    }
}

impl RiskPriority {
    pub fn label(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        }
    }
}

pub fn assess_findings(findings: &mut [Finding], facts: &ScanFacts) {
    let files = file_index(facts);
    for finding in findings {
        let file = finding_file(finding, facts.root_path.as_path(), &files);
        finding.risk = assess_finding(finding, file, RiskInputs::default());
    }
}

pub fn apply_baseline_overlay(
    findings: &mut [Finding],
    statuses: &[FindingBaselineStatus],
    root: &Path,
) {
    let status_by_key = statuses
        .iter()
        .map(|status| (status.key.as_str(), status.status))
        .collect::<HashMap<_, _>>();

    for finding in findings {
        let key = crate::baseline::key::stable_finding_key(finding, root);
        if let Some(status) = status_by_key.get(key.as_str()).copied() {
            apply_overlay_signal(
                finding,
                baseline_signal(status),
                RiskInputs {
                    baseline_status: Some(status),
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_review_overlay(findings: &mut [Finding], in_diff: &[bool]) {
    for (finding, in_diff) in findings.iter_mut().zip(in_diff.iter().copied()) {
        if in_diff {
            apply_overlay_signal(
                finding,
                signal(
                    "review.in-diff",
                    "changed lines",
                    12,
                    "finding touches changed diff lines",
                ),
                RiskInputs {
                    in_diff: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn apply_workspace_hotspot_overlay(findings: &mut [Finding]) {
    let hotspot_packages = workspace_hotspots(findings);
    if hotspot_packages.is_empty() {
        return;
    }

    for finding in findings {
        if finding
            .workspace_package
            .as_ref()
            .is_some_and(|package| hotspot_packages.contains(package))
        {
            apply_overlay_signal(
                finding,
                signal(
                    "workspace.hotspot",
                    "workspace hotspot",
                    5,
                    "workspace package has multiple high-risk findings",
                ),
                RiskInputs {
                    workspace_hotspot: true,
                    ..RiskInputs::default()
                },
            );
        }
    }
}

pub fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(compare_findings);
}

pub fn compare_findings(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    right
        .risk
        .score
        .cmp(&left.risk.score)
        .then_with(|| right.severity.cmp(&left.severity))
        .then_with(|| category_rank(&left.category).cmp(&category_rank(&right.category)))
        .then_with(|| left.rule_id.cmp(&right.rule_id))
        .then_with(|| finding_location_key(left).cmp(&finding_location_key(right)))
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
        push_signal(&mut score, &mut signals, signal);
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

fn add_file_context_signals(file: &FileFacts, score: &mut i16, signals: &mut Vec<RiskSignal>) {
    let context = classify_file(file);
    if context.is_production_code() {
        push_adjustment(
            score,
            signals,
            "role.production",
            "production code",
            10,
            "production code findings are more likely to affect runtime behavior",
        );
    }

    if context.has_role(FileRole::Test) || context.has_role(FileRole::RustTest) {
        push_adjustment(
            score,
            signals,
            "role.test",
            "test code",
            -20,
            "test code often accepts patterns that would be risky in production",
        );
    }

    if context.has_role(FileRole::Generated) {
        push_adjustment(
            score,
            signals,
            "role.generated",
            "generated file",
            -30,
            "generated files should usually be fixed at the generator source",
        );
    }

    if context.has_role(FileRole::Config) {
        push_adjustment(
            score,
            signals,
            "role.config",
            "config file",
            -18,
            "configuration files often use declarative patterns that differ from application code",
        );
    }

    if is_low_signal_audit_path(&file.path)
        && !context.has_role(FileRole::Test)
        && !context.has_role(FileRole::Generated)
    {
        push_adjustment(
            score,
            signals,
            "role.low-signal-path",
            "low-signal path",
            -15,
            "fixtures, examples, and benchmark paths are lower-priority by default",
        );
    }

    if context.has_role(FileRole::AppEntrypoint) {
        push_adjustment(
            score,
            signals,
            "role.entrypoint",
            "entrypoint",
            10,
            "entrypoints can affect application startup and process behavior",
        );
    }

    if context.has_role(FileRole::FrameworkController)
        || context.has_role(FileRole::DotNetController)
    {
        push_adjustment(
            score,
            signals,
            "role.controller",
            "controller/router",
            12,
            "controllers and routers sit on user-facing request boundaries",
        );
    }

    if context.has_role(FileRole::FrameworkService) || context.has_role(FileRole::DotNetService) {
        push_adjustment(
            score,
            signals,
            "role.service",
            "service layer",
            10,
            "service-layer findings can affect reusable production behavior",
        );
    }

    if context.has_role(FileRole::Domain) {
        push_adjustment(
            score,
            signals,
            "role.domain",
            "domain model",
            8,
            "domain code usually carries core business behavior",
        );
    }

    if context.has_role(FileRole::ReactComponent) {
        push_adjustment(
            score,
            signals,
            "role.react-component",
            "React component",
            4,
            "React component findings can affect user-visible behavior",
        );
    }
}

fn apply_overlay_signal(finding: &mut Finding, signal: RiskSignal, _inputs: RiskInputs) {
    if finding
        .risk
        .signals
        .iter()
        .any(|existing| existing.id == signal.id)
    {
        return;
    }

    let score = finding.risk.score as i16 + signal.weight;
    finding.risk.signals.push(signal);
    finding.risk.score = clamp_score(score);
    finding.risk.priority = priority_for_score(finding.risk.score);
    finding.risk.formula_version = FORMULA_VERSION.to_string();
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

fn baseline_signal(status: BaselineStatus) -> RiskSignal {
    match status {
        BaselineStatus::New => signal(
            "baseline.new",
            "new finding",
            10,
            "new findings should be prioritized over accepted existing debt",
        ),
        BaselineStatus::Existing => signal(
            "baseline.existing",
            "existing finding",
            -8,
            "existing baseline findings are already accepted technical debt",
        ),
    }
}

fn push_adjustment(
    score: &mut i16,
    signals: &mut Vec<RiskSignal>,
    id: &str,
    label: &str,
    weight: i16,
    reason: &str,
) {
    push_signal(score, signals, signal(id, label, weight, reason));
}

fn push_signal(score: &mut i16, signals: &mut Vec<RiskSignal>, signal: RiskSignal) {
    *score += signal.weight;
    signals.push(signal);
}

fn signal(id: &str, label: &str, weight: i16, reason: &str) -> RiskSignal {
    RiskSignal {
        id: id.to_string(),
        label: label.to_string(),
        weight,
        reason: reason.to_string(),
    }
}

fn clamp_score(score: i16) -> u8 {
    score.clamp(0, 100) as u8
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

fn workspace_hotspots(findings: &[Finding]) -> HashSet<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for finding in findings {
        if finding.severity >= Severity::High
            && let Some(package) = &finding.workspace_package
        {
            *counts.entry(package.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(package, count)| (count > 1).then_some(package))
        .collect()
}

fn category_rank(category: &FindingCategory) -> usize {
    match category {
        FindingCategory::Security => 0,
        FindingCategory::Architecture => 1,
        FindingCategory::Framework => 2,
        FindingCategory::CodeQuality => 3,
        FindingCategory::Testing => 4,
    }
}

fn finding_location_key(finding: &Finding) -> String {
    finding
        .evidence
        .first()
        .map(|evidence| format!("{}:{}", evidence.path.display(), evidence.line_start))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::{Evidence, Finding};

    #[test]
    fn priority_thresholds_match_v1_plan() {
        assert_eq!(priority_for_score(90), RiskPriority::P0);
        assert_eq!(priority_for_score(70), RiskPriority::P1);
        assert_eq!(priority_for_score(40), RiskPriority::P2);
        assert_eq!(priority_for_score(39), RiskPriority::P3);
    }

    #[test]
    fn high_confidence_critical_score_clamps_to_100() {
        let finding = Finding {
            severity: Severity::Critical,
            confidence: Confidence::High,
            ..Finding::default()
        };

        let assessment = assess_finding(&finding, None, RiskInputs::default());

        assert_eq!(assessment.score, 100);
        assert_eq!(assessment.priority, RiskPriority::P0);
    }

    #[test]
    fn production_domain_file_scores_above_equivalent_test_file() {
        let finding = Finding {
            severity: Severity::Medium,
            confidence: Confidence::Medium,
            evidence: vec![Evidence {
                path: PathBuf::from("src/domain/user.rs"),
                line_start: 1,
                line_end: None,
                snippet: String::new(),
            }],
            ..Finding::default()
        };
        let production = file("src/domain/user.rs", Some("Rust"), false);
        let test = file("tests/user.rs", Some("Rust"), false);

        let prod_risk = assess_finding(&finding, Some(&production), RiskInputs::default());
        let test_risk = assess_finding(&finding, Some(&test), RiskInputs::default());

        assert!(prod_risk.score > test_risk.score);
        assert!(prod_risk.signals.iter().any(|s| s.id == "role.domain"));
        assert!(test_risk.signals.iter().any(|s| s.id == "role.test"));
    }

    fn file(path: &str, language: Option<&str>, has_inline_tests: bool) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: language.map(str::to_string),
            lines_of_code: 10,
            branch_count: 0,
            imports: Vec::new(),
            content: None,
            has_inline_tests,
        }
    }
}
