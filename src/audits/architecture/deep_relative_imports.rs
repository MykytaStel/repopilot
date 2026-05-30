use crate::audits::traits::ProjectAudit;
use crate::findings::types::{Evidence, Finding, FindingCategory, Severity};
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileContentProvider, FileFacts, ScanFacts};

const RULE_ID: &str = "architecture.deep-relative-imports";
const MIN_DEEP_RELATIVE_DEPTH: usize = 3;

pub struct DeepRelativeImportsAudit;

impl ProjectAudit for DeepRelativeImportsAudit {
    fn audit(&self, facts: &ScanFacts, config: &ScanConfig) -> Vec<Finding> {
        let classifier = crate::analysis::ArchitectureClassifier::new(&config.module_mappings);
        facts
            .files
            .iter()
            .filter(|file| {
                let context = classifier.classify(file);
                context.file_role == crate::analysis::FileRole::Production
            })
            .filter_map(find_deep_relative_import)
            .collect()
    }
}

fn find_deep_relative_import(file: &FileFacts) -> Option<Finding> {
    let content = read_file_content(file)?;

    for (line_index, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if should_skip_line(trimmed) || !looks_like_import_line(trimmed) {
            continue;
        }

        let depth = max_relative_import_depth(trimmed);

        if depth >= MIN_DEEP_RELATIVE_DEPTH {
            return Some(build_finding(file, line_index + 1, trimmed, depth));
        }
    }

    None
}

fn read_file_content(file: &FileFacts) -> Option<std::borrow::Cow<'_, str>> {
    FileContentProvider.content(file)
}

fn should_skip_line(trimmed: &str) -> bool {
    trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with('#')
}

fn looks_like_import_line(trimmed: &str) -> bool {
    trimmed.starts_with("import ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("pub use ")
        || trimmed.contains("require(")
        || trimmed.contains("import(")
}

fn max_relative_import_depth(trimmed: &str) -> usize {
    quoted_relative_depth(trimmed).max(rust_super_depth(trimmed))
}

fn quoted_relative_depth(trimmed: &str) -> usize {
    extract_quoted_segments(trimmed)
        .iter()
        .map(|segment| relative_path_depth(segment))
        .max()
        .unwrap_or(0)
}

fn extract_quoted_segments(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();

    for delimiter in ['"', '\'', '`'] {
        let parts: Vec<&str> = line.split(delimiter).collect();
        let mut index = 1;

        while index < parts.len() {
            segments.push(parts[index]);
            index += 2;
        }
    }

    segments
}

fn relative_path_depth(value: &str) -> usize {
    let mut depth = 0;
    let mut rest = value;

    while rest.starts_with("../") {
        depth += 1;
        rest = &rest[3..];
    }

    depth
}

fn rust_super_depth(trimmed: &str) -> usize {
    if !(trimmed.starts_with("use ") || trimmed.starts_with("pub use ")) {
        return 0;
    }

    trimmed.matches("super::").count()
}

fn build_finding(file: &FileFacts, line_number: usize, snippet: &str, depth: usize) -> Finding {
    Finding {
        id: String::new(),
        rule_id: RULE_ID.to_string(),
        recommendation: Finding::recommendation_for_rule_id(RULE_ID),
        title: "Deep relative import found".to_string(),
        description: format!(
            "This file imports across {depth} parent levels. Deep relative imports make modules fragile during refactors and usually indicate missing boundaries, aliases, or facade modules."
        ),
        category: FindingCategory::Architecture,
        severity: severity_for_depth(depth),
        confidence: Default::default(),
        evidence: vec![Evidence {
            path: file.path.clone(),
            line_start: line_number,
            line_end: None,
            snippet: snippet.to_string(),
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn severity_for_depth(depth: usize) -> Severity {
    if depth >= 4 {
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
    fn detects_deep_javascript_relative_import() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("feature.ts");

        fs::write(
            &file_path,
            "import api from \"../../../shared/api\";\nexport const value = api;\n",
        )
        .expect("write file");

        let findings =
            DeepRelativeImportsAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_ID);
        assert_eq!(findings[0].severity, Severity::Low);
        assert_eq!(findings[0].evidence[0].line_start, 1);
    }

    #[test]
    fn detects_very_deep_javascript_relative_import_as_medium() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("feature.ts");

        fs::write(
            &file_path,
            "const config = require(\"../../../../config\");\n",
        )
        .expect("write file");

        let findings =
            DeepRelativeImportsAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn ignores_shallow_relative_imports() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("feature.ts");

        fs::write(&file_path, "import helper from \"../../helper\";\n").expect("write file");

        let findings =
            DeepRelativeImportsAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert!(findings.is_empty());
    }

    #[test]
    fn detects_deep_rust_super_import() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("mod.rs");

        fs::write(&file_path, "use super::super::super::domain;\n").expect("write file");

        let findings =
            DeepRelativeImportsAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_ID);
    }

    #[test]
    fn ignores_commented_imports() {
        let temp = tempdir().expect("temp dir");
        let file_path = temp.path().join("feature.ts");

        fs::write(&file_path, "// import api from \"../../../shared/api\";\n").expect("write file");

        let findings =
            DeepRelativeImportsAudit.audit(&facts_for_file(file_path), &ScanConfig::default());

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
