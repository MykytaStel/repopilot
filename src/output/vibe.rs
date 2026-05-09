mod findings;
mod header;
mod hotfiles;
mod recommendations;

use crate::findings::types::Finding;
use crate::scan::types::ScanSummary;
use std::str::FromStr;

pub struct VibeOptions {
    pub focus: Option<VibeCategory>,
    /// Approximate token budget (1 token ≈ 4 chars).
    pub budget_tokens: usize,
    pub no_header: bool,
}

impl Default for VibeOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: 4096,
            no_header: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VibeCategory {
    Security,
    Architecture,
    Quality,
    Framework,
    All,
}

impl FromStr for VibeCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "security" => Ok(Self::Security),
            "arch" | "architecture" => Ok(Self::Architecture),
            "quality" => Ok(Self::Quality),
            "framework" => Ok(Self::Framework),
            "all" => Ok(Self::All),
            _ => Err(()),
        }
    }
}

impl VibeCategory {
    pub(self) fn matches(&self, category: &crate::findings::types::FindingCategory) -> bool {
        use crate::findings::types::FindingCategory;
        match self {
            VibeCategory::All => true,
            VibeCategory::Security => matches!(category, FindingCategory::Security),
            VibeCategory::Architecture => matches!(category, FindingCategory::Architecture),
            VibeCategory::Quality => matches!(
                category,
                FindingCategory::CodeQuality | FindingCategory::Testing
            ),
            VibeCategory::Framework => matches!(category, FindingCategory::Framework),
        }
    }
}

pub fn render(summary: &ScanSummary, opts: &VibeOptions) -> String {
    let budget_chars = opts.budget_tokens * 4;

    let findings: Vec<&Finding> = summary
        .findings
        .iter()
        .filter(|f| {
            opts.focus
                .as_ref()
                .is_none_or(|focus| focus.matches(&f.category))
        })
        .collect();

    let mut out = String::new();

    if !opts.no_header {
        header::render_header(&mut out, summary, &findings);
    }

    findings::render_findings_by_category(&mut out, &findings, budget_chars, opts.no_header);
    hotfiles::render_hot_files(&mut out, summary);
    recommendations::render_top_recommendations(&mut out, &findings);

    let content_len = out.len();
    recommendations::render_footer(
        &mut out,
        content_len,
        opts.budget_tokens,
        summary.scan_duration_us,
    );

    out
}

#[cfg(test)]
mod tests {
    use super::header::risk_level;
    use super::{VibeCategory, VibeOptions, render};
    use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
    use crate::scan::types::ScanSummary;
    use std::path::PathBuf;

    fn make_finding(
        rule_id: &str,
        title: &str,
        severity: Severity,
        category: FindingCategory,
        path: &str,
        line: usize,
    ) -> Finding {
        Finding {
            id: rule_id.to_string(),
            rule_id: rule_id.to_string(),
            title: title.to_string(),
            description: format!("Description for {title}"),
            category,
            severity,
            evidence: vec![Evidence {
                path: PathBuf::from(path),
                line_start: line,
                line_end: None,
                snippet: format!("// snippet for {title}"),
            }],
            workspace_package: None,
            docs_url: None,
        }
    }

    fn make_summary(findings: Vec<Finding>) -> ScanSummary {
        ScanSummary {
            root_path: PathBuf::from("/my-project"),
            files_count: 42,
            lines_of_code: 3000,
            directories_count: 10,
            languages: vec![],
            findings,
            ..Default::default()
        }
    }

    #[test]
    fn renders_header_with_risk_high() {
        let findings = vec![make_finding(
            "security.secret",
            "Hardcoded secret",
            Severity::Critical,
            FindingCategory::Security,
            "src/auth.rs",
            42,
        )];
        let summary = make_summary(findings);
        let output = render(&summary, &VibeOptions::default());
        assert!(output.contains("# RepoPilot Vibe Check — my-project"));
        assert!(output.contains("🔴 HIGH"));
        assert!(output.contains("1 critical"));
    }

    #[test]
    fn renders_no_header_when_flag_set() {
        let summary = make_summary(vec![]);
        let opts = VibeOptions {
            no_header: true,
            ..Default::default()
        };
        let output = render(&summary, &opts);
        assert!(!output.contains("# RepoPilot Vibe Check"));
    }

