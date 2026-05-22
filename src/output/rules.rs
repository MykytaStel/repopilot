use crate::output::OutputFormat;
use crate::output::render_helpers::escape_table_cell;
use crate::rules::catalog::{RuleCatalogItem, RuleCatalogReport};
use crate::rules::eval::RuleEvaluationReport;
use std::fmt::Write;

pub fn render_catalog(
    report: &RuleCatalogReport,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_catalog_console(report)),
        OutputFormat::Markdown => Ok(render_catalog_markdown(report)),
        OutputFormat::Json => serde_json::to_string_pretty(report),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
    }
}

pub fn render_rule(
    rule: &RuleCatalogItem,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_rule_console(rule)),
        OutputFormat::Markdown => Ok(render_rule_markdown(rule)),
        OutputFormat::Json => serde_json::to_string_pretty(rule),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
    }
}

pub fn render_eval_report(
    report: &RuleEvaluationReport,
    format: OutputFormat,
) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Console => Ok(render_eval_report_console(report)),
        OutputFormat::Markdown => Ok(render_eval_report_markdown(report)),
        OutputFormat::Json => serde_json::to_string_pretty(report),
        OutputFormat::Html | OutputFormat::Sarif => unreachable!("validated output format"),
    }
}

fn render_catalog_console(report: &RuleCatalogReport) -> String {
    let mut output = String::new();
    output.push_str("RepoPilot Rules\n\n");
    for rule in &report.rules {
        writeln!(
            output,
            "{} [{} {} {} {}]\n  {}",
            rule.rule_id,
            rule.severity,
            rule.lifecycle.label(),
            rule.signal_source.label(),
            rule.stability_gate_status,
            rule.title
        )
        .unwrap();
    }
    output
}

fn render_catalog_markdown(report: &RuleCatalogReport) -> String {
    let mut output = String::new();
    output.push_str("# RepoPilot Rules\n\n");
    output.push_str(
        "| Rule | Title | Category | Severity | Confidence | Lifecycle | Source | Quality gate |\n",
    );
    output.push_str("| --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for rule in &report.rules {
        writeln!(
            output,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
            rule.rule_id,
            escape_table_cell(rule.title),
            rule.category,
            rule.severity,
            rule.confidence,
            rule.lifecycle.label(),
            rule.signal_source.label(),
            rule.stability_gate_status
        )
        .unwrap();
    }
    output
}

fn render_rule_console(rule: &RuleCatalogItem) -> String {
    format!(
        "RepoPilot Rule\n\nRule: {}\nTitle: {}\nCategory: {}\nSeverity: {}\nConfidence: {}\nLifecycle: {}\nSignal source: {}\nSemantic source: {}\nRequired scope: {}\nFixture coverage: {} fixture(s), true-positive {}, false-positive {}\nFalse-positive risk: {}\nStability gate: {}\nDocs: {}\nTags: {}\nDescription: {}\nRecommendation: {}\nFalse positives: {}\n",
        rule.rule_id,
        rule.title,
        rule.category,
        rule.severity,
        rule.confidence,
        rule.lifecycle.label(),
        rule.signal_source.label(),
        rule.semantic_source,
        rule.required_scope,
        rule.fixture_coverage.fixtures_total,
        rule.fixture_coverage.has_true_positive_fixture,
        rule.fixture_coverage.has_false_positive_fixture,
        rule.false_positive_risk,
        rule.stability_gate_status,
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
        "# `{}`\n\n- **Title:** {}\n- **Category:** {}\n- **Severity:** {}\n- **Confidence:** {}\n- **Lifecycle:** {}\n- **Signal source:** {}\n- **Semantic source:** {}\n- **Required scope:** {}\n- **Fixture coverage:** {} fixture(s), true-positive {}, false-positive {}\n- **False-positive risk:** {}\n- **Stability gate:** {}\n- **Docs:** {}\n- **Tags:** {}\n\n{}\n\n**Recommendation:** {}\n\n**False-positive notes:** {}\n",
        rule.rule_id,
        rule.title,
        rule.category,
        rule.severity,
        rule.confidence,
        rule.lifecycle.label(),
        rule.signal_source.label(),
        rule.semantic_source,
        rule.required_scope,
        rule.fixture_coverage.fixtures_total,
        rule.fixture_coverage.has_true_positive_fixture,
        rule.fixture_coverage.has_false_positive_fixture,
        rule.false_positive_risk,
        rule.stability_gate_status,
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

fn render_eval_report_console(report: &RuleEvaluationReport) -> String {
    format!(
        "RepoPilot Rule Evaluation\n\nRules evaluated: {}\nFixtures: {}\nExpected findings: {}\nActual findings: {}\nMissing findings: {}\nUnexpected findings: {}\nContract violations: {}\nStable ID failures: {}\n",
        report.rules_evaluated,
        report.fixtures_total,
        report.expected_findings,
        report.actual_findings,
        report.missing_findings,
        report.unexpected_findings,
        report.contract_violations,
        report.stable_id_failures,
    )
}

fn render_eval_report_markdown(report: &RuleEvaluationReport) -> String {
    let mut output = String::new();
    writeln!(output, "# RepoPilot Rule Evaluation\n").unwrap();
    writeln!(output, "- **Rules evaluated:** {}", report.rules_evaluated).unwrap();
    writeln!(output, "- **Fixtures:** {}", report.fixtures_total).unwrap();
    writeln!(
        output,
        "- **Expected findings:** {}",
        report.expected_findings
    )
    .unwrap();
    writeln!(output, "- **Actual findings:** {}", report.actual_findings).unwrap();
    writeln!(
        output,
        "- **Missing findings:** {}",
        report.missing_findings
    )
    .unwrap();
    writeln!(
        output,
        "- **Unexpected findings:** {}",
        report.unexpected_findings
    )
    .unwrap();
    writeln!(
        output,
        "- **Contract violations:** {}",
        report.contract_violations
    )
    .unwrap();
    writeln!(
        output,
        "- **Stable ID failures:** {}\n",
        report.stable_id_failures
    )
    .unwrap();

    if !report.rules.is_empty() {
        output.push_str("| Rule | Fixtures | Expected | Actual | Missing | Unexpected | Contract | Stable IDs |\n");
        output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
        for rule in &report.rules {
            writeln!(
                output,
                "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
                escape_table_cell(&rule.rule_id),
                rule.fixtures_total,
                rule.expected_findings,
                rule.actual_findings,
                rule.missing_findings,
                rule.unexpected_findings,
                rule.contract_violations,
                rule.stable_id_failures
            )
            .unwrap();
        }
    }

    output
}
