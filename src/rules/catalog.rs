use crate::rules::{
    RuleLifecycle, RuleMetadata, SignalSource, all_rule_metadata, lookup_rule_metadata,
};
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RuleCatalogFilter {
    pub lifecycle: Option<RuleLifecycle>,
    pub source: Option<SignalSource>,
}

#[derive(Debug, Serialize)]
pub struct RuleCatalogReport {
    pub rules: Vec<RuleCatalogItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleCatalogItem {
    pub rule_id: &'static str,
    pub title: &'static str,
    pub category: String,
    pub severity: String,
    pub confidence: String,
    pub lifecycle: RuleLifecycle,
    pub signal_source: SignalSource,
    pub docs_url: Option<&'static str>,
    pub tags: &'static [&'static str],
    pub description: &'static str,
    pub recommendation: Option<&'static str>,
    pub false_positive_notes: Option<&'static str>,
    pub semantic_source: &'static str,
    pub required_scope: &'static str,
    pub required_facts: Vec<&'static str>,
    pub cache_policy: &'static str,
    pub produces: Vec<&'static str>,
    pub fixture_coverage: RuleFixtureCoverage,
    pub false_positive_risk: &'static str,
    pub stability_gate_status: &'static str,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct RuleFixtureCoverage {
    pub fixtures_total: usize,
    pub has_true_positive_fixture: bool,
    pub has_false_positive_fixture: bool,
}

pub fn list_rule_catalog(filter: RuleCatalogFilter) -> RuleCatalogReport {
    let rules = all_rule_metadata()
        .filter(|rule| filter.lifecycle.is_none_or(|value| rule.lifecycle == value))
        .filter(|rule| {
            filter
                .source
                .is_none_or(|value| rule.signal_source == value)
        })
        .map(RuleCatalogItem::from)
        .collect::<Vec<_>>();

    RuleCatalogReport { rules }
}

pub fn inspect_rule(rule_id: &str) -> Option<RuleCatalogItem> {
    lookup_rule_metadata(rule_id).map(RuleCatalogItem::from)
}

impl From<&'static RuleMetadata> for RuleCatalogItem {
    fn from(rule: &'static RuleMetadata) -> Self {
        let fixture_coverage = fixture_coverage_for_rule(rule.rule_id);
        Self {
            rule_id: rule.rule_id,
            title: rule.title,
            category: rule.category.label().to_string(),
            severity: rule.default_severity.label().to_string(),
            confidence: rule.default_confidence.label().to_string(),
            lifecycle: rule.requirements.lifecycle,
            signal_source: rule.signal_source,
            docs_url: rule.docs_url,
            tags: rule.tags,
            description: rule.description,
            recommendation: rule.recommendation,
            false_positive_notes: rule.false_positive_notes,
            semantic_source: rule.signal_source.label(),
            required_scope: rule.requirements.scope.label(),
            required_facts: rule
                .requirements
                .fact_kinds
                .iter()
                .map(|fact| fact.label())
                .collect(),
            cache_policy: rule.requirements.cache_policy.label(),
            produces: rule
                .requirements
                .produces
                .iter()
                .map(|output| output.label())
                .collect(),
            false_positive_risk: false_positive_risk(rule),
            stability_gate_status: stability_gate_status(rule, &fixture_coverage),
            fixture_coverage,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FixtureExpectations {
    fixtures: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
struct FixtureCase {
    expected_rule_ids: Vec<String>,
}

fn fixture_coverage_for_rule(rule_id: &str) -> RuleFixtureCoverage {
    let expected_path = default_fixture_root().join(rule_id).join("expected.json");
    let Ok(content) = std::fs::read_to_string(expected_path) else {
        return RuleFixtureCoverage::default();
    };
    let Ok(expectations) = serde_json::from_str::<FixtureExpectations>(&content) else {
        return RuleFixtureCoverage::default();
    };

    let mut coverage = RuleFixtureCoverage {
        fixtures_total: expectations.fixtures.len(),
        ..RuleFixtureCoverage::default()
    };

    for fixture in expectations.fixtures {
        let expects_rule = fixture
            .expected_rule_ids
            .iter()
            .any(|expected_rule| expected_rule == rule_id);
        coverage.has_true_positive_fixture |= expects_rule;
        coverage.has_false_positive_fixture |= !expects_rule;
    }

    coverage
}

fn false_positive_risk(rule: &RuleMetadata) -> &'static str {
    if rule.default_confidence == crate::findings::types::Confidence::Low {
        return "high";
    }

    match (rule.lifecycle, rule.signal_source) {
        (RuleLifecycle::Stable, SignalSource::ImportGraph | SignalSource::DependencyManifest) => {
            "low"
        }
        (RuleLifecycle::Stable, _) => "medium",
        (RuleLifecycle::Preview, SignalSource::TextHeuristic | SignalSource::Mixed) => "medium",
        (RuleLifecycle::Experimental, _) => "high",
        _ => "medium",
    }
}

fn stability_gate_status(
    rule: &RuleMetadata,
    fixture_coverage: &RuleFixtureCoverage,
) -> &'static str {
    let requires_gate = rule.lifecycle == RuleLifecycle::Stable
        || rule.default_severity >= crate::findings::types::Severity::High;

    if !requires_gate {
        return "not-required";
    }

    if fixture_coverage.has_true_positive_fixture && fixture_coverage.has_false_positive_fixture {
        "fixture-covered"
    } else {
        "blocked-needs-true-and-false-positive-fixtures"
    }
}

fn default_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_rules_are_fixture_backed_and_metadata_complete() {
        for rule in list_rule_catalog(RuleCatalogFilter {
            lifecycle: Some(RuleLifecycle::Stable),
            source: None,
        })
        .rules
        {
            assert!(
                rule.fixture_coverage.has_true_positive_fixture,
                "stable rule {} needs a true-positive fixture",
                rule.rule_id
            );
            assert!(
                rule.fixture_coverage.has_false_positive_fixture,
                "stable rule {} needs a false-positive fixture",
                rule.rule_id
            );
            assert_eq!(
                rule.stability_gate_status, "fixture-covered",
                "stable rule {} must pass the fixture gate",
                rule.rule_id
            );
            assert!(
                rule.false_positive_notes
                    .is_some_and(|notes| !notes.trim().is_empty()),
                "stable rule {} needs false-positive notes",
                rule.rule_id
            );
            if matches!(rule.severity.as_str(), "HIGH" | "CRITICAL") {
                assert!(
                    rule.docs_url.is_some(),
                    "stable high/critical rule {} needs docs URL",
                    rule.rule_id
                );
            }
        }
    }
}
