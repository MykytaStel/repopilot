fn render_cluster_plan(out: &mut String, cluster: &RuleCluster<'_>, index: usize) {
    let count_note = if cluster.findings.len() > 1 {
        format!(" ({} findings)", cluster.findings.len())
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "\n#### {index}. [{}] {}{}",
        cluster.severity.label(),
        cluster.title,
        count_note
    );
    let _ = writeln!(out, "- Rule: `{}`", cluster.rule_id);
    if let Some(scope) = &cluster.scope
        && scope != "."
    {
        let _ = writeln!(out, "- Area: `{scope}`");
    }
    let _ = writeln!(
        out,
        "- Priority: {} (max risk {}/100)",
        cluster.priority.label(),
        cluster.max_score
    );

    let examples = example_locations(&cluster.findings, 3);
    if !examples.is_empty() {
        let _ = writeln!(out, "- Examples: {}", examples.join(", "));
    }

    let first = cluster.findings[0];
    if !first.description.is_empty() {
        let _ = writeln!(out, "- Why: {}", first.description);
    }

    let _ = writeln!(out, "- Fix: {}", finding_recommendation(first));
}

/// Verification checklist for the handoff — what to run after applying fixes.
/// Part of the AI task guidance, so the unified context emits it only when the
/// task block is enabled.
pub(crate) fn render_verification(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Verify\n\n- Run `repopilot scan . --min-severity high` after P0/P1 fixes.\n- Run `repopilot review . --base origin/main --fail-on new-high` before merging.\n- Refresh a baseline only when the remaining findings are accepted technical debt."
    );
}
