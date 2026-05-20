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
    fn audit(&self, facts: &ScanFacts, _config: &ScanConfig) -> Vec<Finding> {
        facts
            .files
            .iter()
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
            matches!(
                name,
                "index.ts" | "index.tsx" | "index.js" | "index.jsx" | "index.mts" | "index.mjs"
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
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_large_index_barrel_file() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("index.ts");

        fs::write(
            &file_path,
            [
                "export { A } from './a';",
                "export { B } from './b';",
                "export { C } from './c';",
                "export { D } from './d';",
                "export { E } from './e';",
                "export { F } from './f';",
                "export { G } from './g';",
                "export { H } from './h';",
            ]
            .join("\n"),
        )
        .expect("write file");

        let findings =
            BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_ID);
        assert_eq!(findings[0].severity, Severity::Low);
        assert_eq!(findings[0].evidence[0].line_start, 1);
    }

    #[test]
    fn detects_wildcard_heavy_barrel_file_as_medium() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("index.ts");

        fs::write(
            &file_path,
            [
                "export * from './a';",
                "export * from './b';",
                "export * from './c';",
                "export * from './d';",
                "export * from './e';",
                "export * from './f';",
            ]
            .join("\n"),
        )
        .expect("write file");

        let findings =
            BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn ignores_small_index_barrel_file() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("index.ts");

        fs::write(
            &file_path,
            [
                "export { A } from './a';",
                "export { B } from './b';",
                "export { C } from './c';",
            ]
            .join("\n"),
        )
        .expect("write file");

        let findings =
            BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_non_index_file() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("exports.ts");

        fs::write(
            &file_path,
            [
                "export * from './a';",
                "export * from './b';",
                "export * from './c';",
                "export * from './d';",
            ]
            .join("\n"),
        )
        .expect("write file");

        let findings =
            BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_commented_exports() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("index.ts");

        fs::write(
            &file_path,
            [
                "// export * from './a';",
                "// export * from './b';",
                "// export * from './c';",
                "export { Real } from './real';",
            ]
            .join("\n"),
        )
        .expect("write file");

        let findings =
            BarrelFileRiskAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert!(findings.is_empty());
    }

    fn facts_for_file(path: std::path::PathBuf) -> ScanFacts {
        ScanFacts {
            files: vec![FileFacts {
                path,
                language: None,
                non_empty_lines: 1,
                branch_count: 0,
                imports: vec![],
                content: None,
                has_inline_tests: false,
            }],
            ..ScanFacts::default()
        }
    }
}
