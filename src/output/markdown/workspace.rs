use crate::findings::types::Finding;
use crate::output::render_helpers::{escape_table_cell, workspace_package_rows};
use std::fmt::Write;

pub(crate) fn render_workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let rows = workspace_package_rows(findings);

    if rows.is_empty() {
        return;
    }

    output.push_str("## Workspace Risk Summary\n\n");
    output.push_str("| Package | Critical | High | Medium | Low | Info | Total |\n");
    output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for row in &rows {
        writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} | {} |",
            escape_table_cell(&row.package),
            row.counts[0],
            row.counts[1],
            row.counts[2],
            row.counts[3],
            row.counts[4],
            row.total
        )
        .unwrap();
    }
    output.push('\n');
}
