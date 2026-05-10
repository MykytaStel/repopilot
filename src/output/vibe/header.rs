use crate::findings::types::{Finding, Severity};
use crate::output::report_stats::risk_label_for_findings;
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub(super) fn render_header(out: &mut String, summary: &ScanSummary, findings: &[&Finding]) {
    let project_name = summary
        .root_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    let _ = writeln!(out, "# RepoPilot Vibe Check — {project_name}");
    out.push('\n');

    let risk = risk_level(findings);
    let _ = writeln!(out, "**Risk Level:** {risk}");

    let stack = build_tech_stack(summary);
    if !stack.is_empty() {
        let _ = writeln!(out, "**Tech Stack:** {}", stack.join(", "));
    }

    let token_est = summary.lines_of_code * 5;
    let token_est_str = if token_est >= 1000 {
        format!("~{}k tokens", token_est / 1000)
    } else {
        format!("~{token_est} tokens")
    };
    let _ = writeln!(
        out,
        "**Size:** {} files · {} LOC · {} directories · {token_est_str}",
        summary.files_count, summary.lines_of_code, summary.directories_count
    );

    if !summary.languages.is_empty() {
        let langs: Vec<String> = summary
            .languages
            .iter()
            .take(5)
            .map(|l| format!("{} ({})", l.name, l.files_count))
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
    let density = if summary.lines_of_code > 0 {
        format!(
            " · {:.1}/kloc",
            total as f64 * 1000.0 / summary.lines_of_code as f64
        )
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "**Health:** {total} findings{density} — {critical} critical, {high} high, {medium} medium"
    );
    if summary.skipped_files_count > 0 {
        let _ = writeln!(
            out,
            "⚠️ {} files skipped (too large to scan)",
            summary.skipped_files_count
        );
    }
    out.push('\n');
}

pub(super) fn risk_level(findings: &[&Finding]) -> &'static str {
    match risk_label_for_findings(findings) {
        "High" => "🔴 HIGH",
        "Elevated" => "🟠 ELEVATED",
        "Moderate" => "🟡 MODERATE",
        "Low" => "🟢 LOW",
        _ => "🟢 CLEAN",
    }
}

fn build_tech_stack(summary: &ScanSummary) -> Vec<String> {
    let mut parts: Vec<String> = summary
        .detected_frameworks
        .iter()
        .map(|f| f.label())
        .collect();

    if let Some(rn) = &summary.react_native {
        let new_arch = match (
            rn.android_new_arch_enabled,
            rn.ios_new_arch_enabled,
            rn.expo_new_arch_enabled,
        ) {
            (Some(true), _, _) | (_, Some(true), _) | (_, _, Some(true)) => " (New Arch)",
            (Some(false), _, _) | (_, Some(false), _) => " (Old Arch)",
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
