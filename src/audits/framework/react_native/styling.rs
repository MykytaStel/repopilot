use crate::audits::framework::js_common::{is_comment_line, is_js_file};
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub struct RnInlineStyleAudit;

impl ProjectAudit for RnInlineStyleAudit {
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

            if !content.contains("react-native") {
                continue;
            }

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if is_comment_line(trimmed) {
                    continue;
                }
                if trimmed.contains("style={{") {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.react-native.inline-style".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.inline-style"),
                        title: "Inline style object in JSX".to_string(),
                        description: concat!(
                            "Inline style objects (`style={{ ... }}`) create a new object on every render, ",
                            "which defeats memoization in `React.memo` and `PureComponent` children. ",
                            "Extract styles into a `StyleSheet.create` call outside the component."
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
                        docs_url: Some("https://reactnative.dev/docs/stylesheet".to_string()),
                    });
                    break;
                }
            }
        }

        findings
    }
}

const DEPRECATED_RN_APIS: &[(&str, &str)] = &[
    ("ViewPagerAndroid", "react-native-pager-view"),
    ("ToolbarAndroid", "@react-native-community/toolbar-android"),
    (
        "DatePickerAndroid",
        "@react-native-community/datetimepicker",
    ),
    (
        "TimePickerAndroid",
        "@react-native-community/datetimepicker",
    ),
    ("MaskedViewIOS", "@react-native-masked-view/masked-view"),
    (
        "ProgressBarAndroid",
        "@react-native-community/progress-bar-android",
    ),
    ("ProgressViewIOS", "@react-native-community/progress-view"),
    (
        "SegmentedControlIOS",
        "@react-native-community/segmented-control",
    ),
    ("CheckBox", "@react-native-community/checkbox"),
];

pub struct RnDeprecatedApiAudit;

impl ProjectAudit for RnDeprecatedApiAudit {
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

            if !content.contains("react-native") {
                continue;
            }

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if is_comment_line(trimmed) {
                    continue;
                }
                for (api, replacement) in DEPRECATED_RN_APIS {
                    if trimmed.contains(api) {
                        findings.push(Finding {
                            id: String::new(),
                            rule_id: "framework.react-native.deprecated-api".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.deprecated-api"),
                            title: format!("Deprecated React Native API: {api}"),
                            description: format!(
                                "`{api}` was removed from React Native core. \
                                Replace it with `{replacement}` from the React Native community packages."
                            ),
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
                            docs_url: Some(
                                "https://reactnative.dev/docs/out-of-tree-platforms".to_string(),
                            ),
                        });
                        break;
                    }
                }
            }
        }

        findings
    }
}

pub struct RnFlatListMissingKeyAudit;

impl ProjectAudit for RnFlatListMissingKeyAudit {
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

            if !content.contains("react-native") {
                continue;
            }

            let lines: Vec<&str> = content.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed.contains("<FlatList") && !is_comment_line(trimmed) {
                    let window_end = (i + 20).min(lines.len());
                    let window: String = lines[i..window_end].join("\n");

                    if !window.contains("keyExtractor") {
                        findings.push(Finding {
                            id: String::new(),
                            rule_id: "framework.react-native.flatlist-missing-key".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.flatlist-missing-key"),
                            title: "FlatList is missing keyExtractor".to_string(),
                            description: concat!(
                                "A `FlatList` without `keyExtractor` falls back to array index keys, ",
                                "which breaks list reconciliation when items are reordered or removed. ",
                                "Add `keyExtractor={(item) => item.id.toString()}` (or equivalent unique key)."
                            )
                            .to_string(),
                            category: FindingCategory::Framework,
                            severity: Severity::Low,
                            confidence: Default::default(),
                            evidence: vec![Evidence {
                                path: file.path.clone(),
                                line_start: i + 1,
                                line_end: None,
                                snippet: trimmed.to_string(),
                            }],
                            workspace_package: None,
                            docs_url: Some(
                                "https://reactnative.dev/docs/flatlist#keyextractor".to_string(),
                            ),
                        });
                    }
                    i = window_end;
                    continue;
                }
                i += 1;
            }
        }

        findings
    }
}
