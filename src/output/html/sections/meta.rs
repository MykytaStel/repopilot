pub(super) fn render_scan_meta(summary: &ScanSummary) -> String {
    if summary.mode == crate::scan::types::ScanMode::Changed {
        let base = summary
            .base_ref
            .as_ref()
            .map(|base| format!(" since <code>{}</code>", escape_html(base)))
            .unwrap_or_else(|| " against <code>HEAD</code>".to_string());
        let mut output = format!(
            r#"<p class="meta">Scope: changed files{base}; {} changed file(s); repo-level rules skipped.</p>"#,
            summary.changed_files_count
        );
        if let Some(cache) = &summary.cache_telemetry {
            output.push_str(&format!(
                r#"<p class="meta">Cache: {} hit(s), {} miss(es), {} skipped, {}% hit rate.</p>"#,
                cache.hits, cache.misses, cache.skipped, cache.hit_rate_percent
            ));
        }
        return output;
    }

    r#"<p class="meta">Scope: full scan; repo-level rules included.</p>"#.to_string()
}
