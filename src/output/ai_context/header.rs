use crate::findings::types::{Finding, FindingCategory, Severity};
use crate::output::ai_context::project_name;
use crate::output::report_stats::risk_label_for_findings;
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub(super) fn render_header(out: &mut String, summary: &ScanSummary, findings: &[&Finding]) {
    let project_name = project_name(summary);

    let _ = writeln!(out, "# RepoPilot AI Context — {project_name}");
    out.push('\n');

    let risk = risk_level(findings);
    let _ = writeln!(out, "**Risk Level:** {risk}");

    let stack = build_tech_stack(summary);
    if !stack.is_empty() {
        let _ = writeln!(out, "**Tech Stack:** {}", stack.join(", "));
    }

    let token_est = summary.metrics.non_empty_lines * 5;
    let token_est_str = if token_est >= 1000 {
        format!("~{}k tokens", token_est / 1000)
    } else {
        format!("~{token_est} tokens")
    };
    let _ = writeln!(
        out,
        "**Size:** {} files · {} non-empty lines · {} directories · {token_est_str}",
        summary.metrics.files_analyzed,
        summary.metrics.non_empty_lines,
        summary.metrics.directories_count
    );

    if !summary.metrics.languages.is_empty() {
        let langs: Vec<String> = summary
            .metrics
            .languages
            .iter()
            .take(5)
            .map(|l| format!("{} ({})", l.name, l.files_analyzed))
            .collect();
        let _ = writeln!(out, "**Languages:** {}", langs.join(", "));
    }

    let critical = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high = findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();
    let medium = findings
        .iter()
        .filter(|f| f.severity == Severity::Medium)
        .count();
    let total = findings.len();
    let density = if summary.metrics.non_empty_lines > 0 {
        format!(
            " · {:.1}/kloc",
            total as f64 * 1000.0 / summary.metrics.non_empty_lines as f64
        )
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "**Health:** {total} findings{density} — {critical} critical, {high} high, {medium} medium"
    );
    if summary.metrics.large_files_skipped > 0 {
        let _ = writeln!(
            out,
            "⚠️ {} files skipped (too large to scan)",
            summary.metrics.large_files_skipped
        );
    }
    out.push('\n');
}

/// Prepend an AI task instruction block before the findings.
/// Only rendered when the user will paste the output into an AI assistant.
pub(super) fn render_task_instruction(
    out: &mut String,
    findings: &[&Finding],
    summary: &ScanSummary,
) {
    if findings.is_empty() {
        return;
    }

    let project = project_name(summary);

    let critical_sec = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical && f.category == FindingCategory::Security)
        .count();
    let high_sec = findings
        .iter()
        .filter(|f| f.severity == Severity::High && f.category == FindingCategory::Security)
        .count();
    let high_arch = findings
        .iter()
        .filter(|f| f.severity == Severity::High && f.category == FindingCategory::Architecture)
        .count();
    let critical_total = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    let high_total = findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();

    let _ = writeln!(
        out,
        "> **Instructions for AI assistant:** RepoPilot scan of `{project}`."
    );

    let mut step = 1usize;

    if critical_sec + high_sec > 0 {
        let n = critical_sec + high_sec;
        let sev = if critical_sec > 0 {
            "critical"
        } else {
            "high-severity"
        };
        let _ = writeln!(
            out,
            "> {step}. Fix the {n} {sev} security finding(s) with concrete code patches."
        );
        step += 1;
    } else if critical_total + high_total > 0 {
        let n = critical_total + high_total;
        let _ = writeln!(
            out,
            "> {step}. Fix the {n} critical/high finding(s) with concrete code patches."
        );
        step += 1;
    } else {
        let n = findings.len();
        let _ = writeln!(
            out,
            "> {step}. Review the {n} finding(s) and suggest targeted improvements."
        );
        step += 1;
    }

    if high_arch > 0 {
        let _ = writeln!(
            out,
            "> {step}. Address the {high_arch} high-severity architecture issue(s)."
        );
        step += 1;
    }

    let _ = writeln!(
        out,
        "> {step}. Keep changes minimal and focused — no rewrites unless necessary."
    );

    out.push('\n');
}

pub(super) fn risk_level(findings: &[&Finding]) -> &'static str {
    match risk_label_for_findings(findings) {
        "High" => "🔴 HIGH",
        "Elevated" => "🟠 ELEVATED",
        "Moderate" => "🟡 MODERATE",
        "Low" => "🟢 LOW",
        "Clean" => "🟢 CLEAN",
        _ => "🟢 CLEAN",
    }
}

fn build_tech_stack(summary: &ScanSummary) -> Vec<String> {
    let mut parts: Vec<String> = summary
        .artifacts
        .detected_frameworks
        .iter()
        .map(|f| f.label())
        .collect();

    if let Some(rn) = &summary.artifacts.react_native {
        let archs = [
            rn.android_new_arch_enabled,
            rn.ios_new_arch_enabled,
            rn.expo_new_arch_enabled,
        ];
        let new_count = archs.iter().filter(|v| **v == Some(true)).count();
        let old_count = archs.iter().filter(|v| **v == Some(false)).count();
        let new_arch = match (new_count > 0, old_count > 0) {
            (true, true) => " (Mixed Arch)",
            (true, false) => " (New Arch)",
            (false, true) => " (Old Arch)",
            _ => "",
        };

        if !new_arch.is_empty() {
            for part in &mut parts {
                if part.starts_with("React Native") {
                    part.push_str(new_arch);
                    break;
                }
            }
        }
    }

    parts
}
