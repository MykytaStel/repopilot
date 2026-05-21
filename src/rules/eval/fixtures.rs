use crate::findings::contract::validate_findings_contract;
use crate::findings::types::Finding;
use crate::rules::eval::{RuleEvaluationReport, RuleEvaluationRuleReport};
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

    let mut report = RuleEvaluationReport::default();
    let mut entries = fs::read_dir(&fixture_root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let rule_id = entry.file_name().to_string_lossy().to_string();
        if only_rule.is_some_and(|only_rule| only_rule != rule_id) {
            continue;
        }

        report.add_rule(evaluate_one_rule_fixture(&entry.path(), &rule_id)?);
    }

    Ok(report)
}

fn evaluate_one_rule_fixture(
    rule_root: &Path,
    rule_id: &str,
) -> Result<RuleEvaluationRuleReport, Box<dyn std::error::Error>> {
    let expected_path = rule_root.join("expected.json");
    let expectations: FixtureExpectations =
        serde_json::from_str(&fs::read_to_string(expected_path)?)?;
    let config = ScanConfig {
        detect_missing_tests: false,
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

        report.fixtures_total += 1;
        report.expected_findings += expected.len();
        report.actual_findings += actual_rule_ids.len();
        report.missing_findings += missing_count(&expected, &actual_rule_ids);
        report.unexpected_findings += missing_count(&actual_rule_ids, &expected);
        report.contract_violations += validate_findings_contract(&summary.findings)
            .violations
            .len();

        if has_stable_id_failure(&summary.findings) {
            report.stable_id_failures += 1;
        }
    }

    Ok(report)
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
