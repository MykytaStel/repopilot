use crate::history::model::{
    ComparisonProvenance, ComparisonResult, ComparisonUnavailable, FindingReceipt, RiskDelta,
    RunReceipt, SeverityShift,
};
use std::collections::BTreeMap;

pub fn compare(current: &RunReceipt, prior: &RunReceipt) -> ComparisonResult {
    if current.schema_version != prior.schema_version {
        return ComparisonResult::Unavailable(ComparisonUnavailable::SchemaMismatch);
    }
    if let Some(reason) = incompatible_identity(current, prior) {
        return ComparisonResult::Unavailable(reason);
    }
    ComparisonResult::Compatible(compute_compatible_delta(current, prior))
}

fn incompatible_identity(
    current: &RunReceipt,
    prior: &RunReceipt,
) -> Option<ComparisonUnavailable> {
    let current = &current.comparison;
    let prior = &prior.comparison;
    if current.workspace != prior.workspace {
        Some(ComparisonUnavailable::WorkspaceMismatch)
    } else if current.analysis_target != prior.analysis_target {
        Some(ComparisonUnavailable::TargetMismatch)
    } else if current.scope != prior.scope {
        Some(ComparisonUnavailable::ScopeMismatch)
    } else if current.base_revision != prior.base_revision
        || current.head_revision != prior.head_revision
    {
        Some(ComparisonUnavailable::RevisionRangeMismatch)
    } else if current.profile != prior.profile {
        Some(ComparisonUnavailable::ProfileMismatch)
    } else if current.config_fingerprint != prior.config_fingerprint {
        Some(ComparisonUnavailable::ConfigMismatch)
    } else if current.selection_fingerprint != prior.selection_fingerprint {
        Some(ComparisonUnavailable::SelectionMismatch)
    } else if current.overlay_fingerprint != prior.overlay_fingerprint {
        Some(ComparisonUnavailable::OverlayMismatch)
    } else if current.analysis_schema != prior.analysis_schema {
        Some(ComparisonUnavailable::AnalysisSchemaMismatch)
    } else {
        None
    }
}

fn compute_compatible_delta(current: &RunReceipt, prior: &RunReceipt) -> RiskDelta {
    let current_by_occurrence = by_occurrence(&current.findings);
    let prior_by_occurrence = by_occurrence(&prior.findings);
    let mut delta = RiskDelta {
        comparison: ComparisonProvenance {
            prior_revision: prior.revision.clone(),
            current_revision: current.revision.clone(),
        },
        ..RiskDelta::default()
    };

    for (key, current_finding) in &current_by_occurrence {
        match prior_by_occurrence.get(key) {
            Some(prior_finding) => {
                delta.persisting_findings.push((*current_finding).clone());
                if prior_finding.severity != current_finding.severity {
                    delta.severity_shifts.push(SeverityShift {
                        occurrence_key: key.clone(),
                        rule_id: current_finding.rule_id.clone(),
                        path: current_finding.path.clone(),
                        old_severity: prior_finding.severity,
                        new_severity: current_finding.severity,
                    });
                }
            }
            None => delta.new_findings.push((*current_finding).clone()),
        }
    }

    for (key, prior_finding) in &prior_by_occurrence {
        if !current_by_occurrence.contains_key(key) {
            delta.resolved_findings.push((*prior_finding).clone());
        }
    }
    delta
}

fn by_occurrence(findings: &[FindingReceipt]) -> BTreeMap<String, &FindingReceipt> {
    findings
        .iter()
        .map(|finding| (finding.occurrence_key.clone(), finding))
        .collect()
}

impl RiskDelta {
    pub fn render_console(&self) -> String {
        if !self.has_changes() {
            return format!(
                "Risk Delta: No finding changes; {} occurrence(s) persist.\n",
                self.persisting_findings.len()
            );
        }

        let mut out = String::from("Risk Delta (vs compatible previous run):\n");
        render_console_findings(&mut out, "+", "new", &self.new_findings);
        render_console_findings(&mut out, "-", "resolved", &self.resolved_findings);
        if !self.severity_shifts.is_empty() {
            out.push_str(&format!(
                "  ~ {} severity shift(s)\n",
                self.severity_shifts.len()
            ));
            for item in self.severity_shifts.iter().take(5) {
                out.push_str(&format!(
                    "    - {} ({}): {:?} -> {:?}\n",
                    item.rule_id, item.path, item.old_severity, item.new_severity
                ));
            }
        }
        if !self.persisting_findings.is_empty() {
            out.push_str(&format!(
                "  = {} persisting finding occurrence(s)\n",
                self.persisting_findings.len()
            ));
        }
        out
    }

