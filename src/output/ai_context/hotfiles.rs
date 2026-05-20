use crate::graph::compute_metrics;
use crate::scan::types::ScanSummary;
use std::fmt::Write as FmtWrite;

pub(super) fn render_hot_files(out: &mut String, summary: &ScanSummary) {
    let graph = match &summary.coupling_graph {
        Some(g) => g,
        None => return,
    };

    let mut metrics = compute_metrics(graph);
    metrics.retain(|m| m.fan_in > 0);
    if metrics.is_empty() {
        return;
    }
    metrics.sort_by(|a, b| b.fan_in.cmp(&a.fan_in).then(a.fan_out.cmp(&b.fan_out)));

    let _ = writeln!(out, "## Hot Files (most imported)\n");
    let _ = writeln!(out, "| File | Fan-in | Fan-out | Instability |");
    let _ = writeln!(out, "|---|---|---|---|");
    for m in metrics.iter().take(5) {
        let path = m.path.display().to_string();
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.2} |",
            path, m.fan_in, m.fan_out, m.instability
        );
    }
    out.push('\n');
}
