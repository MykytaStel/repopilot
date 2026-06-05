use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileContentProvider, FileFacts, ScanFacts};

const RULE_ID: &str = "architecture.barrel-file-risk";
const MIN_RE_EXPORTS: usize = 8;
const MIN_WILDCARD_EXPORTS: usize = 3;
const MEDIUM_RE_EXPORTS: usize = 15;
const MEDIUM_WILDCARD_EXPORTS: usize = 6;

pub struct BarrelFileRiskAudit;

impl ProjectAudit for BarrelFileRiskAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        let classifier = crate::analysis::ArchitectureClassifier::new(&config.module_mappings);
        facts
            .files
            .iter()
            .filter(|file| {
                let context = classifier.classify(file);
                context.file_role == crate::analysis::FileRole::Production
            })
            .filter_map(find_barrel_file_risk)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BarrelStats {
    first_export_line: usize,
    re_exports: usize,
    wildcard_exports: usize,
}

fn find_barrel_file_risk(file: &FileFacts) -> Option<Finding> {
    if !is_index_file(file) {
        return None;
    }

    let content = read_file_content(file)?;
    let stats = collect_barrel_stats(&content)?;

    if stats.re_exports >= MIN_RE_EXPORTS || stats.wildcard_exports >= MIN_WILDCARD_EXPORTS {
        return Some(build_finding(file, stats));
    }

    None
}

fn is_index_file(file: &FileFacts) -> bool {
    file.path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let name_lower = name.to_lowercase();
            matches!(
                name_lower.as_str(),
                "index.ts"
                    | "index.tsx"
                    | "index.js"
                    | "index.jsx"
                    | "index.mts"
                    | "index.mjs"
                    | "public-api.ts"
                    | "public_api.ts"
                    | "exports.ts"
                    | "exports.js"
                    | "api.ts"
                    | "api.js"
                    | "main.ts"
                    | "main.js"
            )
        })
        .unwrap_or(false)
}

fn read_file_content(file: &FileFacts) -> Option<std::borrow::Cow<'_, str>> {
    FileContentProvider.content(file)
}

fn collect_barrel_stats(content: &str) -> Option<BarrelStats> {
    let mut first_export_line = None;
    let mut re_exports = 0usize;
    let mut wildcard_exports = 0usize;

    for (line_index, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if should_skip_line(trimmed) || !is_re_export_line(trimmed) {
            continue;
        }

        first_export_line.get_or_insert(line_index + 1);
        re_exports += 1;

        if is_wildcard_export(trimmed) {
            wildcard_exports += 1;
        }
    }

    Some(BarrelStats {
        first_export_line: first_export_line?,
        re_exports,
        wildcard_exports,
    })
}

fn should_skip_line(trimmed: &str) -> bool {
    trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
}

fn is_re_export_line(trimmed: &str) -> bool {
    is_wildcard_export(trimmed)
        || trimmed.starts_with("export {")
        || trimmed.starts_with("export type {")
        || trimmed.starts_with("export interface {")
}

fn is_wildcard_export(trimmed: &str) -> bool {
    trimmed.starts_with("export * from")
        || trimmed.starts_with("export type * from")
        || trimmed.starts_with("export * as ")
}

fn build_finding(file: &FileFacts, stats: BarrelStats) -> Finding {
    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        recommendation: Finding::recommendation_for_rule_id(RULE_ID),
        title: "Risky barrel file detected".to_string(),
        description: format!(
            "This index file re-exports {} modules, including {} wildcard exports. Large barrel files create unstable module hubs and can make dependency boundaries harder to understand.",
            stats.re_exports, stats.wildcard_exports
        ),
        category: FindingCategory::Architecture,
        severity: severity_for_stats(&stats),
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: file.path.clone(),
            line_start: stats.first_export_line,
            line_end: None,
            snippet: format!(
                "barrel re_exports={}, wildcard_exports={}",
                stats.re_exports, stats.wildcard_exports
            ),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn severity_for_stats(stats: &BarrelStats) -> Severity {
    if stats.re_exports >= MEDIUM_RE_EXPORTS || stats.wildcard_exports >= MEDIUM_WILDCARD_EXPORTS {
        Severity::Medium
    } else {
        Severity::Low
    }
}

#[cfg(test)]
mod tests;