    pub fn render_markdown(&self) -> String {
        let mut out = String::from("### Risk Delta\n\n");
        if !self.has_changes() {
            out.push_str(&format!(
                "No changes; {} finding occurrence(s) persist.\n",
                self.persisting_findings.len()
            ));
            return out;
        }
        render_markdown_findings(&mut out, "New", &self.new_findings);
        render_markdown_findings(&mut out, "Resolved", &self.resolved_findings);
        if !self.severity_shifts.is_empty() {
            out.push_str(&format!(
                "* **{} Severity Shift(s)**\n",
                self.severity_shifts.len()
            ));
        }
        out
    }
}

fn render_console_findings(
    out: &mut String,
    marker: &str,
    label: &str,
    findings: &[FindingReceipt],
) {
    if findings.is_empty() {
        return;
    }
    out.push_str(&format!(
        "  {marker} {} {label} finding occurrence(s)\n",
        findings.len()
    ));
    for item in findings.iter().take(5) {
        out.push_str(&format!(
            "    - [{:?}] {} ({})\n",
            item.severity, item.rule_id, item.path
        ));
    }
}

fn render_markdown_findings(out: &mut String, label: &str, findings: &[FindingReceipt]) {
    if findings.is_empty() {
        return;
    }
    out.push_str(&format!("* **{} {label} Finding(s)**\n", findings.len()));
    for item in findings {
        out.push_str(&format!(
            "  * `{:?}` `{}` ({})\n",
            item.severity, item.rule_id, item.path
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::Severity;
    use crate::history::model::{AnalysisScope, ComparisonIdentity};

    #[test]
    fn distinct_occurrences_with_one_baseline_id_are_not_merged() {
        let comparison = comparison(AnalysisScope::Full);
        let prior = receipt(
            comparison.clone(),
            vec![finding("same-id", "occ-a", Severity::Medium)],
        );
        let current = receipt(
            comparison,
            vec![
                finding("same-id", "occ-a", Severity::Medium),
                finding("same-id", "occ-b", Severity::High),
            ],
        );

        let ComparisonResult::Compatible(delta) = compare(&current, &prior) else {
            panic!("matching contracts must compare");
        };
        assert_eq!(delta.persisting_findings.len(), 1);
        assert_eq!(delta.new_findings.len(), 1);
    }

    #[test]
    fn changed_scope_never_resolves_full_scope_occurrences() {
        let full = receipt(
            comparison(AnalysisScope::Full),
            vec![finding("a", "occ-a", Severity::High)],
        );
        let changed = receipt(comparison(AnalysisScope::Changed), Vec::new());
        assert_eq!(
            compare(&changed, &full),
            ComparisonResult::Unavailable(ComparisonUnavailable::ScopeMismatch)
        );
    }

    #[test]
    fn compatible_occurrence_reports_severity_shift() {
        let comparison = comparison(AnalysisScope::Full);
        let prior = receipt(
            comparison.clone(),
            vec![finding("a", "occ-a", Severity::Medium)],
        );
        let current = receipt(comparison, vec![finding("a", "occ-a", Severity::High)]);
        let ComparisonResult::Compatible(delta) = compare(&current, &prior) else {
            panic!("matching contracts must compare");
        };
        assert_eq!(delta.persisting_findings.len(), 1);
        assert_eq!(delta.severity_shifts.len(), 1);
    }

    fn comparison(scope: AnalysisScope) -> ComparisonIdentity {
        ComparisonIdentity {
            workspace: "/repo".to_string(),
            analysis_target: ".".to_string(),
            scope,
            base_revision: None,
            head_revision: Some("head".to_string()),
            profile: "default".to_string(),
            config_fingerprint: "config".to_string(),
            selection_fingerprint: "selection".to_string(),
            overlay_fingerprint: "overlay".to_string(),
            analysis_schema: "scan-report-v0.23".to_string(),
        }
    }

    fn receipt(comparison: ComparisonIdentity, findings: Vec<FindingReceipt>) -> RunReceipt {
        RunReceipt::new(
            comparison,
            "2026-07-26T12:00:00Z".to_string(),
            "head".to_string(),
            findings,
        )
    }

    fn finding(baseline_id: &str, occurrence_key: &str, severity: Severity) -> FindingReceipt {
        FindingReceipt {
            baseline_id: baseline_id.to_string(),
            occurrence_key: occurrence_key.to_string(),
            rule_id: "rule.test".to_string(),
            severity,
            path: "src/main.rs".to_string(),
        }
    }
}
