use crate::audits::framework::js_common::is_js_file;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub struct AsyncStorageFromCoreAudit;

impl ProjectAudit for AsyncStorageFromCoreAudit {
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

            if let Some(line_start) = find_async_storage_from_core(&content) {
                findings.push(Finding {
                    id: String::new(),
                    rule_id: "framework.react-native.async-storage-from-core".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.async-storage-from-core"),
                    title: "AsyncStorage imported from 'react-native' core".to_string(),
                    description: concat!(
                        "`AsyncStorage` was removed from `react-native` core in v0.60 and will throw ",
                        "a runtime error on modern React Native versions. ",
                        "Replace with `import AsyncStorage from '@react-native-async-storage/async-storage'` ",
                        "and add `@react-native-async-storage/async-storage` to your dependencies."
                    )
                    .to_string(),
                    category: FindingCategory::Framework,
                    severity: Severity::High,
                    confidence: Default::default(),
                    evidence: vec![Evidence {
                        path: file.path.clone(),
                        line_start,
                        line_end: None,
                        snippet: content
                            .lines()
                            .nth(line_start - 1)
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                    }],
                    workspace_package: None,
                    docs_url: Some(
                        "https://react-native-async-storage.github.io/async-storage/docs/install"
                            .to_string(),
                    ),
                    risk: Default::default(),
                });
            }
        }

        findings
    }
}

/// Returns the 1-based line number of the import start if AsyncStorage is imported from
/// react-native. Handles both single-line and multi-line imports.
fn find_async_storage_from_core(content: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if trimmed.starts_with("import")
            && is_from_rn_core(trimmed)
            && trimmed.contains("AsyncStorage")
        {
            return Some(i + 1);
        }

        if trimmed.starts_with("import") && trimmed.contains('{') && !trimmed.contains('}') {
            let start_line = i;
            let mut block = trimmed.to_string();
            let mut j = i + 1;

            while j < lines.len() && j - i <= 20 {
                let next = lines[j].trim();
                block.push(' ');
                block.push_str(next);

                if is_from_rn_core(next) {
                    if block.contains("AsyncStorage") {
                        return Some(start_line + 1);
                    }
                    break;
                }
                if next.starts_with('}') && !is_from_rn_core(next) {
                    break;
                }
                j += 1;
            }
            i = j + 1;
            continue;
        }

        i += 1;
    }

    None
}

fn is_from_rn_core(line: &str) -> bool {
    line.contains("from 'react-native'")
        || line.contains("from \"react-native\"")
        || line.contains("from 'react-native';")
        || line.contains("from \"react-native\";")
}
