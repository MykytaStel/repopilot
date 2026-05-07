use crate::findings::types::{Finding, Severity};
use crate::scan::types::ScanSummary;
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const TOOL_NAME: &str = "RepoPilot";
const TOOL_INFORMATION_URI: &str = "https://github.com/MykytaStel/repopilot";

#[derive(Debug, Clone, Serialize)]
pub struct SarifLog {
    pub version: String,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifDriver {
    pub name: String,
    #[serde(rename = "informationUri")]
    pub information_uri: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifMessage,
    #[serde(rename = "fullDescription", skip_serializing_if = "Option::is_none")]
    pub full_description: Option<SarifMessage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<SarifLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<SarifRegion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SarifRegion {
    #[serde(rename = "startLine")]
    pub start_line: usize,
}

pub fn scan_summary_to_sarif(summary: &ScanSummary, root: &Path) -> SarifLog {
    findings_to_sarif(&summary.findings, root)
}

pub fn findings_to_sarif(findings: &[Finding], root: &Path) -> SarifLog {
    SarifLog {
        version: SARIF_VERSION.to_string(),
        schema: SARIF_SCHEMA.to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: TOOL_NAME.to_string(),
                    information_uri: TOOL_INFORMATION_URI.to_string(),
                    rules: sarif_rules(findings),
                },
            },
            results: findings
                .iter()
                .map(|finding| sarif_result(finding, root))
                .collect(),
        }],
    }
}

fn sarif_result(finding: &Finding, root: &Path) -> SarifResult {
    SarifResult {
        rule_id: finding.rule_id.clone(),
        level: sarif_level(finding.severity).to_string(),
        message: SarifMessage {
            text: finding_message(finding),
        },
        locations: finding
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
            .collect(),
    }
}

fn sarif_rules(findings: &[Finding]) -> Vec<SarifRule> {
    let rule_ids = findings
        .iter()
        .map(|finding| finding.rule_id.as_str())
        .collect::<BTreeSet<_>>();

    rule_ids
        .into_iter()
        .map(|rule_id| SarifRule {
            id: rule_id.to_string(),
            name: rule_id.to_string(),
            short_description: SarifMessage {
                text: format!("RepoPilot rule {rule_id}"),
            },
            full_description: None,
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
