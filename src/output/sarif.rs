use crate::baseline::diff::BaselineScanReport;
use crate::findings::types::{Finding, Severity};
use crate::scan::types::ScanSummary;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

mod model;
pub use model::*;

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const TOOL_NAME: &str = "RepoPilot";
const TOOL_INFORMATION_URI: &str = "https://github.com/MykytaStel/repopilot";

pub fn scan_summary_to_sarif(summary: &ScanSummary, root: &Path) -> SarifLog {
    findings_to_sarif(&summary.findings, root)
}

pub fn render(summary: &ScanSummary) -> Result<String, serde_json::Error> {
    let sarif = scan_summary_to_sarif(summary, &summary.root_path);
    serde_json::to_string_pretty(&sarif)
}

pub fn render_with_baseline(report: &BaselineScanReport) -> Result<String, serde_json::Error> {
    let sarif = findings_to_sarif_with_baseline(report);
    serde_json::to_string_pretty(&sarif)
}

pub fn findings_to_sarif(findings: &[Finding], root: &Path) -> SarifLog {
    let properties = findings
        .iter()
        .map(|f| SarifResultProperties {
            baseline_status: None,
            baseline_key: None,
            workspace_package: f.workspace_package.clone(),
            category: f.category.label().to_string(),
        })
        .collect();
    findings_to_sarif_with_properties(findings, root, properties)
}

fn findings_to_sarif_with_baseline(report: &BaselineScanReport) -> SarifLog {
    let properties = report
        .summary
        .findings
        .iter()
        .enumerate()
        .map(|(index, finding)| SarifResultProperties {
            baseline_status: Some(report.finding_status(index).lowercase_label().to_string()),
            baseline_key: Some(
                report
                    .findings
                    .get(index)
                    .map(|f| f.key.clone())
                    .unwrap_or_default(),
            ),
            workspace_package: finding.workspace_package.clone(),
            category: finding.category.label().to_string(),
        })
        .collect();

    findings_to_sarif_with_properties(
        &report.summary.findings,
        &report.summary.root_path,
        properties,
    )
}

fn findings_to_sarif_with_properties(
    findings: &[Finding],
    root: &Path,
    properties: Vec<SarifResultProperties>,
) -> SarifLog {
    SarifLog {
        version: SARIF_VERSION.to_string(),
        schema: SARIF_SCHEMA.to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: TOOL_NAME.to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    information_uri: TOOL_INFORMATION_URI.to_string(),
                    rules: sarif_rules(findings),
                },
            },
            results: findings
                .iter()
                .enumerate()
                .map(|(index, finding)| sarif_result(finding, root, properties.get(index).cloned()))
                .collect(),
        }],
    }
}

fn sarif_result(
    finding: &Finding,
    root: &Path,
    properties: Option<SarifResultProperties>,
) -> SarifResult {
    let locations: Vec<SarifLocation> = finding
        .evidence
        .iter()
        .filter_map(|evidence| {
            let uri = sarif_uri(&evidence.path, root)?;
            Some(SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation { uri },
                    region: (evidence.line_start > 0).then_some(SarifRegion {
                        start_line: evidence.line_start,
                    }),
                },
            })
        })
        .collect();

    let partial_fingerprints = if let Some(first) = finding.evidence.first() {
        let key = format!(
            "{}:{}:{}",
            finding.rule_id,
            first.path.display(),
            first.line_start
        );
        BTreeMap::from([("primaryLocationLineHash/v1".to_string(), key)])
    } else {
        BTreeMap::new()
    };

    SarifResult {
        rule_id: finding.rule_id.clone(),
        level: sarif_level(finding.severity).to_string(),
        message: SarifMessage {
            text: finding_message(finding),
        },
        locations,
        partial_fingerprints,
        properties,
    }
}

fn sarif_rules(findings: &[Finding]) -> Vec<SarifRule> {
    let mut rule_map: BTreeMap<&str, &Finding> = BTreeMap::new();
    for finding in findings {
        rule_map.entry(finding.rule_id.as_str()).or_insert(finding);
    }

    rule_map
        .into_iter()
        .map(|(rule_id, finding)| {
            let meta = crate::rules::lookup_rule_metadata(rule_id);
            let help_uri = meta
                .and_then(|m| m.docs_url)
                .map(str::to_owned)
                .or_else(|| finding.docs_url.clone());
            let help = meta.and_then(|m| m.recommendation).map(|rec| SarifMessage {
                text: rec.to_string(),
            });
            SarifRule {
                id: rule_id.to_string(),
                name: rule_id.to_string(),
                short_description: SarifMessage {
                    text: finding.description.clone(),
                },
                full_description: None,
                help_uri,
                help,
            }
        })
        .collect()
}

fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical | Severity::High => "error",
        Severity::Medium => "warning",
        Severity::Low | Severity::Info => "note",
    }
}

fn finding_message(finding: &Finding) -> String {
    if finding.title.is_empty() {
        finding.description.clone()
    } else {
        finding.title.clone()
    }
}

fn sarif_uri(path: &Path, root: &Path) -> Option<String> {
    if path.as_os_str().is_empty() {
        return None;
    }

    let relative = root_relative_path(path, root);
    Some(path_to_forward_slash_uri(&relative))
}

fn root_relative_path(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn path_to_forward_slash_uri(path: &Path) -> String {
    path.components()
        .map(component_to_string)
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn component_to_string(component: Component<'_>) -> String {
    component.as_os_str().to_string_lossy().replace('\\', "/")
}
