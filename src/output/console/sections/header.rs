pub(crate) fn render_header(output: &mut String, summary: &ScanSummary, stats: &ReportStats) {
    output.push_str("RepoPilot Scan\n");
    writeln!(output, "Version: {TOOL_VERSION}").unwrap();
    writeln!(output, "Path: {}", summary.root_path.display()).unwrap();
    if let Some(profile) = &summary.visibility_profile {
        writeln!(output, "Profile: {profile}").unwrap();
    }
    render_scope(output, summary);
    writeln!(
        output,
        "Risk: {} | Health score: {}/100 {}",
        stats.risk_label,
        stats.health_score,
        health_score_bar(stats.health_score)
    )
    .unwrap();
    writeln!(
        output,
        "Findings: {} visible ({:.1}/kloc)",
        stats.total_findings, stats.finding_density
    )
    .unwrap();
    if summary.hidden_suggestions_count > 0 {
        writeln!(
            output,
            "Hidden suggestions: {} strict-only suggestions",
            summary.hidden_suggestions_count
        )
        .unwrap();
        writeln!(
            output,
            "Note: {} strict-only suggestions hidden. Run with --profile strict or --include-maintainability to view.",
            summary.hidden_suggestions_count
        )
        .unwrap();
    }
    render_hidden_suggestions_breakdown(output, summary);
    render_local_feedback(output, summary);
    render_diagnostics(output, summary);
    writeln!(
        output,
        "Directories analyzed: {} | Non-empty lines: {}",
        summary.directories_count, summary.non_empty_lines
    )
    .unwrap();
    render_scan_input(output, summary);
    render_cache_telemetry(output, summary);
    if summary.scan_duration_us > 0 {
        writeln!(
            output,
            "Scan time: {:.2}s",
            summary.scan_duration_us as f64 / 1_000_000.0
        )
        .unwrap();
    }
    output.push('\n');
}

fn render_local_feedback(output: &mut String, summary: &ScanSummary) {
    let Some(feedback) = &summary.local_feedback else {
        return;
    };

    let path = feedback
        .feedback_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".repopilot/feedback.yml".to_string());
    writeln!(
        output,
        "Local feedback: {} suppression(s) loaded, {} finding(s) suppressed ({})",
        feedback.suppressions_loaded, feedback.suppressed_findings_count, path
    )
    .unwrap();

    if feedback.unmatched_suppressions_count > 0 {
        writeln!(
            output,
            "Local feedback unmatched: {} suppression(s)",
            feedback.unmatched_suppressions_count
        )
        .unwrap();
    }
    if feedback.invalid_suppressions_count > 0 {
        writeln!(
            output,
            "Local feedback invalid: {} suppression(s)",
            feedback.invalid_suppressions_count
        )
        .unwrap();
    }
}

fn render_diagnostics(output: &mut String, summary: &ScanSummary) {
    if summary.diagnostics.is_empty() {
        return;
    }

    output.push_str("Diagnostics:\n");
    for diagnostic in &summary.diagnostics {
        let path = diagnostic
            .path
            .as_ref()
            .map(|path| format!(" ({})", path.display()))
            .unwrap_or_default();
        writeln!(
            output,
            "  [{}] {}{}: {}",
            diagnostic_severity_label(diagnostic.severity),
            diagnostic.code,
            path,
            diagnostic.message
        )
        .unwrap();
    }
}

fn diagnostic_severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Error => "error",
    }
}

fn render_cache_telemetry(output: &mut String, summary: &ScanSummary) {
    let Some(cache) = &summary.cache_telemetry else {
        return;
    };

    output.push_str("Cache telemetry:\n");
    writeln!(
        output,
        " Cache hits: {:>7} | misses: {:>7} | skipped: {:>7} | hit rate: {}%",
        cache.hits, cache.misses, cache.skipped, cache.hit_rate_percent
    )
    .unwrap();

    if !cache.changed_file_reasons.is_empty() {
        let reasons = cache
            .changed_file_reasons
            .iter()
            .map(|item| format!("{} ({})", item.reason, item.count))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(output, " Changed file reasons: {reasons}").unwrap();
    }

    writeln!(
        output,
        " Cache timing: load {}ms | hash {}ms | lookup {}ms | reuse {}ms | miss scan {}ms | write {}ms | est. saved {}",
        cache.timings.load_us / 1000,
        cache.timings.file_hash_us / 1000,
        cache.timings.lookup_us / 1000,
        cache.timings.hit_reuse_us / 1000,
        cache.timings.miss_scan_us / 1000,
        cache.timings.write_us / 1000,
        format_optional_ms(cache.timings.estimated_time_saved_us)
    )
    .unwrap();

    for file in cache.changed_files.iter().take(8) {
        writeln!(
            output,
            "  {}: {} ({}, {})",
            file.path.display(),
            file.cache_status,
            file.change_reason,
            file.cache_reason
        )
        .unwrap();
    }

    if cache.changed_files.len() > 8 {
        writeln!(
            output,
            "  ... {} more changed file(s)",
            cache.changed_files.len() - 8
        )
        .unwrap();
    }
}

fn format_optional_ms(value: Option<u64>) -> String {
    value
        .map(|value| format!("{}ms", value / 1000))
        .unwrap_or_else(|| "n/a".to_string())
}

fn render_hidden_suggestions_breakdown(output: &mut String, summary: &ScanSummary) {
    if summary.hidden_suggestions.is_empty() {
        return;
    }

    output.push_str(
        "Hidden suggestions breakdown:
",
    );

    for item in summary.hidden_suggestions.iter().take(8) {
        writeln!(
            output,
            "  {:>4}  {} / {} / {} ({})",
            item.count, item.category, item.intent, item.rule_id, item.reason
        )
        .unwrap();
    }

    if summary.hidden_suggestions.len() > 8 {
        writeln!(
            output,
            "  ... {} more hidden group(s)",
            summary.hidden_suggestions.len() - 8
        )
        .unwrap();
    }
}

fn render_scope(output: &mut String, summary: &ScanSummary) {
    if summary.mode == crate::scan::types::ScanMode::Changed {
        let base = summary
            .base_ref
            .as_ref()
            .map(|base| format!(" since {base}"))
            .unwrap_or_else(|| " against HEAD".to_string());
        writeln!(
            output,
            "Scope: changed files{base} | changed files: {} | repo-level rules: skipped",
            summary.changed_files_count
        )
        .unwrap();
        return;
    }

    writeln!(output, "Scope: full scan | repo-level rules: included").unwrap();
}
