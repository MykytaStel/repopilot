use crate::cli::{CompareOutputFormatArg, RuleLifecycleArg, SignalSourceArg};
use crate::commands::{CliExit, EXIT_RUNTIME, EXIT_USAGE};
use repopilot::findings::contract::validate_findings_contract;
use repopilot::output::OutputFormat;
use repopilot::report::writer::write_report;
use repopilot::rules::{RuleLifecycle, RuleMetadata, SignalSource, all_rule_metadata};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct RuleCatalogReport {
    rules: Vec<RuleCatalogItem>,
}

#[derive(Debug, Serialize)]
struct RuleCatalogItem {
    rule_id: &'static str,
    title: &'static str,
    category: String,
    severity: String,
    confidence: String,
    lifecycle: RuleLifecycle,
    signal_source: SignalSource,
    docs_url: Option<&'static str>,
    tags: &'static [&'static str],
    description: &'static str,
    recommendation: Option<&'static str>,
    false_positive_notes: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct RuleEvaluationReport {
    pub rules_evaluated: usize,
    pub fixtures_total: usize,
    pub expected_findings: usize,
    pub actual_findings: usize,
    pub missing_findings: usize,
    pub unexpected_findings: usize,
    pub contract_violations: usize,
    pub stable_id_failures: usize,
}

#[derive(Debug, Deserialize)]
struct FixtureExpectations {
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    path: PathBuf,
    expected_rule_ids: Vec<String>,
}

pub fn list_rules(
    format: CompareOutputFormatArg,
    lifecycle: Option<RuleLifecycleArg>,
    source: Option<SignalSourceArg>,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let lifecycle = lifecycle.map(RuleLifecycle::from);
    let source = source.map(SignalSource::from);
    let rules = all_rule_metadata()
        .filter(|rule| lifecycle.is_none_or(|value| rule.lifecycle == value))
        .filter(|rule| source.is_none_or(|value| rule.signal_source == value))
        .map(RuleCatalogItem::from)
        .collect::<Vec<_>>();
    let report = RuleCatalogReport { rules };
    let rendered = render_catalog(&report, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

pub fn inspect_rule(
    rule_id: String,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(rule) = repopilot::rules::lookup_rule_metadata(&rule_id) else {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: format!("Unknown RepoPilot rule `{rule_id}`"),
        }));
    };

    let report = RuleCatalogItem::from(rule);
    let rendered = render_rule(&report, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

pub fn eval_rules(
    rule: Option<String>,
    fixtures: Option<PathBuf>,
    format: CompareOutputFormatArg,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(rule_id) = &rule
        && repopilot::rules::lookup_rule_metadata(rule_id).is_none()
    {
        return Err(Box::new(CliExit {
            code: EXIT_USAGE,
            message: format!("Unknown RepoPilot rule `{rule_id}`"),
        }));
    }

    let report = evaluate_rule_fixtures(rule.as_deref(), fixtures.as_deref())?;
    let rendered = render_eval_report(&report, OutputFormat::from(format))?;
    write_report(&rendered, output.as_deref())?;
    Ok(())
}

fn render_catalog(
    report: &RuleCatalogReport,
    format: OutputFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match validate_output_format(format)? {
        OutputFormat::Console => Ok(render_catalog_console(report)),
        OutputFormat::Markdown => Ok(render_catalog_markdown(report)),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
    }
}

fn render_rule(
    rule: &RuleCatalogItem,
    format: OutputFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match validate_output_format(format)? {
        OutputFormat::Console => Ok(render_rule_console(rule)),
        OutputFormat::Markdown => Ok(render_rule_markdown(rule)),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(rule)?),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
    }
}

fn render_eval_report(
    report: &RuleEvaluationReport,
    format: OutputFormat,
) -> Result<String, Box<dyn std::error::Error>> {
    match validate_output_format(format)? {
        OutputFormat::Console => Ok(format!(
            "RepoPilot Rule Evaluation\n\nRules evaluated: {}\nFixtures: {}\nExpected findings: {}\nActual findings: {}\nMissing findings: {}\nUnexpected findings: {}\nContract violations: {}\nStable ID failures: {}\n",
            report.rules_evaluated,
            report.fixtures_total,
            report.expected_findings,
            report.actual_findings,
            report.missing_findings,
            report.unexpected_findings,
            report.contract_violations,
            report.stable_id_failures,
        )),
        OutputFormat::Markdown => Ok(format!(
            "# RepoPilot Rule Evaluation\n\n- **Rules evaluated:** {}\n- **Fixtures:** {}\n- **Expected findings:** {}\n- **Actual findings:** {}\n- **Missing findings:** {}\n- **Unexpected findings:** {}\n- **Contract violations:** {}\n- **Stable ID failures:** {}\n",
            report.rules_evaluated,
            report.fixtures_total,
            report.expected_findings,
            report.actual_findings,
            report.missing_findings,
            report.unexpected_findings,
            report.contract_violations,
            report.stable_id_failures,
        )),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
    }
}

fn render_catalog_console(report: &RuleCatalogReport) -> String {
    let mut output = String::new();
    output.push_str("RepoPilot Rules\n\n");
    for rule in &report.rules {
        output.push_str(&format!(
            "{} [{} {} {}]\n  {}\n",
            rule.rule_id,
            rule.severity,
            rule.lifecycle.label(),
            rule.signal_source.label(),
            rule.title
        ));
    }
    output
}

