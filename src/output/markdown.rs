use crate::baseline::diff::BaselineScanReport;
use crate::baseline::gate::CiGateResult;
use crate::frameworks::DetectedFramework;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::output::render_helpers::escape_table_cell;
use crate::scan::types::ScanSummary;

pub fn render(summary: &ScanSummary) -> String {
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");

    output.push_str("## Summary\n\n");
    output.push_str(&format!("- **Path:** `{}`\n", summary.root_path.display()));
    output.push_str(&format!("- **Files analyzed:** {}\n", summary.files_count));
    output.push_str(&format!(
        "- **Directories analyzed:** {}\n",
        summary.directories_count
    ));
    output.push_str(&format!(
        "- **Lines of code:** {}\n\n",
        summary.lines_of_code
    ));
    if summary.skipped_files_count > 0 {
        output.push_str(&format!(
            "- **Files skipped:** {} ({} bytes)\n\n",
            summary.skipped_files_count, summary.skipped_bytes
        ));
    }

    output.push_str("## Languages\n\n");

    if summary.languages.is_empty() {
        output.push_str("No languages detected.\n\n");
    } else {
        output.push_str("| Language | Files |\n");
        output.push_str("| --- | ---: |\n");

        for language in &summary.languages {
            output.push_str(&format!(
                "| {} | {} |\n",
                escape_table_cell(&language.name),
                language.files_count
            ));
        }

        output.push('\n');
    }

    render_frameworks_section(&mut output, &summary.detected_frameworks);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }

    output.push_str("## Findings\n\n");

    if summary.findings.is_empty() {
        output.push_str("No findings found.\n\n");
    } else {
        output.push_str("| Severity | Rule | Title | Evidence |\n");
        output.push_str("| --- | --- | --- | --- |\n");

        for finding in &summary.findings {
            let evidence = finding
                .evidence
                .first()
                .map(|evidence| {
                    format!(
                        "`{}:{}` — {}",
                        evidence.path.display(),
                        evidence.line_start,
                        evidence.snippet.trim()
                    )
                })
                .unwrap_or_else(|| "No evidence".to_string());

            output.push_str(&format!(
                "| {} | `{}` | {} | {} |\n",
                finding.severity_label(),
                finding.rule_id,
                escape_table_cell(&finding.title),
                escape_table_cell(&evidence)
            ));
        }

        output.push('\n');
    }

    output.push_str("## Markers\n\n");

    let marker_findings: Vec<_> = summary
        .findings
        .iter()
        .filter(|f| f.rule_id.starts_with("code-marker."))
        .collect();

    if marker_findings.is_empty() {
        output.push_str("No TODO/FIXME/HACK markers found.\n");
    } else {
        output.push_str("| Type | File | Line | Snippet |\n");
        output.push_str("| --- | --- | ---: | --- |\n");

        for finding in marker_findings {
            let kind = finding
                .rule_id
                .strip_prefix("code-marker.")
                .unwrap_or("")
                .to_uppercase();

            if let Some(ev) = finding.evidence.first() {
                output.push_str(&format!(
                    "| {} | `{}` | {} | {} |\n",
                    kind,
                    ev.path.display(),
                    ev.line_start,
                    escape_table_cell(ev.snippet.trim())
                ));
            }
        }
    }

    output
}

