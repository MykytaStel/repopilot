//! Deterministic, evidence-backed verification plans for high-confidence
//! findings — no LLM, no rule-authoring burden. Each plan anchors to the
//! finding's own [`Evidence`], adds one category-level "what to check" step,
//! and closes with a fixed statement of what static analysis cannot verify.

use crate::findings::decision::VerificationPlan;
use crate::findings::types::{Confidence, Finding, FindingCategory};

/// Builds a verification plan for `finding`, or `None` when its confidence
/// isn't [`Confidence::High`] — a low/medium-confidence finding is already
/// flagged as uncertain, so a step-by-step confirmation plan would overstate
/// how actionable it is.
pub fn build_verification_plan(finding: &Finding) -> Option<VerificationPlan> {
    if finding.confidence != Confidence::High {
        return None;
    }

    let mut steps: Vec<String> = finding
        .evidence
        .iter()
        .map(|evidence| {
            let end = evidence
                .line_end
                .map(|end| format!("-{end}"))
                .unwrap_or_default();
            format!(
                "Open {}:{}{end} and confirm the flagged code shown is still present: `{}`.",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim(),
            )
        })
        .collect();

    steps.push(category_check(&finding.category).to_string());
    steps.push(
        "This plan is generated from static evidence only \u{2014} it does not execute code, \
         run tests, or observe runtime behavior. Treat it as a starting point for manual or \
         test-based confirmation, not proof the finding is a true or false positive."
            .to_string(),
    );

    Some(VerificationPlan { steps })
}

/// One "what to check, how to confirm or dismiss" step per finding category.
fn category_check(category: &FindingCategory) -> &'static str {
    match category {
        FindingCategory::Security => {
            "Confirm the flagged input reaches this code without validation or sanitization, \
             and check for an existing test covering this path."
        }
        FindingCategory::Architecture => {
            "Confirm the reported structural metric (size, fan-out, coupling, depth) against \
             the thresholds configured in `repopilot.toml`."
        }
        FindingCategory::CodeQuality => {
            "Run or add a test for the affected function to confirm behavior is unchanged \
             before and after any fix."
        }
        FindingCategory::Testing => {
            "Check whether an equivalent test already exists elsewhere in the suite before \
             adding a new one."
        }
        FindingCategory::Framework => {
            "Confirm the flagged framework usage matches the version and configuration \
             actually pinned in this repository's manifests."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::provenance::FindingProvenance;
    use crate::findings::severity::Severity;
    use crate::findings::types::Evidence;
    use std::path::PathBuf;

    fn finding_with(
        category: FindingCategory,
        confidence: Confidence,
        evidence: Vec<Evidence>,
    ) -> Finding {
        Finding {
            id: "rule.example:src/lib.rs:deadbeef".to_string(),
            rule_id: "rule.example".to_string(),
            title: "Example".to_string(),
            description: "desc".to_string(),
            recommendation: "fix it".to_string(),
            category,
            severity: Severity::High,
            confidence,
            evidence,
            workspace_package: None,
            docs_url: None,
            provenance: FindingProvenance::default(),
            risk: Default::default(),
        }
    }

    fn evidence(path: &str, line_start: usize, line_end: Option<usize>, snippet: &str) -> Evidence {
        Evidence {
            path: PathBuf::from(path),
            line_start,
            line_end,
            snippet: snippet.to_string(),
        }
    }

    #[test]
    fn high_confidence_single_evidence_produces_three_ordered_steps() {
        let finding = finding_with(
            FindingCategory::Security,
            Confidence::High,
            vec![evidence("src/lib.rs", 5, None, "let x = 1;")],
        );

        let plan = build_verification_plan(&finding).expect("high confidence should get a plan");

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(
            plan.steps[0],
            "Open src/lib.rs:5 and confirm the flagged code shown is still present: `let x = 1;`."
        );
        assert!(plan.steps[1].contains("validation or sanitization"));
        assert!(plan.steps[2].contains("does not execute code"));
    }

    #[test]
    fn line_end_is_rendered_as_a_range() {
        let finding = finding_with(
            FindingCategory::Security,
            Confidence::High,
            vec![evidence("src/lib.rs", 5, Some(8), "fn f() {}")],
        );

        let plan = build_verification_plan(&finding).unwrap();
        assert!(plan.steps[0].starts_with("Open src/lib.rs:5-8"));
    }

    #[test]
    fn multiple_evidence_entries_each_get_their_own_step() {
        let finding = finding_with(
            FindingCategory::Security,
            Confidence::High,
            vec![
                evidence("src/a.rs", 1, None, "a"),
                evidence("src/b.rs", 2, None, "b"),
            ],
        );

        let plan = build_verification_plan(&finding).unwrap();
        // 2 evidence steps + category step + closing step.
        assert_eq!(plan.steps.len(), 4);
        assert!(plan.steps[0].contains("src/a.rs:1"));
        assert!(plan.steps[1].contains("src/b.rs:2"));
    }

    #[test]
    fn medium_and_low_confidence_yield_no_plan() {
        let medium = finding_with(FindingCategory::Security, Confidence::Medium, vec![]);
        let low = finding_with(FindingCategory::Security, Confidence::Low, vec![]);

        assert!(build_verification_plan(&medium).is_none());
        assert!(build_verification_plan(&low).is_none());
    }

    #[test]
    fn every_category_has_its_own_check_text() {
        let categories = [
            FindingCategory::Security,
            FindingCategory::Architecture,
            FindingCategory::CodeQuality,
            FindingCategory::Testing,
            FindingCategory::Framework,
        ];

        let checks: Vec<&str> = categories.iter().map(category_check).collect();
        let unique: std::collections::BTreeSet<&str> = checks.iter().copied().collect();
        assert_eq!(unique.len(), categories.len(), "checks must all differ");
    }

    #[test]
    fn same_finding_produces_identical_plan_across_calls() {
        let finding = finding_with(
            FindingCategory::CodeQuality,
            Confidence::High,
            vec![evidence("src/lib.rs", 5, None, "let x = 1;")],
        );

        let first = build_verification_plan(&finding);
        let second = build_verification_plan(&finding);
        assert_eq!(first, second);
    }
}
