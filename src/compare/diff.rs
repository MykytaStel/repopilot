use crate::findings::types::{Finding, Severity};
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
pub struct CompareSummary {
    pub new_findings: Vec<Finding>,
    pub resolved_findings: Vec<Finding>,
    pub severity_increased: Vec<(Finding, Severity)>,
    pub before_files: usize,
    pub after_files: usize,
    pub before_loc: usize,
    pub after_loc: usize,
}

pub fn diff_summaries(before: &ScanSummary, after: &ScanSummary) -> CompareSummary {
    let before_map: HashMap<String, &Finding> = before
        .findings
        .iter()
        .map(|f| (stable_finding_key(f), f))
        .collect();

    let after_map: HashMap<String, &Finding> = after
        .findings
        .iter()
        .map(|f| (stable_finding_key(f), f))
        .collect();

    let before_ids: HashSet<String> = before_map.keys().cloned().collect();
    let after_ids: HashSet<String> = after_map.keys().cloned().collect();

    let new_findings = after_ids
        .difference(&before_ids)
        .map(|id| after_map[id].clone())
        .collect();

    let resolved_findings = before_ids
        .difference(&after_ids)
        .map(|id| before_map[id].clone())
        .collect();

    let severity_increased = after_ids
        .intersection(&before_ids)
        .filter_map(|id| {
            let before_f = before_map[id];
            let after_f = after_map[id];
            if severity_rank(&after_f.severity) > severity_rank(&before_f.severity) {
                Some((after_f.clone(), before_f.severity.clone()))
            } else {
                None
            }
        })
        .collect();

    CompareSummary {
        new_findings,
        resolved_findings,
        severity_increased,
        before_files: before.files_count,
        after_files: after.files_count,
        before_loc: before.lines_of_code,
        after_loc: after.lines_of_code,
    }
}

fn stable_finding_key(finding: &Finding) -> String {
    finding
        .evidence
        .first()
        .map(|evidence| {
            format!(
                "{}:{}:{}",
                finding.rule_id,
                evidence.path.display(),
                evidence.line_start
            )
        })
        .unwrap_or_else(|| finding.id.clone())
}

fn severity_rank(s: &Severity) -> u8 {
    match s {
        Severity::Info => 0,
        Severity::Low => 1,
        Severity::Medium => 2,
        Severity::High => 3,
        Severity::Critical => 4,
    }
}
