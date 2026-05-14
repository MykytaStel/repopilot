use crate::findings::types::{Finding, Severity};
use std::io::Write as IoWrite;

pub fn category_weight(findings: &[&Finding]) -> usize {
    let w: usize = findings
        .iter()
        .map(|f| match f.severity {
            Severity::Critical => 4,
            Severity::High => 2,
            Severity::Medium => 1,
            Severity::Low | Severity::Info => 0,
        })
        .sum();
    w.max(1)
}

pub struct CategoryAllocation {
    /// Character budget for findings content in this category (not counting category header).
    pub chars: usize,
    /// How many snippet lines to show per finding: 3=full, 1=compressed, 0=minimal (no snippet).
    pub snippet_lines: usize,
}

/// Allocate `findings_budget_chars` across categories proportionally to their severity weights.
pub fn allocate_categories(
    weights: &[usize],
    findings_budget_chars: usize,
) -> Vec<CategoryAllocation> {
    let total_weight: usize = weights.iter().sum();
    if total_weight == 0 {
        return weights
            .iter()
            .map(|_| CategoryAllocation {
                chars: 0,
                snippet_lines: 0,
            })
            .collect();
    }
    weights
        .iter()
        .map(|&w| {
            let chars = findings_budget_chars * w / total_weight;
            let snippet_lines = if chars >= 800 {
                3
            } else if chars >= 300 {
                1
            } else {
                0
            };
            CategoryAllocation {
                chars,
                snippet_lines,
            }
        })
        .collect()
}

pub struct BreakdownSection {
    pub label: String,
    pub tokens: usize,
}

pub struct SectionBreakdown {
    pub sections: Vec<BreakdownSection>,
    pub total_tokens: usize,
    pub budget_tokens: usize,
    pub hidden_findings: usize,
}

impl SectionBreakdown {
    pub fn render_stderr(&self) {
        let stderr = std::io::stderr();
        let mut h = stderr.lock();

        let bar_width = 16usize;
        let _ = writeln!(
            h,
            "\n── Token Budget ──────────────────────────────────────"
        );
        for section in &self.sections {
            let filled = if self.budget_tokens > 0 {
                (section.tokens * bar_width / self.budget_tokens).min(bar_width)
            } else {
                0
            };
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);
            let label_trunc = if section.label.len() > 26 {
                &section.label[..26]
            } else {
                &section.label
            };
            let _ = writeln!(h, "  {:<26} {} {:>5}", label_trunc, bar, section.tokens);
        }
        let _ = writeln!(h, "  ──────────────────────────────────────────────────");
        let status = if self.total_tokens <= self.budget_tokens {
            "✅"
        } else {
            "⚠️ "
        };
        let _ = writeln!(
            h,
            "  Total: {} / {} tokens  {}",
            self.total_tokens, self.budget_tokens, status
        );
        if self.hidden_findings > 0 {
            let _ = writeln!(
                h,
                "  Not shown: {} finding(s) — use --focus to narrow scope",
                self.hidden_findings
            );
        }
        let _ = writeln!(
            h,
            "──────────────────────────────────────────────────────\n"
        );
    }
}
