use std::fmt::Write as FmtWrite;

pub(super) fn render_footer(
    out: &mut String,
    content_len: usize,
    budget_tokens: usize,
    scan_duration_us: u64,
) {
    let approx_tokens = content_len / 4;
    let scan_ms = scan_duration_us / 1000;
    let _ = writeln!(
        out,
        "---\n*~{approx_tokens} tokens (budget: {budget_tokens}) · scanned in {scan_ms}ms — paste into Claude Code, Cursor, or ChatGPT to start fixing*"
    );
}
