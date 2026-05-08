use crate::audits::framework::js_common::{is_comment_line, is_js_file};
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::PathBuf;

// ── Old Architecture ──────────────────────────────────────────────────────────

pub struct ReactNativeOldArchAudit;

impl ProjectAudit for ReactNativeOldArchAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        if check_new_arch_enabled(&facts.root_path) {
            return vec![];
        }

        let (evidence_path, snippet) = config_file_evidence(&facts.root_path);

        vec![Finding {
            id: String::new(),
            rule_id: "framework.react-native.old-architecture".to_string(),
            title: "React Native New Architecture is not enabled".to_string(),
            description: concat!(
                "This project does not have `newArchEnabled: true` in its app.json / app.config.js / react-native.config.js. ",
                "The New Architecture (Fabric renderer + TurboModules) eliminates the asynchronous JS bridge, ",
                "delivers faster UI updates, and is required by an increasing number of third-party libraries. ",
                "Enable it by setting `\"newArchEnabled\": true` in your app.json `expo` block or in react-native.config.js."
            ).to_string(),
            category: FindingCategory::Framework,
            severity: Severity::Medium,
            evidence: vec![Evidence {
                path: evidence_path,
                line_start: 1,
                line_end: None,
                snippet,
            }],
        }]
    }
}

fn check_new_arch_enabled(root: &std::path::Path) -> bool {
    // Check both Expo (app.json/app.config.*) and bare RN (react-native.config.js)
    for name in [
        "app.json",
        "app.config.js",
        "app.config.ts",
        "react-native.config.js",
    ] {
        if let Ok(content) = std::fs::read_to_string(root.join(name)) {
            if content.contains("\"newArchEnabled\": true")
                || content.contains("\"newArchEnabled\":true")
                || content.contains("newArchEnabled: true")
            {
                return true;
            }
        }
    }
    false
}

fn config_file_evidence(root: &std::path::Path) -> (PathBuf, String) {
    for name in [
        "app.json",
        "app.config.js",
        "app.config.ts",
        "react-native.config.js",
    ] {
        let path = root.join(name);
        if path.exists() {
            return (path, format!("newArchEnabled not set to true in {name}"));
        }
    }
    (
        root.to_path_buf(),
        "No app.json / app.config.js / react-native.config.js found; newArchEnabled not configured"
            .to_string(),
    )
}

// ── AsyncStorage from core ────────────────────────────────────────────────────

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
                });
            }
        }

        findings
    }
}

/// Returns the 1-based line number of the import start if AsyncStorage is imported from react-native.
/// Handles both single-line and multi-line imports:
///   import { View, AsyncStorage } from 'react-native';
///   import {
///     AsyncStorage,
///   } from 'react-native';
fn find_async_storage_from_core(content: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Single-line: import { ..., AsyncStorage, ... } from 'react-native'
        if trimmed.starts_with("import")
            && is_from_rn_core(trimmed)
            && trimmed.contains("AsyncStorage")
        {
            return Some(i + 1);
        }

        // Multi-line: import { starts here, accumulate until the `from 'react-native'` line
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
                // Closing brace without react-native → not our import
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

// ── Hermes disabled ───────────────────────────────────────────────────────────

pub struct HermesDisabledAudit;

impl ProjectAudit for HermesDisabledAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let mut findings = Vec::new();

        let podfile = facts.root_path.join("ios/Podfile");
        if let Ok(content) = std::fs::read_to_string(&podfile) {
            if content.contains("hermes_enabled: false")
                || content.contains("hermes_enabled => false")
            {
                findings.push(hermes_finding(podfile, "hermes_enabled: false"));
            }
        }

        // Legacy location (< RN 0.70): android/app/build.gradle
        let gradle = facts.root_path.join("android/app/build.gradle");
        if let Ok(content) = std::fs::read_to_string(&gradle) {
            if content.contains("enableHermes: false") || content.contains("enableHermes : false") {
                findings.push(hermes_finding(gradle, "enableHermes: false"));
            }
        }

        // Modern location (>= RN 0.70): android/gradle.properties
        let gradle_props = facts.root_path.join("android/gradle.properties");
        if let Ok(content) = std::fs::read_to_string(&gradle_props) {
            if gradle_properties_hermes_disabled(&content) {
                findings.push(hermes_finding(gradle_props, "hermesEnabled=false"));
            }
        }

        findings
    }
}

/// Returns true when gradle.properties explicitly disables Hermes.
/// Tolerates spaces around `=` and ignores inline comments.
fn gradle_properties_hermes_disabled(content: &str) -> bool {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key != "hermesEnabled" && key != "enableHermes" {
            continue;
        }
        let value = rest.split_once('#').map(|(v, _)| v).unwrap_or(rest).trim();
        if value == "false" {
            return true;
        }
    }
    false
}

fn hermes_finding(path: PathBuf, snippet: &str) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "framework.react-native.hermes-disabled".to_string(),
        title: "Hermes JavaScript engine is disabled".to_string(),
        description: concat!(
            "Hermes is explicitly disabled in this project. ",
            "Hermes compiles JavaScript to bytecode at build time, reducing startup time by 2–3× ",
            "and lowering memory usage. It has been the recommended and default JS engine since React Native 0.70. ",
            "Remove the `hermes_enabled: false` / `enableHermes: false` flag to enable it."
        )
        .to_string(),
        category: FindingCategory::Framework,
        severity: Severity::Low,
        evidence: vec![Evidence {
            path,
            line_start: 1,
            line_end: None,
            snippet: snippet.to_string(),
        }],
    }
}

// ── Old React Navigation v4 ───────────────────────────────────────────────────

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
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
                    });
                    break;
                }
            }
        }

        findings
    }
}

