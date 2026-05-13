use crate::audits::framework::js_common::{is_comment_line, is_js_file};
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub struct ReactNavigationV4Audit;

impl ProjectAudit for ReactNavigationV4Audit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            if !is_js_file(&file.path) {
                continue;
            }

            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                if is_comment_line(trimmed) {
                    continue;
                }

                if trimmed.starts_with("import")
                    && !trimmed.contains("@react-navigation")
                    && (trimmed.contains("from 'react-navigation'")
                        || trimmed.contains("from \"react-navigation\""))
                {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.react-native.old-react-navigation".to_string(),
                        title: "React Navigation v4 (unscoped package) detected".to_string(),
                        description: concat!(
                            "`react-navigation` (v4) is no longer maintained and incompatible with React Native 0.70+. ",
                            "The modern API is `@react-navigation/native` (v5/v6), which uses hooks, ",
                            "has full TypeScript support, and is actively maintained. ",
                            "Migrate: replace `from 'react-navigation'` imports with `from '@react-navigation/native'` ",
                            "and update navigator definitions. Migration guide: https://reactnavigation.org/docs/upgrading-from-4.x"
                        )
                        .to_string(),
                        category: FindingCategory::Framework,
                        severity: Severity::Medium,
                        confidence: Default::default(),
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                        workspace_package: None,
                        docs_url: Some(
                            "https://reactnavigation.org/docs/getting-started".to_string(),
                        ),
                    });
                    break;
                }
            }
        }

        findings
    }
}

pub struct DirectStateMutationAudit;

impl ProjectAudit for DirectStateMutationAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        for file in &facts.files {
            let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "tsx" && ext != "jsx" {
                continue;
            }

            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                if is_comment_line(trimmed) {
                    continue;
                }

                if is_direct_state_mutation(trimmed) {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.react-native.direct-state-mutation".to_string(),
                        title: "Direct mutation of this.state detected".to_string(),
                        description: concat!(
                            "Directly assigning to `this.state` bypasses React's change detection — ",
                            "the component will not re-render and the UI will fall out of sync with the actual state. ",
                            "Always use `this.setState({ key: value })` to update state in class components. ",
                            "In function components, use the setter returned by `useState`."
                        )
                        .to_string(),
                        category: FindingCategory::Framework,
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
                    });
                    break;
                }
            }
        }

        findings
    }
}

/// Returns true if the line looks like `this.state.someKey = value` (not `==` or `===`).
fn is_direct_state_mutation(trimmed: &str) -> bool {
    let Some(pos) = trimmed.find("this.state.") else {
        return false;
    };
    let after_prefix = &trimmed[pos + "this.state.".len()..];
    let rest = after_prefix.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
    let rest = rest.trim_start();
    rest.starts_with('=') && !rest.starts_with("==")
}
