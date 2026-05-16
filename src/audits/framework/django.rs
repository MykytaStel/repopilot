use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

// ── django.debug-true ─────────────────────────────────────────────────────────

pub struct DjangoDebugTrueAudit;

impl ProjectAudit for DjangoDebugTrueAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            if !is_django_settings_file(&file.path) {
                continue;
            }

            let Ok(content) = std::fs::read_to_string(&file.path) else {
                continue;
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if is_comment(trimmed) {
                    continue;
                }
                if trimmed == "DEBUG = True" {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.django.debug-true".to_string(),
                        recommendation: Finding::recommendation_for_rule_id(
                            "framework.django.debug-true",
                        ),
                        title: "DEBUG = True in Django settings".to_string(),
                        description: concat!(
                            "Django's `DEBUG = True` mode exposes detailed error pages with stack traces, ",
                            "local variables, and settings values to anyone who triggers an exception. ",
                            "This setting must be `False` in any environment reachable from the internet."
                        )
                        .to_string(),
                        category: FindingCategory::Security,
                        severity: Severity::High,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                        workspace_package: None,
                        docs_url: None,
            risk: Default::default(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ── django.missing-allowed-hosts ──────────────────────────────────────────────

pub struct DjangoEmptyAllowedHostsAudit;

impl ProjectAudit for DjangoEmptyAllowedHostsAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            if !is_django_settings_file(&file.path) {
                continue;
            }

            let Ok(content) = std::fs::read_to_string(&file.path) else {
                continue;
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if is_comment(trimmed) {
                    continue;
                }
                if trimmed == "ALLOWED_HOSTS = []" {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.django.missing-allowed-hosts".to_string(),
                        recommendation: Finding::recommendation_for_rule_id(
                            "framework.django.missing-allowed-hosts",
                        ),
                        title: "ALLOWED_HOSTS is empty in Django settings".to_string(),
                        description: concat!(
                            "`ALLOWED_HOSTS = []` permits any host header when `DEBUG = False`, ",
                            "making the application vulnerable to HTTP Host header attacks. ",
                            "Set `ALLOWED_HOSTS` to the explicit domain names and IPs this service accepts."
                        )
                        .to_string(),
                        category: FindingCategory::Security,
                        severity: Severity::High,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                        workspace_package: None,
                        docs_url: None,
            risk: Default::default(),
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ── django.raw-sql-query ──────────────────────────────────────────────────────

pub struct DjangoRawSqlAudit;

impl ProjectAudit for DjangoRawSqlAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            if file
                .path
                .extension()
                .and_then(|e| e.to_str())
                .is_none_or(|ext| ext != "py")
            {
                continue;
            }

            let Ok(content) = std::fs::read_to_string(&file.path) else {
                continue;
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if is_comment(trimmed) {
                    continue;
                }
                if trimmed.contains("cursor.execute(") && contains_string_formatting(trimmed) {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.django.raw-sql-query".to_string(),
                        recommendation: Finding::recommendation_for_rule_id(
                            "framework.django.raw-sql-query",
                        ),
                        title: "Raw SQL with string formatting detected".to_string(),
                        description: concat!(
                            "Using Python string formatting (`%`, `.format()`, or f-strings) inside a `cursor.execute()` call ",
                            "can introduce SQL injection vulnerabilities. ",
                            "Pass query parameters as a separate argument tuple: `cursor.execute(sql, [param])` ",
                            "to let the database driver safely escape values."
                        )
                        .to_string(),
                        category: FindingCategory::Security,
                        severity: Severity::Medium,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                        workspace_package: None,
                        docs_url: None,
            risk: Default::default(),
                    });
                }
            }
        }

        findings
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn is_django_settings_file(path: &std::path::Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    path.extension().and_then(|e| e.to_str()) == Some("py")
        && (stem == "settings" || stem.starts_with("settings_") || stem.ends_with("_settings"))
}

fn is_comment(line: &str) -> bool {
    line.starts_with('#')
}

fn contains_string_formatting(line: &str) -> bool {
    line.contains("% ") || line.contains(".format(") || line.contains("f\"") || line.contains("f'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_settings_file_names() {
        assert!(is_django_settings_file(std::path::Path::new(
            "myapp/settings.py"
        )));
        assert!(is_django_settings_file(std::path::Path::new(
            "config/settings_production.py"
        )));
        assert!(is_django_settings_file(std::path::Path::new(
            "settings/base_settings.py"
        )));
        assert!(!is_django_settings_file(std::path::Path::new("views.py")));
    }

    #[test]
    fn detects_string_formatting_in_execute() {
        assert!(contains_string_formatting(
            r#"cursor.execute(f"SELECT * FROM users WHERE id={user_id}")"#
        ));
        assert!(contains_string_formatting(
            "cursor.execute(\"SELECT %s\" % val)"
        ));
        assert!(!contains_string_formatting(
            "cursor.execute(sql, [user_id])"
        ));
    }
}
