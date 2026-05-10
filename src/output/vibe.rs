mod findings;
mod header;
mod hotfiles;
mod recommendations;

use crate::findings::types::Finding;
use crate::scan::types::ScanSummary;
use std::str::FromStr;

pub const DEFAULT_TOKEN_BUDGET: usize = 4096;

pub struct VibeOptions {
    pub focus: Option<VibeCategory>,
    /// Approximate token budget (1 token ≈ 4 chars).
    pub budget_tokens: usize,
    pub no_header: bool,
}

impl Default for VibeOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: DEFAULT_TOKEN_BUDGET,
            no_header: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VibeCategory {
    Security,
    Architecture,
    Quality,
    Framework,
    All,
}

impl FromStr for VibeCategory {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "security" => Ok(Self::Security),
            "arch" | "architecture" => Ok(Self::Architecture),
            "quality" => Ok(Self::Quality),
            "framework" => Ok(Self::Framework),
            "all" => Ok(Self::All),
            _ => Err(()),
        }
    }
}

impl VibeCategory {
    pub(crate) fn matches(&self, category: &crate::findings::types::FindingCategory) -> bool {
        use crate::findings::types::FindingCategory;
        match self {
            VibeCategory::All => true,
            VibeCategory::Security => matches!(category, FindingCategory::Security),
            VibeCategory::Architecture => matches!(category, FindingCategory::Architecture),
            VibeCategory::Quality => matches!(
                category,
                FindingCategory::CodeQuality | FindingCategory::Testing
            ),
            VibeCategory::Framework => matches!(category, FindingCategory::Framework),
        }
    }

    fn includes_architecture_context(&self) -> bool {
        matches!(self, VibeCategory::Architecture | VibeCategory::All)
    }
}

pub fn render(summary: &ScanSummary, opts: &VibeOptions) -> String {
    let budget_chars = opts.budget_tokens * 4;

    let findings: Vec<&Finding> = summary
        .findings
        .iter()
        .filter(|f| {
            opts.focus
                .as_ref()
                .is_none_or(|focus| focus.matches(&f.category))
        })
        .collect();

    let mut out = String::new();

    if !opts.no_header {
        header::render_header(&mut out, summary, &findings);
    }

    findings::render_findings_by_category(&mut out, &findings, budget_chars, opts.no_header);
    if opts
        .focus
        .as_ref()
        .is_none_or(VibeCategory::includes_architecture_context)
    {
        hotfiles::render_hot_files(&mut out, summary);
    }
    recommendations::render_top_recommendations(&mut out, &findings);

    let content_len = out.len();
    recommendations::render_footer(
        &mut out,
        content_len,
        opts.budget_tokens,
        summary.scan_duration_us,
    );

    out
}

#[cfg(test)]
mod tests;