pub fn render_with_baseline(report: &BaselineScanReport, ci_gate: Option<&CiGateResult>) -> String {
    let summary = &report.summary;
    let mut output = String::new();

    output.push_str("# RepoPilot Scan Report\n\n");

    output.push_str("## Summary\n\n");
    output.push_str(&format!("- **Path:** `{}`\n", summary.root_path.display()));
    match &report.baseline_path {
        Some(path) => output.push_str(&format!("- **Baseline:** `{}`\n", path.display())),
        None => output.push_str("- **Baseline:** none (all findings treated as new)\n"),
    }
    output.push_str(&format!("- **Files analyzed:** {}\n", summary.files_count));
    output.push_str(&format!(
        "- **Directories analyzed:** {}\n",
        summary.directories_count
    ));
    output.push_str(&format!("- **Lines of code:** {}\n", summary.lines_of_code));
    if summary.skipped_files_count > 0 {
        output.push_str(&format!(
            "- **Files skipped:** {} ({} bytes)\n",
            summary.skipped_files_count, summary.skipped_bytes
        ));
    }
    output.push_str(&format!("- **New findings:** {}\n", report.new_count()));
    output.push_str(&format!(
        "- **Existing findings:** {}\n",
        report.existing_count()
    ));
    if let Some(ci_gate) = ci_gate {
        let status = if ci_gate.passed() { "passed" } else { "failed" };
        output.push_str(&format!(
            "- **CI gate:** {status} (`{}`)\n",
            ci_gate.label()
        ));
    }
    output.push('\n');

    output.push_str("## Languages\n\n");

    if summary.languages.is_empty() {
        output.push_str("No languages detected.\n\n");
    } else {
        output.push_str("| Language | Files |\n");
        output.push_str("| --- | ---: |\n");

        for language in &summary.languages {
            output.push_str(&format!(
                "| {} | {} |\n",
                escape_table_cell(&language.name),
                language.files_count
            ));
        }

        output.push('\n');
    }

    render_frameworks_section(&mut output, &summary.detected_frameworks);
    if let Some(rn) = &summary.react_native {
        render_react_native_section(&mut output, rn);
    }

    output.push_str("## Findings\n\n");

    if summary.findings.is_empty() {
        output.push_str("No findings found.\n\n");
    } else {
        output.push_str("| Severity | Baseline | Rule | Title | Evidence |\n");
        output.push_str("| --- | --- | --- | --- | --- |\n");

        for (index, finding) in summary.findings.iter().enumerate() {
            output.push_str(&format!(
                "| {} | {} | `{}` | {} | {} |\n",
                finding.severity_label(),
                report.finding_status(index).lowercase_label(),
                finding.rule_id,
                escape_table_cell(&finding.title),
                escape_table_cell(&render_evidence(finding))
            ));
        }

        output.push('\n');
    }

    output.push_str("## Markers\n\n");

    let marker_findings: Vec<_> = summary
        .findings
        .iter()
        .filter(|f| f.rule_id.starts_with("code-marker."))
        .collect();

    if marker_findings.is_empty() {
        output.push_str("No TODO/FIXME/HACK markers found.\n");
    } else {
        output.push_str("| Type | Baseline | File | Line | Snippet |\n");
        output.push_str("| --- | --- | --- | ---: | --- |\n");

        for finding in marker_findings {
            let kind = finding
                .rule_id
                .strip_prefix("code-marker.")
                .unwrap_or("")
                .to_uppercase();
            let status = summary
                .findings
                .iter()
                .position(|candidate| candidate == finding)
                .map(|index| report.finding_status(index).lowercase_label())
                .unwrap_or("new");

            if let Some(ev) = finding.evidence.first() {
                output.push_str(&format!(
                    "| {} | {} | `{}` | {} | {} |\n",
                    kind,
                    status,
                    ev.path.display(),
                    ev.line_start,
                    escape_table_cell(ev.snippet.trim())
                ));
            }
        }
    }

    output
}

fn render_react_native_section(output: &mut String, rn: &ReactNativeArchitectureProfile) {
    output.push_str("### React Native\n\n");

    let version = rn.react_native_version.as_deref().unwrap_or("unknown");
    output.push_str(&format!("- **Version:** {version}\n"));

    output.push_str(&format!(
        "- **iOS:** {}\n",
        if rn.has_ios {
            "detected"
        } else {
            "not detected"
        }
    ));
    output.push_str(&format!(
        "- **Android:** {}\n",
        if rn.has_android {
            "detected"
        } else {
            "not detected"
        }
    ));
    output.push_str(&format!(
        "- **Android New Architecture:** {}\n",
        format_tristate(rn.android_new_arch_enabled)
    ));
    output.push_str(&format!(
        "- **iOS New Architecture:** {}\n",
        format_tristate(rn.ios_new_arch_enabled)
    ));
    output.push_str(&format!(
        "- **Hermes:** {}\n",
        format_tristate(rn.hermes_enabled)
    ));
    output.push_str(&format!(
        "- **Codegen config:** {}\n\n",
        if rn.has_codegen_config {
            "found"
        } else {
            "missing"
        }
    ));
}

fn format_tristate(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "unknown",
    }
}

fn render_frameworks_section(output: &mut String, frameworks: &[DetectedFramework]) {
    if frameworks.is_empty() {
        return;
    }
    let labels: Vec<String> = frameworks.iter().map(|f| f.label()).collect();
    output.push_str("## Frameworks\n\n");
    output.push_str(&format!("{}\n\n", labels.join(" · ")));
}

fn render_evidence(finding: &crate::findings::types::Finding) -> String {
    finding
        .evidence
        .first()
        .map(|evidence| {
            format!(
                "`{}:{}` — {}",
                evidence.path.display(),
                evidence.line_start,
                evidence.snippet.trim()
            )
        })
        .unwrap_or_else(|| "No evidence".to_string())
}
