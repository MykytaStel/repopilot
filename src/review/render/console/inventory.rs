use crate::output::DetailLevel;
use crate::review::model::ReviewReport;
use crate::review::render::helpers::render_ranges_suffix;
use std::collections::BTreeMap;
use std::path::{Component, Path};

const DEFAULT_CHANGED_FILE_LIMIT: usize = 12;

pub(super) fn render_changed_files(
    output: &mut String,
    report: &ReviewReport,
    detail: DetailLevel,
) {
    if detail == DetailLevel::Findings {
        render_area_counts(output, report);
    }

    output.push_str("\nChanged files:\n");
    if report.changed_files.is_empty() {
        output.push_str("  No changed files found\n");
        return;
    }

    let shown = match detail {
        DetailLevel::Findings => report.changed_files.len().min(DEFAULT_CHANGED_FILE_LIMIT),
        DetailLevel::Full => report.changed_files.len(),
        DetailLevel::Summary => 0,
    };
    for file in report.changed_files.iter().take(shown) {
        output.push_str(&format!(
            "  {:?} {}{}\n",
            file.status,
            file.path.display(),
            render_ranges_suffix(file)
        ));
    }

    if shown < report.changed_files.len() {
        output.push_str(&format!(
            "  ... {} more changed file(s); rerun with --detail full\n",
            report.changed_files.len() - shown
        ));
    }
}

pub(super) fn render_detail_hint(output: &mut String, detail: DetailLevel) {
    if detail != DetailLevel::Findings {
        return;
    }

    output.push_str("\nDetails:\n");
    output.push_str(
        "  Rerun with --detail full for the complete changed-file inventory, impact paths, and evidence.\n",
    );
}

fn render_area_counts(output: &mut String, report: &ReviewReport) {
    let mut counts = BTreeMap::<String, usize>::new();
    for file in &report.changed_files {
        *counts.entry(area_label(&file.path)).or_default() += 1;
    }

    output.push_str("\nChanged areas:\n");
    if counts.is_empty() {
        output.push_str("  None\n");
        return;
    }
    for (area, count) in counts {
        output.push_str(&format!("  {area}: {count}\n"));
    }
}

fn area_label(path: &Path) -> String {
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if components.len() <= 1 {
        "root".to_string()
    } else {
        components[0].clone()
    }
}
