use crate::findings::types::{Finding, Severity};
use crate::rules::lookup_rule_metadata;
use std::cmp::Reverse;
use std::fmt::Write as FmtWrite;

pub(super) fn render_top_recommendations(out: &mut String, findings: &[&Finding]) {
    let mut top: Vec<&Finding> = findings
        .iter()
        .copied()
        .filter(|f| {
            f.severity >= Severity::High
                && (lookup_rule_metadata(&f.rule_id)
                    .and_then(|m| m.recommendation)
                    .is_some()
                    || !f.description.is_empty())
        })
        .collect();

    top.sort_by_key(|finding| Reverse(finding.severity));
    top.dedup_by_key(|f| &f.rule_id);
    let top: Vec<_> = top.into_iter().take(5).collect();

    if top.is_empty() {
        return;
    }

    let _ = writeln!(out, "## Top Recommendations");
    out.push('\n');

    for (i, finding) in top.iter().enumerate() {
        let location = finding.evidence.first().map(|e| {
            let path = e.path.display().to_string();
            if e.line_start > 0 {
                format!("{path}:{}", e.line_start)
            } else {
                path
            }
        });

        let rec = lookup_rule_metadata(&finding.rule_id)
            .and_then(|m| m.recommendation)
            .unwrap_or(finding.description.as_str());

        let loc_note = location
            .as_deref()
            .map(|l| format!(" (`{l}`)"))
            .unwrap_or_default();

        let _ = writeln!(out, "{}. **{}**{loc_note} — {rec}", i + 1, finding.title);
    }
    out.push('\n');
}

pub(super) fn render_footer(
    out: &mut String,
    content_len: usize,
    budget_tokens: usize,
    scan_duration_us: u64,
) {
    let approx_tokens = content_len / 4;
    let scan_ms = scan_duration_us / 1000;
    let _ = writeln!(
        out,
        "---\n*~{approx_tokens} tokens (budget: {budget_tokens}) · scanned in {scan_ms}ms — paste into Claude Code, Cursor, or ChatGPT to start fixing*"
    );
}
