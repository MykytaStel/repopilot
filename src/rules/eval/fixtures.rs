use crate::findings::contract::validate_findings_contract;
use crate::findings::types::{Finding, Severity};
use crate::rules::eval::{RuleEvaluationReport, RuleEvaluationRuleReport};
use crate::rules::{RuleLifecycle, all_rule_metadata, lookup_rule_metadata};
use crate::scan::config::ScanConfig;
use crate::scan::scanner::scan_path_with_config;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct FixtureExpectations {
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    path: PathBuf,
    expected_rule_ids: Vec<String>,
}

pub fn evaluate_rule_fixtures(
    only_rule: Option<&str>,
    fixture_root: Option<&Path>,
) -> Result<RuleEvaluationReport, Box<dyn std::error::Error>> {
    let fixture_root = fixture_root
        .map(Path::to_path_buf)
        .unwrap_or_else(default_fixture_root);

    if !fixture_root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Rule fixture root does not exist: {}",
                fixture_root.display()
            ),
        )
        .into());
    }

    let fixture_dirs = discover_rule_fixture_dirs(&fixture_root)?;
    let rule_ids = rule_ids_to_evaluate(only_rule, &fixture_dirs);

    let mut report = RuleEvaluationReport::default();

    for rule_id in rule_ids {
        if let Some(rule_root) = fixture_dirs.get(&rule_id) {
            report.add_rule(evaluate_one_rule_fixture(rule_root, &rule_id)?);
        } else {
            report.add_rule(evaluate_missing_rule_fixture(&rule_id));
        }
    }

    Ok(report)
}

fn discover_rule_fixture_dirs(
    fixture_root: &Path,
) -> Result<BTreeMap<String, PathBuf>, Box<dyn std::error::Error>> {
    let mut fixture_dirs = BTreeMap::new();
    let entries = fs::read_dir(fixture_root)?.collect::<Result<Vec<_>, _>>()?;

    for entry in entries {
        if !entry.file_type()?.is_dir() {
            continue;
        }
        fixture_dirs.insert(
            entry.file_name().to_string_lossy().to_string(),
            entry.path(),
        );
    }

    Ok(fixture_dirs)
}

fn rule_ids_to_evaluate(
    only_rule: Option<&str>,
    fixture_dirs: &BTreeMap<String, PathBuf>,
) -> Vec<String> {
    if let Some(rule_id) = only_rule {
        return vec![rule_id.to_string()];
    }

    let mut rule_ids = fixture_dirs.keys().cloned().collect::<BTreeSet<_>>();
    rule_ids.extend(
        all_rule_metadata()
            .filter(|rule| rule.lifecycle == RuleLifecycle::Stable)
            .map(|rule| rule.rule_id.to_string()),
    );
    rule_ids.into_iter().collect()
}

fn evaluate_missing_rule_fixture(rule_id: &str) -> RuleEvaluationRuleReport {
    let mut report = RuleEvaluationRuleReport {
        rule_id: rule_id.to_string(),
        ..RuleEvaluationRuleReport::default()
    };
    apply_rule_quality_gate(rule_id, &mut report);
    report
}

