use crate::audits::framework::js_common::is_js_file;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;
use std::path::PathBuf;

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

        let gradle = facts.root_path.join("android/app/build.gradle");
        if let Ok(content) = std::fs::read_to_string(&gradle) {
            if content.contains("enableHermes: false") || content.contains("enableHermes : false") {
                findings.push(hermes_finding(gradle, "enableHermes: false"));
            }
        }

        let gradle_props = facts.root_path.join("android/gradle.properties");
        if let Ok(content) = std::fs::read_to_string(&gradle_props) {
            if gradle_properties_hermes_disabled(&content) {
                findings.push(hermes_finding(gradle_props, "hermesEnabled=false"));
            }
        }

        findings
    }
}

pub struct ReactNativeCodegenMissingAudit;

impl ProjectAudit for ReactNativeCodegenMissingAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let Some(profile) = &facts.react_native else {
            return vec![];
        };
        if profile.has_codegen_config || !has_codegen_usage_signal(facts) {
            return vec![];
        }

        vec![Finding {
            id: String::new(),
            rule_id: "framework.react-native.codegen-missing".to_string(),
            recommendation: Finding::recommendation_for_rule_id(
                "framework.react-native.codegen-missing",
            ),
            title: "React Native Codegen config is missing".to_string(),
            description: concat!(
                "This project appears to use Turbo Native Modules or Fabric components, ",
                "but package.json does not define codegenConfig. ",
                "Add codegenConfig so React Native can generate native interfaces consistently."
            )
            .to_string(),
            category: FindingCategory::Framework,
            severity: Severity::Medium,
            confidence: Default::default(),
            evidence: vec![Evidence {
                path: facts.root_path.join("package.json"),
                line_start: 1,
                line_end: None,
                snippet: "codegenConfig missing while Codegen usage was detected".to_string(),
            }],
            workspace_package: None,
            docs_url: Some("https://reactnative.dev/docs/the-new-architecture/codegen".to_string()),
            risk: Default::default(),
        }]
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
        recommendation: Finding::recommendation_for_rule_id("framework.react-native.hermes-disabled"),
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
        confidence: Default::default(),
        evidence: vec![Evidence {
            path,
            line_start: 1,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: Some("https://reactnative.dev/docs/hermes".to_string()),
            risk: Default::default(),
    }
}

fn has_codegen_usage_signal(facts: &ScanFacts) -> bool {
    facts
        .files
        .iter()
        .filter(|file| is_js_file(&file.path))
        .any(|file| {
            std::fs::read_to_string(&file.path)
                .map(|content| {
                    content.contains("TurboModuleRegistry")
                        || content.contains("codegenNativeComponent")
                        || (content.contains("TurboModule") && content.contains("react-native"))
                })
                .unwrap_or(false)
        })
}
