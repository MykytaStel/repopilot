//! Golden snapshot of the AI context brief.
//!
//! `repopilot ai context` and the `repopilot_context` MCP tool both wrap
//! `output::ai_context::render`. This test scans one committed sample project,
//! applies the default visibility profile (matching the product scan), and
//! snapshots the rendered Markdown for the default and `--focus security`
//! variants — so a change to the agent-facing brief is a deliberate, reviewed
//! diff rather than a silent drift.
//!
//! Determinism: the scan wall-clock (`scanned in Nms`) is zeroed, the volatile
//! context-graph cache status line is normalized (write/hit/miss varies with
//! cache state across runs), and the crate version is templated. Evidence paths
//! are repo-relative because the scan root is the relative fixture path and
//! `cargo test` runs from the crate root. Bless with `REPOPILOT_UPDATE_GOLDEN=1`.

use repopilot::findings::visibility::{FindingVisibilityProfile, apply_visibility_profile};
use repopilot::output::ai_context::{
    AiContextRenderOptions, AiFocusCategory, DEFAULT_TOKEN_BUDGET, render,
};
use repopilot::scan::config::ScanConfig;
use repopilot::scan::scanner::scan_path_with_config;
use repopilot::scan::types::ScanSummary;
use std::fs;
use std::path::{Path, PathBuf};

const FIXTURE: &str = "tests/fixtures/projects/ai-context-sample";

fn scanned_summary() -> ScanSummary {
    let mut summary = scan_path_with_config(Path::new(FIXTURE), &ScanConfig::default())
        .expect("scan ai-context-sample fixture");
    assert!(
        summary
            .artifacts
            .findings
            .iter()
            .any(|finding| finding.rule_id == "framework.django.debug-true"),
        "fixture scan produced no Django findings — is `cargo test` running from the crate root?"
    );
    // Mirror the product scan's default surface so the golden matches `ai context`.
    apply_visibility_profile(&mut summary, FindingVisibilityProfile::Default);
    // The footer prints the scan wall-clock; pin it for a stable snapshot.
    summary.metadata.scan_duration_us = 0;
    summary
}

fn render_variant(focus: Option<AiFocusCategory>) -> String {
    let options = AiContextRenderOptions {
        focus,
        budget_tokens: DEFAULT_TOKEN_BUDGET,
        no_header: false,
        no_task: false,
    };
    render(&scanned_summary(), &options)
}

#[test]
fn ai_context_default_brief_stays_stable() {
    assert_golden("ai-context-default.md", render_variant(None));
}

#[test]
fn ai_context_security_focus_brief_stays_stable() {
    assert_golden(
        "ai-context-focus-security.md",
        render_variant(Some(AiFocusCategory::Security)),
    );
}

fn assert_golden(name: &str, actual: String) {
    let actual = normalize(actual);
    let path = golden_path(name);

    if std::env::var_os("REPOPILOT_UPDATE_GOLDEN").is_some() {
        fs::write(&path, &actual).expect("failed to update golden fixture");
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("missing golden {name}: {err}; bless with REPOPILOT_UPDATE_GOLDEN=1")
    });
    assert_eq!(actual, expected, "AI context golden changed: {name}");
}

/// Pin the volatile bits: the crate version, the context-graph cache status
/// (write/hit/miss varies with cache state between runs), and the footer's
/// approximate token count (which shifts with the cache-status string length).
fn normalize(output: String) -> String {
    let mut out = String::new();
    for line in output.replace("\r\n", "\n").lines() {
        let line = if line.trim_start().starts_with("- Cache:") {
            "- Cache: {{CACHE}}".to_string()
        } else if line.contains("tokens (budget:") {
            pin_token_count(line)
        } else {
            line.replace(env!("CARGO_PKG_VERSION"), "{{VERSION}}")
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Footer: `*~1054 tokens (budget: 4096) · scanned in 0ms — …*`. Replace the
/// approximate count between `~` and ` tokens` with a placeholder.
fn pin_token_count(line: &str) -> String {
    match (line.find('~'), line.find(" tokens")) {
        (Some(start), Some(end)) if start < end => {
            format!("{}~{{{{TOKENS}}}}{}", &line[..start], &line[end..])
        }
        _ => line.to_string(),
    }
}

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden")
        .join(name)
}