fn evaluate_one_rule_fixture(
    rule_root: &Path,
    rule_id: &str,
) -> Result<RuleEvaluationRuleReport, Box<dyn std::error::Error>> {
    let expected_path = rule_root.join("expected.json");
    let expectations: FixtureExpectations =
        serde_json::from_str(&fs::read_to_string(expected_path)?)?;
    // Mirror a real default scan (`detect_missing_tests` is on by default) so the
    // testing audits are exercisable as fixtures; `include_low_signal` keeps the
    // Low-severity heuristic rules visible. Per-rule expectations filter findings
    // by `rule_id`, so enabling these audits cannot affect other rules' fixtures.
    let config = ScanConfig {
        detect_missing_tests: true,
        include_low_signal: true,
        ..ScanConfig::default()
    };
    let mut report = RuleEvaluationRuleReport {
        rule_id: rule_id.to_string(),
        ..RuleEvaluationRuleReport::default()
    };

    for fixture in expectations.fixtures {
        let path = rule_root.join(&fixture.path);
        let scan_path = materialize_fixture_for_scan(&path)?;
        let summary = scan_path_with_config(&scan_path, &config)?;
        cleanup_materialized_fixture(&scan_path, &path);
        let actual_rule_ids = summary
            .artifacts
            .findings
            .iter()
            .filter(|finding| finding.rule_id == rule_id)
            .map(|finding| finding.rule_id.clone())
            .collect::<Vec<_>>();
        let expected = fixture
            .expected_rule_ids
            .iter()
            .filter(|expected_rule| expected_rule.as_str() == rule_id)
            .cloned()
            .collect::<Vec<_>>();

        if expected.is_empty() {
            report.has_false_positive_fixture = true;
        } else {
            report.has_true_positive_fixture = true;
        }
        report.fixtures_total += 1;
        report.expected_findings += expected.len();
        report.actual_findings += actual_rule_ids.len();
        report.missing_findings += missing_count(&expected, &actual_rule_ids);
        report.unexpected_findings += missing_count(&actual_rule_ids, &expected);
        report.contract_violations += validate_findings_contract(&summary.artifacts.findings)
            .violations
            .len();

        for finding in &summary.artifacts.findings {
            if let Some(metadata) = crate::rules::lookup_rule_metadata(&finding.rule_id) {
                if finding.severity > metadata.severity_ceiling() {
                    report.contract_violations += 1;
                }
                if !metadata.contextual_confidence {
                    if finding.confidence != metadata.default_confidence {
                        report.contract_violations += 1;
                    }
                } else if finding.confidence > metadata.confidence_ceiling() {
                    report.contract_violations += 1;
                }
            }
        }

        if has_stable_id_failure(&summary.artifacts.findings) {
            report.stable_id_failures += 1;
        }
    }

    apply_rule_quality_gate(rule_id, &mut report);

    Ok(report)
}

fn apply_rule_quality_gate(rule_id: &str, report: &mut RuleEvaluationRuleReport) {
    let Some(metadata) = lookup_rule_metadata(rule_id) else {
        return;
    };

    if metadata.lifecycle != RuleLifecycle::Stable {
        return;
    }

    let mut failures = 0usize;

    if !report.has_true_positive_fixture {
        failures += 1;
    }
    if !report.has_false_positive_fixture {
        failures += 1;
    }
    if report.missing_findings > 0 {
        failures += 1;
    }
    if report.unexpected_findings > 0 {
        failures += 1;
    }
    if report.contract_violations > 0 {
        failures += 1;
    }
    if report.stable_id_failures > 0 {
        failures += 1;
    }
    if metadata.default_severity >= Severity::High && metadata.docs_url.is_none() {
        failures += 1;
    }
    if metadata
        .false_positive_notes
        .is_none_or(|notes| notes.trim().is_empty())
    {
        failures += 1;
    }

    report.quality_gate_failures = failures;
}

fn materialize_fixture_for_scan(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path_text = path.to_string_lossy().to_lowercase();
    if !path_text.contains("fixture") {
        return Ok(path.to_path_buf());
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let target = std::env::temp_dir().join(format!(
        "repopilot-rule-eval-{}-{nanos}",
        std::process::id()
    ));
    copy_dir_all(path, &target)?;
    Ok(target)
}

fn cleanup_materialized_fixture(scan_path: &Path, original_path: &Path) {
    if scan_path != original_path {
        let _ = fs::remove_dir_all(scan_path);
    }
}

fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn missing_count(expected: &[String], actual: &[String]) -> usize {
    let mut counts = BTreeMap::<&str, usize>::new();
    for item in actual {
        *counts.entry(item.as_str()).or_default() += 1;
    }

    expected
        .iter()
        .filter(|item| {
            let count = counts.entry(item.as_str()).or_default();
            if *count == 0 {
                true
            } else {
                *count -= 1;
                false
            }
        })
        .count()
}

fn has_stable_id_failure(findings: &[Finding]) -> bool {
    let mut seen = BTreeSet::new();
    findings
        .iter()
        .any(|finding| finding.id.trim().is_empty() || !seen.insert(finding.id.as_str()))
}

fn default_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules")
}
