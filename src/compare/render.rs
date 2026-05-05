use super::diff::CompareSummary;

pub fn render_json(summary: &CompareSummary) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(summary)
}

pub fn render_console(summary: &CompareSummary) -> String {
    let mut out = String::new();

    out.push_str("RepoPilot Compare Report\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');

    let files_delta = summary.after_files as i64 - summary.before_files as i64;
    let loc_delta = summary.after_loc as i64 - summary.before_loc as i64;

    out.push_str(&format!(
        "Files:       {} → {}  ({:+})\n",
        summary.before_files, summary.after_files, files_delta
    ));
    out.push_str(&format!(
        "LOC:         {} → {}  ({:+})\n",
        summary.before_loc, summary.after_loc, loc_delta
    ));
    out.push('\n');
    out.push_str(&format!(
        "New findings:       {:>4}\n",
        summary.new_findings.len()
    ));
    out.push_str(&format!(
        "Resolved:           {:>4}\n",
        summary.resolved_findings.len()
    ));
    out.push_str(&format!(
        "Severity increased: {:>4}\n",
        summary.severity_increased.len()
    ));

    if !summary.new_findings.is_empty() {
        out.push_str("\n── New findings ──────────────────────────\n");
        for f in &summary.new_findings {
            out.push_str(&format!(
                "  [{}] {} — {}\n",
                f.severity_label(),
                f.rule_id,
                f.title
            ));
        }
    }

    if !summary.resolved_findings.is_empty() {
        out.push_str("\n── Resolved findings ─────────────────────\n");
        for f in &summary.resolved_findings {
            out.push_str(&format!(
                "  [{}] {} — {}\n",
                f.severity_label(),
                f.rule_id,
                f.title
            ));
        }
    }

    if !summary.severity_increased.is_empty() {
        out.push_str("\n── Severity increased ────────────────────\n");
        for (f, old_sev) in &summary.severity_increased {
            out.push_str(&format!(
                "  {} → [{}] {}\n",
                old_sev.label(),
                f.severity_label(),
                f.rule_id
            ));
        }
    }

    out
}

pub fn render_markdown(summary: &CompareSummary) -> String {
    let mut out = String::new();

    out.push_str("# RepoPilot Compare Report\n\n");
    out.push_str("## Overview\n\n");
    out.push_str("| Metric | Before | After | Delta |\n| --- | ---: | ---: | ---: |\n");

    let files_delta = summary.after_files as i64 - summary.before_files as i64;
    let loc_delta = summary.after_loc as i64 - summary.before_loc as i64;
    let new_count = summary.new_findings.len() as i64;
    let resolved_count = summary.resolved_findings.len() as i64;

    out.push_str(&format!(
        "| Files | {} | {} | {:+} |\n",
        summary.before_files, summary.after_files, files_delta
    ));
    out.push_str(&format!(
        "| LOC | {} | {} | {:+} |\n",
        summary.before_loc, summary.after_loc, loc_delta
    ));
    out.push_str(&format!(
        "| Findings | — | — | {:+} |\n\n",
        new_count - resolved_count
    ));

    if !summary.new_findings.is_empty() {
        out.push_str("## New findings\n\n");
        out.push_str("| Severity | Rule | Title |\n| --- | --- | --- |\n");
        for f in &summary.new_findings {
            out.push_str(&format!(
                "| {} | `{}` | {} |\n",
                f.severity_label(),
                f.rule_id,
                f.title
            ));
        }
        out.push('\n');
    }

    if !summary.resolved_findings.is_empty() {
        out.push_str("## Resolved findings\n\n");
        out.push_str("| Severity | Rule | Title |\n| --- | --- | --- |\n");
        for f in &summary.resolved_findings {
            out.push_str(&format!(
                "| {} | `{}` | {} |\n",
                f.severity_label(),
                f.rule_id,
                f.title
            ));
        }
        out.push('\n');
    }

    out
}
