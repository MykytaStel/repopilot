use crate::analysis::api_contract::RemovedExportOccurrence;
use crate::analysis::symbols::SymbolKind;
use crate::findings::provenance::{AnalysisScope, FindingProvenance};
use crate::findings::types::{Evidence, Finding, FindingCategory};

pub(super) fn occurrence_to_finding(occurrence: &RemovedExportOccurrence) -> Finding {
    let (description, snippet) = describe(occurrence);
    Finding {
        rule_id: "behavioral.removed-export-still-imported".to_string(),
        description,
        category: FindingCategory::CodeQuality,
        evidence: vec![Evidence {
            path: occurrence.importer_path.clone(),
            line_start: occurrence.line_start,
            line_end: Some(occurrence.line_end),
            snippet,
        }],
        provenance: FindingProvenance {
            analysis_scope: AnalysisScope::GitDiff,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn describe(occurrence: &RemovedExportOccurrence) -> (String, String) {
    let symbol_kind = match occurrence.symbol_kind {
        SymbolKind::Value => "value",
        SymbolKind::Type => "type",
    };
    let exporter = occurrence
        .exporter_path
        .to_string_lossy()
        .replace('\\', "/");
    let import_form = if occurrence.exported_name == "default" {
        "default"
    } else {
        "named"
    };
    let rust = occurrence
        .importer_path
        .extension()
        .is_some_and(|ext| ext == "rs");
    let evidence_form = if rust { "Rust direct use" } else { "import" };
    let relation = if rust {
        "referenced directly as"
    } else {
        "imported as local binding"
    };
    let snippet = format!(
        "{import_form} {symbol_kind} {evidence_form} '{} as {}' from '{}' resolves to '{}'",
        occurrence.exported_name, occurrence.local_name, occurrence.module_specifier, exporter,
    );

    let snippet = if rust {
        format!(
            "{snippet} (bytes {}..{})",
            occurrence.byte_start, occurrence.byte_end
        )
    } else {
        snippet
    };
    let description = format!(
        "Removed {symbol_kind} export '{}' from {exporter} remains {relation} '{}'.",
        occurrence.exported_name, occurrence.local_name,
    );
    (description, snippet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::types::Severity;
    use std::path::PathBuf;

    #[test]
    fn occurrence_projects_to_git_diff_finding_on_the_caller_import() {
        let finding = occurrence_to_finding(&RemovedExportOccurrence {
            exporter_path: PathBuf::from("src/api.ts"),
            importer_path: PathBuf::from("src/caller.ts"),
            exported_name: "loadUser".to_string(),
            local_name: "load".to_string(),
            symbol_kind: SymbolKind::Value,
            module_specifier: "./api.ts".to_string(),
            line_start: 3,
            line_end: 3,
            byte_start: 12,
            byte_end: 28,
        });

        assert_eq!(finding.rule_id, "behavioral.removed-export-still-imported");
        assert_eq!(finding.category, FindingCategory::CodeQuality);
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(finding.provenance.analysis_scope, AnalysisScope::GitDiff);
        assert_eq!(finding.evidence[0].path, PathBuf::from("src/caller.ts"));
        assert_eq!(finding.evidence[0].line_start, 3);
        assert!(finding.evidence[0].snippet.contains("src/api.ts"));
        assert!(finding.evidence[0].snippet.contains("loadUser as load"));
    }
}
