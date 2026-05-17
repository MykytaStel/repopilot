use crate::findings::types::Finding;
use crate::output::report_stats::severity_index;
use std::collections::BTreeMap;

pub(crate) fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

/// Aggregates per-package severity counts for workspace risk tables.
/// Returns a BTreeMap of package name → [critical, high, medium, low, info] counts.
pub(crate) fn workspace_package_counts<'a>(
    findings: &'a [Finding],
) -> BTreeMap<&'a str, [usize; 5]> {
    let mut table: BTreeMap<&'a str, [usize; 5]> = BTreeMap::new();
    for f in findings {
        if let Some(pkg) = f.workspace_package.as_deref() {
            let counts = table.entry(pkg).or_insert([0; 5]);
            counts[severity_index(f.severity)] += 1;
        }
    }
    table
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspacePackageRow {
    pub package: String,
    pub counts: [usize; 5],
    pub total: usize,
}

pub(crate) fn workspace_package_rows(findings: &[Finding]) -> Vec<WorkspacePackageRow> {
    workspace_package_counts(findings)
        .into_iter()
        .map(|(package, counts)| WorkspacePackageRow {
            package: package.to_string(),
            counts,
            total: counts.iter().sum(),
        })
        .collect()
}
