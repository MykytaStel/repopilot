use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::ScanFacts;

pub struct RnDepHealthAudit;

impl ProjectAudit for RnDepHealthAudit {
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        let deps = match read_all_deps(&facts.root_path) {
            Some(d) => d,
            None => return vec![],
        };

        let rn_ver = deps
            .get("react-native")
            .and_then(|v| v.as_str())
            .and_then(parse_version);

        let mut findings = Vec::new();

        // 1. @react-native-community/async-storage (legacy, unmaintained)
        if deps.contains_key("@react-native-community/async-storage") {
            findings.push(Finding {
                id: String::new(),
                rule_id: "framework.rn-async-storage-legacy".to_string(),
                title: "Deprecated async-storage package in use".to_string(),
                description: concat!(
                    "`@react-native-community/async-storage` is unmaintained. ",
                    "Migrate to `@react-native-async-storage/async-storage` which is the actively maintained fork. ",
                    "Run: `npm remove @react-native-community/async-storage && npm install @react-native-async-storage/async-storage`"
                ).to_string(),
                category: FindingCategory::Framework,
                severity: Severity::Medium,
                evidence: vec![Evidence {
                    path: facts.root_path.join("package.json"),
                    line_start: 1,
                    line_end: None,
                    snippet: "@react-native-community/async-storage is deprecated".to_string(),
                }],
                workspace_package: None,
            });
        }

        // 2. @react-navigation/native v4/v5 with RN >= 0.73 (needs v6+)
        let nav_ver = deps
            .get("@react-navigation/native")
            .and_then(|v| v.as_str())
            .and_then(parse_version);
        if let Some(nav_ver) = nav_ver {
            if nav_ver.0 < 6 && rn_ver.map(|rn| rn >= (0, 73, 0)).unwrap_or(false) {
                findings.push(Finding {
                    id: String::new(),
                    rule_id: "framework.rn-navigation-compat".to_string(),
                    title: "React Navigation version is incompatible with this React Native version"
                        .to_string(),
                    description: format!(
                        "`@react-navigation/native` v{}.x is not compatible with `react-native` ≥0.73. \
                         Upgrade to v6 or later: `npm install @react-navigation/native@latest`",
                        nav_ver.0
                    ),
                    category: FindingCategory::Framework,
                    severity: Severity::High,
                    evidence: vec![Evidence {
                        path: facts.root_path.join("package.json"),
                        line_start: 1,
                        line_end: None,
                        snippet: format!(
                            "@react-navigation/native: v{}.{}.{} — upgrade to v6+",
                            nav_ver.0, nav_ver.1, nav_ver.2
                        ),
                    }],
                    workspace_package: None,
                });
            }
        }

        // 3. react-native-reanimated <3 with RN >= 0.73
        let rea_ver = deps
            .get("react-native-reanimated")
            .and_then(|v| v.as_str())
            .and_then(parse_version);
        if let Some(rea_ver) = rea_ver {
            if rea_ver.0 < 3 && rn_ver.map(|rn| rn >= (0, 73, 0)).unwrap_or(false) {
                findings.push(Finding {
                    id: String::new(),
                    rule_id: "framework.rn-reanimated-compat".to_string(),
                    title:
                        "react-native-reanimated version is incompatible with this React Native version"
                            .to_string(),
                    description: format!(
                        "`react-native-reanimated` v{}.x requires React Native <0.73. \
                         Upgrade to v3+: `npm install react-native-reanimated@latest`",
                        rea_ver.0
                    ),
                    category: FindingCategory::Framework,
                    severity: Severity::High,
                    evidence: vec![Evidence {
                        path: facts.root_path.join("package.json"),
                        line_start: 1,
                        line_end: None,
                        snippet: format!(
                            "react-native-reanimated: v{}.{}.{} — upgrade to v3+",
                            rea_ver.0, rea_ver.1, rea_ver.2
                        ),
                    }],
                    workspace_package: None,
                });
            }
        }