fn render_catalog_markdown(report: &RuleCatalogReport) -> String {
    let mut output = String::new();
    output.push_str("# RepoPilot Rules\n\n");
    output.push_str("| Rule | Title | Category | Severity | Confidence | Lifecycle | Source |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for rule in &report.rules {
        output.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} |\n",
            rule.rule_id,
            escape_table_cell(rule.title),
            rule.category,
            rule.severity,
            rule.confidence,
            rule.lifecycle.label(),
            rule.signal_source.label()
        ));
    }
    output
}

fn render_rule_console(rule: &RuleCatalogItem) -> String {
    format!(
        "RepoPilot Rule\n\nRule: {}\nTitle: {}\nCategory: {}\nSeverity: {}\nConfidence: {}\nLifecycle: {}\nSignal source: {}\nDocs: {}\nTags: {}\nDescription: {}\nRecommendation: {}\nFalse positives: {}\n",
        rule.rule_id,
        rule.title,
        rule.category,
        rule.severity,
        rule.confidence,
        rule.lifecycle.label(),
        rule.signal_source.label(),
        rule.docs_url.unwrap_or("none"),
        if rule.tags.is_empty() {
            "none".to_string()
        } else {
            rule.tags.join(", ")
        },
        rule.description,
        rule.recommendation.unwrap_or("none"),
        rule.false_positive_notes.unwrap_or("none"),
    )
}

fn render_rule_markdown(rule: &RuleCatalogItem) -> String {
    format!(
        "# `{}`\n\n- **Title:** {}\n- **Category:** {}\n- **Severity:** {}\n- **Confidence:** {}\n- **Lifecycle:** {}\n- **Signal source:** {}\n- **Docs:** {}\n- **Tags:** {}\n\n{}\n\n**Recommendation:** {}\n\n**False-positive notes:** {}\n",
        rule.rule_id,
        rule.title,
        rule.category,
        rule.severity,
        rule.confidence,
        rule.lifecycle.label(),
        rule.signal_source.label(),
        rule.docs_url.unwrap_or("none"),
        if rule.tags.is_empty() {
            "none".to_string()
        } else {
            rule.tags.join(", ")
        },
        rule.description,
        rule.recommendation.unwrap_or("none"),
        rule.false_positive_notes.unwrap_or("none"),
    )
}

fn evaluate_rule_fixtures(
    only_rule: Option<&str>,
    fixture_root: Option<&Path>,
) -> Result<RuleEvaluationReport, Box<dyn std::error::Error>> {
    let fixture_root = fixture_root
        .map(Path::to_path_buf)
        .unwrap_or_else(default_fixture_root);

    if !fixture_root.exists() {
        return Err(Box::new(CliExit {
            code: EXIT_RUNTIME,
            message: format!(
                "Rule fixture root does not exist: {}",
                fixture_root.display()
            ),
        }));
    }

    let mut rules_evaluated = BTreeSet::new();
    let mut report = RuleEvaluationReport {
        rules_evaluated: 0,
        fixtures_total: 0,
        expected_findings: 0,
        actual_findings: 0,
        missing_findings: 0,
        unexpected_findings: 0,
        contract_violations: 0,
        stable_id_failures: 0,
    };

    for entry in fs::read_dir(&fixture_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let rule_id = entry.file_name().to_string_lossy().to_string();
        if only_rule.is_some_and(|only_rule| only_rule != rule_id) {
            continue;
        }
        rules_evaluated.insert(rule_id.clone());
        evaluate_one_rule_fixture(&entry.path(), &rule_id, &mut report)?;
    }

    report.rules_evaluated = rules_evaluated.len();
    Ok(report)
}

fn evaluate_one_rule_fixture(
    rule_root: &Path,
    rule_id: &str,
    report: &mut RuleEvaluationReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected_path = rule_root.join("expected.json");
    let expectations: FixtureExpectations =
        serde_json::from_str(&fs::read_to_string(expected_path)?)?;
    let config = ScanConfig {
        detect_missing_tests: false,
        include_low_signal: true,
        ..ScanConfig::default()
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

    Ok(())
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

fn has_stable_id_failure(findings: &[repopilot::findings::types::Finding]) -> bool {
    let mut seen = BTreeSet::new();
    findings
        .iter()
        .any(|finding| finding.id.trim().is_empty() || !seen.insert(finding.id.as_str()))
}

fn default_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules")
}

fn validate_output_format(format: OutputFormat) -> Result<OutputFormat, CliExit> {
    match format {
        OutputFormat::Console | OutputFormat::Json | OutputFormat::Markdown => Ok(format),
        OutputFormat::Html | OutputFormat::Sarif => Err(CliExit {
            code: EXIT_USAGE,
            message: "`inspect rules` supports only console, markdown, and json output".to_string(),
        }),
    }
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

impl From<&'static RuleMetadata> for RuleCatalogItem {
    fn from(rule: &'static RuleMetadata) -> Self {
        Self {
            rule_id: rule.rule_id,
            title: rule.title,
            category: rule.category.label().to_string(),
            severity: rule.default_severity.label().to_string(),
            confidence: rule.default_confidence.label().to_string(),
            lifecycle: rule.lifecycle,
            signal_source: rule.signal_source,
            docs_url: rule.docs_url,
            tags: rule.tags,
            description: rule.description,
            recommendation: rule.recommendation,
            false_positive_notes: rule.false_positive_notes,
        }
    }
}
