use crate::findings::types::Finding;
use crate::output::render_helpers::workspace_package_rows;
use std::fmt::Write;

pub(crate) fn workspace_risk_table(output: &mut String, findings: &[Finding]) {
    let rows = workspace_package_rows(findings);

    if rows.is_empty() {
        return;
    }

    let name_width = rows
        .iter()
        .map(|row| row.package.len())
        .max()
        .unwrap_or(7)
        .max(7);

    output.push_str("Workspace Risk Summary:\n");
    writeln!(
        output,
        "  {:<width$}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}",
        "Package",
        "Crit",
        "High",
        "Med",
        "Low",
        "Info",
        "Total",
        width = name_width
    )
    .unwrap();
    writeln!(
        output,
        "  {:-<width$}  -----  -----  -----  -----  -----  -----",
        "",
        width = name_width
    )
    .unwrap();
    for row in &rows {
        writeln!(
            output,
            "  {:<width$}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}  {:>5}",
            row.package.as_str(),
            row.counts[0],
            row.counts[1],
            row.counts[2],
            row.counts[3],
            row.counts[4],
            row.total,
            width = name_width
        )
        .unwrap();
    }
    output.push('\n');
}
