use crate::output::html::escape::escape_html;
use crate::scan::types::ScanSummary;

pub(super) fn render_frameworks_section(summary: &ScanSummary) -> String {
    if summary.artifacts.detected_frameworks.is_empty()
        && summary.artifacts.framework_projects.is_empty()
        && summary.artifacts.react_native.is_none()
    {
        return String::new();
    }

    let labels: Vec<String> = summary
        .artifacts
        .detected_frameworks
        .iter()
        .map(|f| format!(r#"<li class="pill">{}</li>"#, escape_html(&f.label())))
        .collect();
    let mut output = String::from("<h2>Frameworks</h2>");
    if !labels.is_empty() {
        output.push_str(&format!(
            r#"<ul class="inline-list">{}</ul>"#,
            labels.join("")
        ));
    }

    render_framework_projects(summary, &mut output);
    render_react_native(summary, &mut output);

    output
}

fn render_framework_projects(summary: &ScanSummary, output: &mut String) {
    let nested_projects: Vec<_> = summary
        .artifacts
        .framework_projects
        .iter()
        .filter(|project| project.path.as_path() != std::path::Path::new("."))
        .collect();
    if nested_projects.is_empty() {
        return;
    }

    output.push_str("<h3>Framework Projects</h3><table><thead><tr><th>Path</th><th>Frameworks</th></tr></thead><tbody>");
    for project in nested_projects {
        let frameworks = project
            .frameworks
            .iter()
            .map(|f| escape_html(&f.label()))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!(
            "<tr><td><code>{}</code></td><td>{}</td></tr>",
            escape_html(&project.path.to_string_lossy()),
            frameworks
        ));
    }
    output.push_str("</tbody></table>");
}

fn render_react_native(summary: &ScanSummary, output: &mut String) {
    if let Some(rn) = &summary.artifacts.react_native {
        output.push_str(&format!(
            "<div class=\"panel\"><h3>React Native</h3><p class=\"meta\">Version {} | Android New Architecture {} | iOS New Architecture {} | Hermes {} | Codegen {}</p></div>",
            escape_html(rn.react_native_version.as_deref().unwrap_or("unknown")),
            escape_html(format_tristate(rn.android_new_arch_enabled)),
            escape_html(format_tristate(rn.ios_new_arch_enabled)),
            escape_html(format_tristate(rn.hermes_enabled)),
            if rn.has_codegen_config { "found" } else { "missing" }
        ));
    }
}

fn format_tristate(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}