// ── Direct state mutation in class components ─────────────────────────────────

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
                        evidence: vec![Evidence {
                            path: file.path.clone(),
                            line_start: idx + 1,
                            line_end: None,
                            snippet: trimmed.to_string(),
                        }],
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
    // skip the property name (alphanumeric + underscore)
    let rest = after_prefix.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
    // skip optional whitespace
    let rest = rest.trim_start();
    // must be `=` but NOT `==` or `===`
    rest.starts_with('=') && !rest.starts_with("==")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::config::ScanConfig;
    use crate::scan::facts::{FileFacts, ScanFacts};
    use std::io::Write;
    use tempfile::tempdir;

    fn facts_for(root: &std::path::Path) -> ScanFacts {
        ScanFacts {
            root_path: root.to_path_buf(),
            ..ScanFacts::default()
        }
    }

    fn jsx_file(dir: &tempfile::TempDir, name: &str, content: &str) -> FileFacts {
        let path = dir.path().join(name);
        write!(std::fs::File::create(&path).unwrap(), "{content}").unwrap();
        FileFacts {
            path,
            language: Some("TypeScript React".to_string()),
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: vec![],
            content: String::new(),
        }
    }

    // ── Old architecture ──────────────────────────────────────────────────────

    #[test]
    fn old_arch_flagged_when_no_app_json() {
        let dir = tempdir().unwrap();
        let findings =
            ReactNativeOldArchAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.old-architecture"
        );
    }

    #[test]
    fn old_arch_not_flagged_when_enabled() {
        let dir = tempdir().unwrap();
        write!(
            std::fs::File::create(dir.path().join("app.json")).unwrap(),
            r#"{{"expo": {{"newArchEnabled": true}}}}"#
        )
        .unwrap();
        let findings =
            ReactNativeOldArchAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn old_arch_not_flagged_when_enabled_in_rn_config() {
        let dir = tempdir().unwrap();
        writeln!(
            std::fs::File::create(dir.path().join("react-native.config.js")).unwrap(),
            "module.exports = {{ newArchEnabled: true }};"
        )
        .unwrap();
        let findings =
            ReactNativeOldArchAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── AsyncStorage ──────────────────────────────────────────────────────────

    #[test]
    fn async_storage_single_line_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Screen.tsx",
            "import { View, AsyncStorage, Text } from 'react-native';\n",
        ));
        let findings = AsyncStorageFromCoreAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.async-storage-from-core"
        );
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn async_storage_multi_line_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Screen.tsx",
            "import {\n  View,\n  AsyncStorage,\n  Text,\n} from 'react-native';\n",
        ));
        let findings = AsyncStorageFromCoreAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1, "multi-line import must be detected");
        assert_eq!(findings[0].evidence[0].line_start, 1);
    }

    #[test]
    fn async_storage_from_own_package_not_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Screen.tsx",
            "import AsyncStorage from '@react-native-async-storage/async-storage';\n",
        ));
        let findings = AsyncStorageFromCoreAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── React Navigation v4 ───────────────────────────────────────────────────

    #[test]
    fn old_react_navigation_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Navigator.tsx",
            "import { createStackNavigator } from 'react-navigation';\n",
        ));
        let findings = ReactNavigationV4Audit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.old-react-navigation"
        );
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn modern_react_navigation_not_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Navigator.tsx",
            "import { NavigationContainer } from '@react-navigation/native';\n",
        ));
        let findings = ReactNavigationV4Audit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── Direct state mutation ─────────────────────────────────────────────────

    #[test]
    fn direct_state_mutation_detected() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Comp.tsx",
            "class MyComp extends React.Component {\n  handleClick() {\n    this.state.count = 5;\n  }\n}\n",
        ));
        let findings = DirectStateMutationAudit.audit(&facts, &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.direct-state-mutation"
        );
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn state_equality_check_not_flagged() {
        let dir = tempdir().unwrap();
        let mut facts = facts_for(dir.path());
        facts.files.push(jsx_file(
            &dir,
            "Comp.tsx",
            "if (this.state.count === 5) { doSomething(); }\n",
        ));
        let findings = DirectStateMutationAudit.audit(&facts, &ScanConfig::default());
        assert!(findings.is_empty());
    }

    // ── Hermes disabled via gradle.properties ─────────────────────────────────

    #[test]
    fn hermes_disabled_in_gradle_properties_is_flagged() {
        let dir = tempdir().unwrap();
        let android = dir.path().join("android");
        std::fs::create_dir(&android).unwrap();
        write!(
            std::fs::File::create(android.join("gradle.properties")).unwrap(),
            "hermesEnabled=false\nnewArchEnabled=true\n"
        )
        .unwrap();

        let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].rule_id,
            "framework.react-native.hermes-disabled"
        );
    }

    #[test]
    fn hermes_enabled_in_gradle_properties_is_not_flagged() {
        let dir = tempdir().unwrap();
        let android = dir.path().join("android");
        std::fs::create_dir(&android).unwrap();
        writeln!(
            std::fs::File::create(android.join("gradle.properties")).unwrap(),
            "hermesEnabled=true"
        )
        .unwrap();

        let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert!(findings.is_empty());
    }

    #[test]
    fn hermes_disabled_gradle_properties_with_inline_comment_is_flagged() {
        let dir = tempdir().unwrap();
        let android = dir.path().join("android");
        std::fs::create_dir(&android).unwrap();
        writeln!(
            std::fs::File::create(android.join("gradle.properties")).unwrap(),
            "hermesEnabled=false   # JSC is faster for our use case"
        )
        .unwrap();

        let findings = HermesDisabledAudit.audit(&facts_for(dir.path()), &ScanConfig::default());
        assert_eq!(findings.len(), 1);
    }
}
