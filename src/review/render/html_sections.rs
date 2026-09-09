use super::super::helpers::{
    render_ranges, verification_duration_evidence, verification_proof_summary,
};
use super::escape;
use crate::review::model::ReviewReport;
use crate::review::proof::ChangeProof;
use crate::review::signals::tiered::TieredSignals;

const DETAIL_LIMIT: usize = 20;

pub(super) fn render_change_map(report: &ReviewReport, proof: &ChangeProof) -> String {
    let files = if report.changed_files.is_empty() {
        "<p class=\"empty\">No changed files found.</p>".to_string()
    } else {
        let rows = report
            .changed_files
            .iter()
            .map(|file| {
                format!(
                    "<tr><td><span class=\"status {}\">{:?}</span></td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                    format!("{:?}", file.status).to_ascii_lowercase(),
                    file.status,
                    escape(&file.path.to_string_lossy()),
                    escape(&render_ranges(file)),
                    render_hunks(file)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<table><thead><tr><th>Status</th><th>Changed file</th><th>Ranges</th><th>Before / after</th></tr></thead><tbody>{rows}</tbody></table>"
        )
    };
    let contracts = if proof.contract_deltas.is_empty() {
        "<p class=\"empty\">No typed contract deltas were detected in this scope.</p>".to_string()
    } else {
        let rows = proof
            .contract_deltas
            .iter()
            .map(|delta| {
                format!(
                    "<tr><td>{:?}</td><td>{:?}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>",
                    delta.family,
                    delta.change,
                    escape(&delta.exporter_path),
                    escape(&delta.consumer_path),
                    escape(&delta.evidence)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<table><thead><tr><th>Family</th><th>Change</th><th>Contract</th><th>Consumer</th><th>Evidence</th></tr></thead><tbody>{rows}</tbody></table>"
        )
    };
    format!(
        r###"<section id="change-map"><h2>Change Map</h2>
<p class="muted"><a href="#proof-card" data-jump="proof-card">Back to Proof Card</a> · <a href="#contract-map" data-jump="contract-map">Contract / consumer map</a> · <a href="#impact-paths" data-jump="impact-paths">Impact paths</a></p>
<h3 id="changed-files">Changed files</h3>{files}
<h3 id="contract-map">Contract / consumer map</h3>{contracts}
</section>"###
    )
}

fn render_hunks(file: &crate::review::diff::ChangedFile) -> String {
    if file.hunks.is_empty() {
        return "<span class=\"muted\">n/a</span>".to_string();
    }
    let details = file
        .hunks
        .iter()
        .map(|hunk| {
            let before = if hunk.removed_lines.is_empty() {
                "(none)".to_string()
            } else {
                hunk.removed_lines
                    .iter()
                    .map(|line| escape(line))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let after = if hunk.added_lines.is_empty() {
                "(none)".to_string()
            } else {
                hunk.added_lines
                    .iter()
                    .map(|line| escape(line))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let header = hunk
                .header
                .as_deref()
                .map(|header| format!(" · {}", escape(header)))
                .unwrap_or_default();
            format!(
                "<details><summary>hunk{header}</summary><div class=\"diff-grid\"><div class=\"diff-side\"><strong>Before</strong><pre>{before}</pre></div><div class=\"diff-side\"><strong>After</strong><pre>{after}</pre></div></div></details>"
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!("<div>{details}</div>")
}

pub(super) fn render_impact(report: &ReviewReport) -> String {
    let impact = &report.impact_paths;
    if impact.files.is_empty() && report.blast_radius.is_empty() {
        return String::new();
    }
    let paths = impact
        .files
        .iter()
        .map(|file| {
            let direct = file
                .direct_dependents
                .iter()
                .map(|path| {
                    format!(
                        "<li>direct: <code>{}</code></li>",
                        escape(&path.to_string_lossy())
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            let transitive = file
                .transitive_dependents
                .iter()
                .map(|path| {
                    format!(
                        "<li>transitive: <code>{}</code></li>",
                        escape(&path.to_string_lossy())
                    )
                })
                .collect::<Vec<_>>()
                .join("");
            format!(
                "<li><code>{}</code><ul>{direct}{transitive}</ul></li>",
                escape(&file.path.to_string_lossy())
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let blast = report
        .blast_radius
        .iter()
        .map(|path| format!("<li><code>{}</code></li>", escape(&path.to_string_lossy())))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<section class="panel" id="impact-paths"><h2>Impact Paths</h2>
<p>{} impacted file(s) across {} director{} (depth {}).</p>
<ul>{paths}</ul>
{}{}
</section>"#,
        impact.affected_surface.impacted_files,
        impact.affected_surface.affected_directories.len(),
        if impact.affected_surface.affected_directories.len() == 1 {
            "y"
        } else {
            "ies"
        },
        impact.depth,
        if blast.is_empty() {
            ""
        } else {
            "<h3>Blast radius</h3><ul>"
        },
        if blast.is_empty() {
            "".to_string()
        } else {
            format!("{blast}</ul>")
        },
    )
}

pub(super) fn render_signals(tiered: &TieredSignals) -> String {
    if tiered.is_empty() {
        return String::new();
    }
    let mut remaining = DETAIL_LIMIT;
    let mut sections = String::new();
    for (label, signals) in [
        ("Definitely sensitive", &tiered.definitely),
        ("Maybe sensitive", &tiered.maybe),
        ("Large diff / noise", &tiered.noise),
    ] {
        if remaining == 0 || signals.is_empty() {
            continue;
        }
        sections.push_str(&format!("<h3>{label}</h3>"));
        for signal in signals
            .iter()
            .filter(|signal| !signal.suppressed)
            .take(remaining)
        {
            let location = signal.line.map_or_else(
                || signal.path.clone(),
                |line| format!("{}:{line}", signal.path),
            );
            sections.push_str(&format!(
                "<article class=\"signal\"><strong>{}</strong><div class=\"signal-meta\"><code>{}</code> · {} · reach {}</div><div>{}</div></article>",
                escape(&signal.headline),
                escape(&location),
                escape(&format!("{:?}", signal.family)),
                signal.blast_radius,
                escape(signal.detail.as_deref().unwrap_or("No further detail."))
            ));
            remaining -= 1;
        }
    }
    if sections.is_empty() {
        return String::new();
    }
    let total_visible = tiered
        .definitely
        .iter()
        .chain(tiered.maybe.iter())
        .chain(tiered.noise.iter())
        .filter(|signal| !signal.suppressed)
        .count();
    if total_visible > DETAIL_LIMIT {
        sections.push_str(&format!(
            "<p class=\"muted\">{} additional signal(s) omitted; use JSON for the full list.</p>",
            total_visible - DETAIL_LIMIT
        ));
    }
    format!(
        "<section class=\"panel\" id=\"review-signals\"><h2>Review Signals</h2>{sections}</section>"
    )
}

pub(super) fn render_verification(report: &ReviewReport, proof: &ChangeProof) -> String {
    if report.verification.is_empty() {
        return String::new();
    }
    let rows = report
        .verification
        .iter()
        .map(|outcome| {
            let source = if outcome.reused { "cached" } else { "executed" };
            format!(
                "<tr><td><code>{}</code></td><td>{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape(&outcome.check_id),
                outcome.status,
                source,
                verification_duration_evidence(outcome),
                outcome.exit_code.map_or_else(|| "-".to_string(), |code| code.to_string()),
                if outcome.revision_compatible { "compatible" } else { "changed" }
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        "<section class=\"panel\" id=\"verification\"><h2>Verification</h2><p class=\"muted\">{}</p><table><thead><tr><th>Check</th><th>Status</th><th>Source</th><th>Duration</th><th>Exit</th><th>Revision</th></tr></thead><tbody>{rows}</tbody></table></section>",
        escape(&verification_proof_summary(report, proof.obligations))
    )
}

pub(super) fn render_findings(report: &ReviewReport) -> String {
    let total = report.summary.artifacts.findings.len();
    let findings = report
        .summary
        .artifacts
        .findings
        .iter()
        .take(DETAIL_LIMIT)
        .map(|finding| {
            format!(
                "<article class=\"signal\"><strong>{}</strong><div class=\"signal-meta\">{} · {} · <code>{}</code></div></article>",
                escape(&finding.title),
                escape(finding.severity_label()),
                escape(finding.confidence_label()),
                escape(&finding.rule_id)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    if findings.is_empty() {
        return "<section class=\"panel\" id=\"findings\"><h2>Findings</h2><p class=\"empty\">No findings.</p></section>".to_string();
    }
    let omitted = if total > DETAIL_LIMIT {
        format!(
            "<p class=\"muted\">{} additional finding(s) omitted; use JSON for the full list.</p>",
            total - DETAIL_LIMIT
        )
    } else {
        String::new()
    };
    format!(
        "<section class=\"panel\" id=\"findings\"><h2>Findings</h2>{findings}{omitted}</section>"
    )
}