        // 4. react-native-gesture-handler <2 with RN >= 0.72
        let gh_ver = deps
            .get("react-native-gesture-handler")
            .and_then(|v| v.as_str())
            .and_then(parse_version);
        if let Some(gh_ver) = gh_ver {
            if gh_ver.0 < 2 && rn_ver.map(|rn| rn >= (0, 72, 0)).unwrap_or(false) {
                findings.push(Finding {
                    id: String::new(),
                    rule_id: "framework.rn-gesture-handler-old".to_string(),
                    title: "react-native-gesture-handler v1 is incompatible with this React Native version"
                        .to_string(),
                    description: format!(
                        "`react-native-gesture-handler` v{}.x does not support React Native ≥0.72. \
                         Upgrade to v2+: `npm install react-native-gesture-handler@latest`",
                        gh_ver.0
                    ),
                    category: FindingCategory::Framework,
                    severity: Severity::High,
                    evidence: vec![Evidence {
                        path: facts.root_path.join("package.json"),
                        line_start: 1,
                        line_end: None,
                        snippet: format!(
                            "react-native-gesture-handler: v{}.{}.{} — upgrade to v2+",
                            gh_ver.0, gh_ver.1, gh_ver.2
                        ),
                    }],
                    workspace_package: None,
                });
            }
        }

        // 5. Known New Architecture incompatible packages (when RN >= 0.71)
        if rn_ver.map(|rn| rn >= (0, 71, 0)).unwrap_or(false) {
            for pkg in NEW_ARCH_INCOMPATIBLE {
                if deps.contains_key(*pkg) {
                    findings.push(Finding {
                        id: String::new(),
                        rule_id: "framework.rn-new-arch-incompatible-dep".to_string(),
                        title: format!("`{pkg}` does not support React Native New Architecture"),
                        description: format!(
                            "`{pkg}` has no New Architecture support. \
                             Check the library's GitHub issues for a migration path or switch to an alternative."
                        ),
                        category: FindingCategory::Framework,
                        severity: Severity::Medium,
                        evidence: vec![Evidence {
                            path: facts.root_path.join("package.json"),
                            line_start: 1,
                            line_end: None,
                            snippet: format!("{pkg} lacks New Architecture support"),
                        }],
                        workspace_package: None,
                    });
                }
            }
        }

        findings
    }
}

/// Packages known to lack React Native New Architecture (Fabric/TurboModules) support.
const NEW_ARCH_INCOMPATIBLE: &[&str] = &[
    "react-native-camera",
    "react-native-firebase",
    "react-native-linear-gradient",
    "react-native-maps",
    "react-native-svg",
    "react-native-video",
    "rn-fetch-blob",
    "@react-native-community/cameraroll",
    "@react-native-community/netinfo",
    "react-native-share",
    "react-native-splash-screen",
];

fn read_all_deps(root: &std::path::Path) -> Option<serde_json::Map<String, serde_json::Value>> {
    let content = std::fs::read_to_string(root.join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let mut merged = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = value.get(key).and_then(|v| v.as_object()) {
            for (k, v) in obj {
                merged.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }
    if merged.is_empty() { None } else { Some(merged) }
}

/// Parse a semver-like version string into (major, minor, patch).
/// Strips leading `^`, `~`, `>=`, `>`, `=`.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s
        .trim()
        .trim_start_matches(['^', '~', '=', '>', '<'])
        .trim();
    let parts: Vec<&str> = s.splitn(4, '.').collect();
    let major = parts.first()?.parse::<u64>().ok()?;
    let minor = parts.get(1).unwrap_or(&"0").parse::<u64>().ok().unwrap_or(0);
    let patch = parts
        .get(2)
        .map(|p| p.split('-').next().unwrap_or("0"))
        .unwrap_or("0")
        .parse::<u64>()
        .ok()
        .unwrap_or(0);
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_semver_variations() {
        assert_eq!(parse_version("^0.74.0"), Some((0, 74, 0)));
        assert_eq!(parse_version("~3.0.0"), Some((3, 0, 0)));
        assert_eq!(parse_version("2.0.0-rc.1"), Some((2, 0, 0)));
        assert_eq!(parse_version("1"), Some((1, 0, 0)));
        assert_eq!(parse_version(">=0.73.0"), Some((0, 73, 0)));
        assert_eq!(parse_version("not-a-version"), None);
    }
}
