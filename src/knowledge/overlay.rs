use crate::findings::severity::Severity;
use crate::scan::types::ScanDiagnostic;
use serde::Deserialize;
use std::path::PathBuf;

pub const OVERLAY_PATH: &str = ".repopilot/overlay.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayTarget {
    Rule(String),
    Kind(String),
}

#[derive(Debug, Clone)]
pub struct OverlayEntry {
    pub index: usize,
    pub target: OverlayTarget,
    pub path_text: Option<String>,
    pub path_glob: Option<globset::GlobMatcher>,
    pub severity: Option<Severity>,
    pub reason: Option<String>,
    pub expires: Option<chrono::NaiveDate>,
}

#[derive(Debug, Default)]
pub struct OverlayValidation {
    pub overlay_path: PathBuf,
    pub exists: bool,
    pub entries: Vec<OverlayEntry>,
    pub invalid_entries_count: usize,
    pub parse_error: Option<String>,
    pub diagnostics: Vec<ScanDiagnostic>,
}

#[derive(Debug, Default, Deserialize)]
struct RawOverlayFile {
    #[serde(default, rename = "overlay")]
    entries: Vec<RawOverlayEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct RawOverlayEntry {
    rule: Option<String>,
    kind: Option<String>,
    path: Option<String>,
    severity: Option<String>,
    reason: Option<String>,
    expires: Option<String>,
}

pub fn parse_overlay_content(content: &str, overlay_path: PathBuf) -> OverlayValidation {
    let parsed = match toml::from_str::<RawOverlayFile>(content) {
        Ok(parsed) => parsed,
        Err(error) => {
            let message = error.to_string();
            return OverlayValidation {
                overlay_path: overlay_path.clone(),
                exists: true,
                parse_error: Some(message.clone()),
                diagnostics: vec![
                    ScanDiagnostic::warning(
                        "overlay.parse-failed",
                        format!("Could not parse local overlay TOML: {message}"),
                    )
                    .with_path(overlay_path),
                ],
                ..OverlayValidation::default()
            };
        }
    };

    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut invalid_entries_count = 0;

    for (offset, raw) in parsed.entries.into_iter().enumerate() {
        let index = offset + 1;
        match build_entry(index, raw, &overlay_path) {
            Ok(entry) => entries.push(entry),
            Err(diagnostic) => {
                invalid_entries_count += 1;
                diagnostics.push(diagnostic);
            }
        }
    }

    OverlayValidation {
        overlay_path,
        exists: true,
        entries,
        invalid_entries_count,
        parse_error: None,
        diagnostics,
    }
}

fn build_entry(
    index: usize,
    raw: RawOverlayEntry,
    overlay_path: &std::path::Path,
) -> Result<OverlayEntry, ScanDiagnostic> {
    let target = match (clean(raw.rule), clean(raw.kind)) {
        (Some(rule), None) => OverlayTarget::Rule(rule),
        (None, Some(kind)) => OverlayTarget::Kind(kind),
        (None, None) => {
            return Err(ScanDiagnostic::warning(
                "overlay.invalid-entry",
                format!("Overlay entry #{index} is missing exactly one of `rule` or `kind`."),
            )
            .with_path(overlay_path.to_path_buf()));
        }
        (Some(_), Some(_)) => {
            return Err(ScanDiagnostic::warning(
                "overlay.invalid-entry",
                format!("Overlay entry #{index} has both `rule` and `kind`; exactly one is allowed."),
            )
            .with_path(overlay_path.to_path_buf()));
        }
    };

    if let OverlayTarget::Rule(rule_id) = &target
        && crate::rules::lookup_rule_metadata(rule_id).is_none()
    {
        return Err(ScanDiagnostic::warning(
            "overlay.unknown-rule",
            format!("Overlay entry #{index} references unknown rule id `{rule_id}`."),
        )
        .with_path(overlay_path.to_path_buf()));
    }

    if matches!(target, OverlayTarget::Kind(_)) && raw.severity.is_some() {
        return Err(ScanDiagnostic::warning(
            "overlay.invalid-entry",
            format!("Overlay entry #{index} sets `severity` on a `kind` entry; review signals have no severity."),
        )
        .with_path(overlay_path.to_path_buf()));
    }

    let severity = match clean(raw.severity) {
        None => None,
        Some(label) => match Severity::from_lowercase_label(&label) {
            Some(severity) => Some(severity),
            None => {
                return Err(ScanDiagnostic::warning(
                    "overlay.invalid-severity",
                    format!(
                        "Overlay entry #{index} has invalid severity `{label}` (expected info/low/medium/high/critical)."
                    ),
                )
                .with_path(overlay_path.to_path_buf()));
            }
        },
    };

    let path_text = clean(raw.path);
    let path_glob = match &path_text {
        None => None,
        Some(pattern) => match globset::Glob::new(pattern) {
            Ok(glob) => Some(glob.compile_matcher()),
            Err(error) => {
                return Err(ScanDiagnostic::warning(
                    "overlay.invalid-path-glob",
                    format!("Overlay entry #{index} has an invalid path glob `{pattern}`: {error}"),
                )
                .with_path(overlay_path.to_path_buf()));
            }
        },
    };

    let expires = match clean(raw.expires) {
        None => None,
        Some(value) => match chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
            Ok(date) => Some(date),
            Err(_) => {
                return Err(ScanDiagnostic::warning(
                    "overlay.invalid-expiry",
                    format!("Overlay entry #{index} has invalid expires date `{value}`."),
                )
                .with_path(overlay_path.to_path_buf()));
            }
        },
    };

    Ok(OverlayEntry {
        index,
        target,
        path_text,
        path_glob,
        severity,
        reason: clean(raw.reason),
        expires,
    })
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_rule_entry_with_path_and_severity() {
        let content = r#"
            [[overlay]]
            rule = "architecture.large-file"
            path = "legacy/**"
            severity = "low"
            reason = "Legacy freeze until Q3 migration"
        "#;
        let validation = parse_overlay_content(content, PathBuf::from(".repopilot/overlay.toml"));

        assert!(validation.parse_error.is_none());
        assert_eq!(validation.entries.len(), 1);
        let entry = &validation.entries[0];
        assert_eq!(entry.target, OverlayTarget::Rule("architecture.large-file".to_string()));
        assert_eq!(entry.severity, Some(Severity::Low));
        assert_eq!(entry.reason.as_deref(), Some("Legacy freeze until Q3 migration"));
        assert!(entry.path_glob.is_some());
    }
}
