use crate::audits::framework::js_common::is_comment_line;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

// ── Class components ──────────────────────────────────────────────────────────

pub struct ReactClassComponentAudit;

impl ProjectAudit for ReactClassComponentAudit {
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
                if trimmed.contains("extends")
                    && (trimmed.contains("React.Component")
                        || trimmed.contains("React.PureComponent"))
                {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.react.class-component".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react.class-component"),
                        title: "Class component found".to_string(),
                        description: concat!(
                            "This file uses a class-based React component (`extends React.Component` or `React.PureComponent`). ",
                            "Class components are a legacy pattern — the React team recommends migrating to function components with hooks. ",
                            "Hooks provide the same lifecycle and state capabilities with less boilerplate, ",
                            "better tree-shaking, and simpler testing."
                        ).to_string(),
                        category: FindingCategory::Framework,
                        severity: Severity::Low,
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
                    break; // one finding per file
                }
            }
        }

        findings
    }
}

// ── PropTypes ─────────────────────────────────────────────────────────────────

pub struct ReactPropTypesAudit;

impl ProjectAudit for ReactPropTypesAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let has_typescript = facts
            .languages
            .iter()
            .any(|l| l.name == "TypeScript" || l.name == "TypeScript React");
        if !has_typescript {
            return vec![];
        }

        let mut findings = Vec::new();

        for file in &facts.files {
            let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "tsx" && ext != "jsx" && ext != "ts" && ext != "js" {
                continue;
            }

            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.contains("from 'prop-types'") || trimmed.contains("from \"prop-types\"")
                {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.react.prop-types".to_string(),
        recommendation: Finding::recommendation_for_rule_id("framework.react.prop-types"),
                        title: "PropTypes used in TypeScript project".to_string(),
                        description: concat!(
                            "This project already uses TypeScript, which provides static type checking at compile time. ",
                            "`prop-types` adds redundant runtime overhead — it validates types in production builds and duplicates ",
                            "what TypeScript already enforces. Remove the `prop-types` package and replace PropTypes with TypeScript interface or type definitions."
                        ).to_string(),
                        category: FindingCategory::Framework,
                        severity: Severity::Low,
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
                    break; // one finding per file
                }
            }
        }

        findings
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::config::ScanConfig;
    use crate::scan::facts::{FileFacts, ScanFacts};
    use crate::scan::types::LanguageSummary;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn class_component_detected() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("App.tsx");
        let mut f = std::fs::File::create(&file_path).unwrap();
        write!(
            f,
            "class MyComp extends React.Component {{\n  render() {{ return null; }}\n}}\n"
        )
        .unwrap();

        let mut facts = ScanFacts {
            root_path: dir.path().to_path_buf(),
            ..ScanFacts::default()
        };
        facts.files.push(FileFacts {
            path: file_path,
            language: Some("TypeScript React".to_string()),
            lines_of_code: 3,
            branch_count: 0,
            imports: vec![],
            content: None,
            has_inline_tests: false,
        });

        let findings = ReactClassComponentAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "framework.react.class-component");
    }

    #[test]
    fn prop_types_skipped_without_typescript() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Comp.jsx");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "import PropTypes from 'prop-types';").unwrap();

        let mut facts = ScanFacts {
            root_path: dir.path().to_path_buf(),
            ..ScanFacts::default()
        };
        facts.files.push(FileFacts {
            path: file_path,
            language: Some("JavaScript React".to_string()),
            lines_of_code: 1,
            branch_count: 0,
            imports: vec![],
            content: None,
            has_inline_tests: false,
        });
        // no TypeScript in languages list

        let findings = ReactPropTypesAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn prop_types_flagged_in_typescript_project() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Comp.tsx");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "import PropTypes from 'prop-types';").unwrap();

        let mut facts = ScanFacts {
            root_path: dir.path().to_path_buf(),
            ..ScanFacts::default()
        };
        facts.languages.push(LanguageSummary {
            name: "TypeScript React".to_string(),
            files_count: 5,
        });
        facts.files.push(FileFacts {
            path: file_path,
            language: Some("TypeScript React".to_string()),
            lines_of_code: 1,
            branch_count: 0,
            imports: vec![],
            content: None,
            has_inline_tests: false,
        });

        let findings = ReactPropTypesAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "framework.react.prop-types");
    }
}