    #[test]
    fn focus_filters_to_security_only() {
        let findings = vec![
            make_finding(
                "security.secret",
                "Hardcoded secret",
                Severity::Critical,
                FindingCategory::Security,
                "src/auth.rs",
                1,
            ),
            make_finding(
                "architecture.large-file",
                "Large file",
                Severity::Medium,
                FindingCategory::Architecture,
                "src/big.rs",
                1,
            ),
        ];
        let summary = make_summary(findings);
        let opts = VibeOptions {
            focus: Some(VibeCategory::Security),
            ..Default::default()
        };
        let output = render(&summary, &opts);
        assert!(output.contains("Security"));
        assert!(!output.contains("Architecture"));
    }

    #[test]
    fn focus_quality_includes_testing_and_code_quality() {
        let findings = vec![
            make_finding(
                "code-quality.todo",
                "TODO marker",
                Severity::Low,
                FindingCategory::CodeQuality,
                "src/a.rs",
                1,
            ),
            make_finding(
                "testing.missing-tests",
                "Missing tests",
                Severity::Medium,
                FindingCategory::Testing,
                "src/b.rs",
                1,
            ),
            make_finding(
                "security.secret",
                "Secret",
                Severity::Critical,
                FindingCategory::Security,
                "src/c.rs",
                1,
            ),
        ];
        let summary = make_summary(findings);
        let opts = VibeOptions {
            focus: Some(VibeCategory::Quality),
            ..Default::default()
        };
        let output = render(&summary, &opts);
        assert!(output.contains("Code Quality") || output.contains("Testing"));
        assert!(!output.contains("Security"));
    }

    #[test]
    fn risk_level_moderate_for_one_high() {
        let findings = [make_finding(
            "arch.coupling",
            "High coupling",
            Severity::High,
            FindingCategory::Architecture,
            "src/a.rs",
            1,
        )];
        let refs: Vec<&Finding> = findings.iter().collect();
        assert_eq!(risk_level(&refs), "🟡 MODERATE");
    }

    #[test]
    fn risk_level_elevated_for_three_high() {
        let findings: Vec<Finding> = (0..3)
            .map(|i| {
                make_finding(
                    "arch.coupling",
                    "High coupling",
                    Severity::High,
                    FindingCategory::Architecture,
                    "src/a.rs",
                    i,
                )
            })
            .collect();
        let refs: Vec<&Finding> = findings.iter().collect();
        assert_eq!(risk_level(&refs), "🟠 ELEVATED");
    }

    #[test]
    fn risk_level_low_for_no_high() {
        let findings = [make_finding(
            "code.todo",
            "TODO",
            Severity::Low,
            FindingCategory::CodeQuality,
            "src/a.rs",
            1,
        )];
        let refs: Vec<&Finding> = findings.iter().collect();
        assert_eq!(risk_level(&refs), "🟢 LOW");
    }

    #[test]
    fn token_estimate_in_footer() {
        let summary = make_summary(vec![]);
        let output = render(&summary, &VibeOptions::default());
        assert!(output.contains("tokens"));
        assert!(output.contains("budget: 4096"));
    }

    #[test]
    fn top_recommendations_omitted_when_no_high_findings() {
        let findings = vec![make_finding(
            "code.todo",
            "TODO marker",
            Severity::Low,
            FindingCategory::CodeQuality,
            "src/a.rs",
            1,
        )];
        let summary = make_summary(findings);
        let output = render(&summary, &VibeOptions::default());
        assert!(!output.contains("## Top Recommendations"));
    }

    #[test]
    fn top_recommendations_shown_for_high_findings() {
        let findings = vec![make_finding(
            "security.secret",
            "Hardcoded secret",
            Severity::High,
            FindingCategory::Security,
            "src/a.rs",
            5,
        )];
        let summary = make_summary(findings);
        let output = render(&summary, &VibeOptions::default());
        assert!(output.contains("## Top Recommendations"));
    }

    #[test]
    fn empty_scan_renders_without_panic() {
        let summary = make_summary(vec![]);
        let output = render(&summary, &VibeOptions::default());
        assert!(output.contains("RepoPilot Vibe Check"));
        assert!(output.contains("0 findings"));
    }

    #[test]
    fn vibe_category_from_str() {
        assert_eq!("security".parse(), Ok(VibeCategory::Security));
        assert_eq!("arch".parse(), Ok(VibeCategory::Architecture));
        assert_eq!("quality".parse(), Ok(VibeCategory::Quality));
        assert_eq!("framework".parse(), Ok(VibeCategory::Framework));
        assert_eq!("all".parse(), Ok(VibeCategory::All));
        assert_eq!("unknown".parse::<VibeCategory>(), Err(()));
    }
}
