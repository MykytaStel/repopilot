mod budget;
mod findings;
mod header;
mod hotfiles;
mod recommendations;

pub use budget::SectionBreakdown;

use crate::findings::types::Finding;
use crate::output::vibe::budget::BreakdownSection;
use crate::scan::types::ScanSummary;
use std::path::Path;
use std::str::FromStr;

pub const DEFAULT_TOKEN_BUDGET: usize = 4096;

pub struct VibeOptions {
    pub focus: Option<VibeCategory>,
    /// Approximate token budget (1 token ≈ 4 chars).
    pub budget_tokens: usize,
    pub no_header: bool,
    /// Suppress the AI task instruction block (implies no_header keeps it hidden too).
    pub no_task: bool,
}

impl Default for VibeOptions {
    fn default() -> Self {
        Self {
            focus: None,
            budget_tokens: DEFAULT_TOKEN_BUDGET,
            no_header: false,
            no_task: false,
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

/// Render the vibe context, discarding breakdown info.
pub fn render(summary: &ScanSummary, opts: &VibeOptions) -> String {
    let (content, _) = render_internal(summary, opts);
    content
}

/// Render the vibe context and return per-section token breakdown for display to the user.
pub fn render_with_breakdown(
    summary: &ScanSummary,
    opts: &VibeOptions,
) -> (String, SectionBreakdown) {
    render_internal(summary, opts)
}

fn render_internal(summary: &ScanSummary, opts: &VibeOptions) -> (String, SectionBreakdown) {
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
    let mut sections: Vec<BreakdownSection> = Vec::new();

    // Task instruction block (shown before header so AI sees intent first).
    if !opts.no_header && !opts.no_task {
        let pre = out.len();
        header::render_task_instruction(&mut out, &findings, summary);
        let added = out.len() - pre;
        if added > 0 {
            sections.push(BreakdownSection {
                label: "Task instruction".into(),
                tokens: added / 4,
            });
        }
    }

    // Project header (risk, tech stack, size, health).
    if !opts.no_header {
        let pre = out.len();
        header::render_header(&mut out, summary, &findings);
        sections.push(BreakdownSection {
            label: "Header".into(),
            tokens: (out.len() - pre) / 4,
        });
    }

    // Findings — 2-pass budget allocation.
    let compact = opts.no_header;
    let infos = findings::render_findings_by_category(&mut out, &findings, budget_chars, compact);
    for info in &infos {
        if info.chars_added > 0 {
            sections.push(BreakdownSection {
                label: info.label.clone(),
                tokens: info.chars_added / 4,
            });
        }
    }

    // Hot files coupling table (architecture-relevant only).
    if opts
        .focus
        .as_ref()
        .is_none_or(VibeCategory::includes_architecture_context)
    {
        let pre = out.len();
        hotfiles::render_hot_files(&mut out, summary);
        let added = out.len() - pre;
        if added > 0 {
            sections.push(BreakdownSection {
                label: "Hot Files".into(),
                tokens: added / 4,
            });
        }
    }

    // Recommendation clusters.
    let pre = out.len();
    recommendations::render_top_recommendations(&mut out, &findings);
    let rec_added = out.len() - pre;
    if rec_added > 0 {
        sections.push(BreakdownSection {
            label: "Recommendations".into(),
            tokens: rec_added / 4,
        });
    }

    // Footer.
    let pre = out.len();
    let content_len = out.len();
    recommendations::render_footer(
        &mut out,
        content_len,
        opts.budget_tokens,
        summary.scan_duration_us,
    );
    sections.push(BreakdownSection {
        label: "Footer".into(),
        tokens: (out.len() - pre) / 4,
    });

    let total_tokens = out.len() / 4;
    let shown_findings: usize = infos.iter().map(|i| i.shown).sum();
    let hidden_findings = findings.len().saturating_sub(shown_findings);

    let breakdown = SectionBreakdown {
        sections,
        total_tokens,
        budget_tokens: opts.budget_tokens,
        hidden_findings,
    };

    (out, breakdown)
}

pub(crate) fn project_name(summary: &ScanSummary) -> String {
    path_name(&summary.root_path)
        .or_else(|| {
            (summary.root_path == Path::new("."))
                .then(|| std::env::current_dir().ok())
                .flatten()
                .and_then(|path| path_name(&path))
        })
        .unwrap_or_else(|| "project".to_string())
}

fn path_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != ".")
        .map(str::to_string)
}

#[cfg(test)]
mod tests;
