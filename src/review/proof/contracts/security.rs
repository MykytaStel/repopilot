use super::{ChangeProofContractDelta, ContractChangeKind, ContractConfidence, ContractFamily};
use crate::audits::context::classify::helpers::is_test_file;
use crate::review::model::ReviewReport;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn security_deltas(report: &ReviewReport) -> Vec<ChangeProofContractDelta> {
    let changed_tests = report
        .changed_files
        .iter()
        .filter(|file| is_test_file(&file.path))
        .map(|file| file.path_string())
        .collect::<Vec<_>>();

    let mut deltas = Vec::new();
    for boundary in report
        .boundary_signals
        .iter()
        .filter(|signal| signal.category.is_code_boundary())
    {
        let category = boundary.category.label();
        let impacted_entrypoints = impacted_entrypoints(report, &boundary.path);
        let impact_evidence = if impacted_entrypoints.is_empty() {
            format!("Changed {category} boundary; no bounded entrypoint consumer was established.")
        } else {
            format!(
                "Changed {category} boundary; import graph reaches {} entrypoint candidate(s).",
                impacted_entrypoints.len()
            )
        };
        deltas.push(ChangeProofContractDelta {
            family: ContractFamily::SecurityBoundary,
            change: ContractChangeKind::BoundaryChanged,
            exporter_path: boundary.path.clone(),
            consumer_path: boundary.path.clone(),
            line_start: None,
            line_end: None,
            evidence: impact_evidence,
            confidence: Some(ContractConfidence::Limited),
        });

        for entrypoint in impacted_entrypoints {
            deltas.push(ChangeProofContractDelta {
                family: ContractFamily::SecurityBoundary,
                change: ContractChangeKind::EntryPointImpacted,
                exporter_path: boundary.path.clone(),
                consumer_path: entrypoint,
                line_start: None,
                line_end: None,
                evidence: "Bounded import graph identifies an entrypoint candidate; static analysis does not prove request reachability.".to_string(),
                confidence: Some(ContractConfidence::Limited),
            });
        }

        let related_tests = changed_tests
            .iter()
            .filter(|path| test_matches_boundary(path, &boundary.path))
            .collect::<Vec<_>>();
        if related_tests.is_empty() {
            deltas.push(test_delta(
                &boundary.path,
                &boundary.path,
                ContractChangeKind::TestMissing,
                if changed_tests.is_empty() {
                    "No test file changed alongside the security boundary; static analysis cannot prove missing behavioral coverage."
                } else {
                    "Changed tests could not be linked to this boundary by stable path evidence; static naming cannot prove coverage."
                },
            ));
        } else {
            for test_path in related_tests {
                deltas.push(test_delta(
                    &boundary.path,
                    test_path,
                    ContractChangeKind::TestChanged,
                    "A changed test has a stable path relationship to this boundary; static naming still cannot prove behavioral coverage.",
                ));
            }
        }
    }
    deltas
}

fn test_delta(
    boundary_path: &str,
    test_path: &str,
    change: ContractChangeKind,
    evidence: &str,
) -> ChangeProofContractDelta {
    ChangeProofContractDelta {
        family: ContractFamily::TestCoverage,
        change,
        exporter_path: boundary_path.to_string(),
        consumer_path: test_path.to_string(),
        line_start: None,
        line_end: None,
        evidence: evidence.to_string(),
        confidence: Some(ContractConfidence::Limited),
    }
}

fn impacted_entrypoints(report: &ReviewReport, boundary_path: &str) -> Vec<String> {
    let Some(impact) = report
        .impact_paths
        .files
        .iter()
        .find(|impact| impact.path == Path::new(boundary_path))
    else {
        return Vec::new();
    };

    impact
        .direct_dependents
        .iter()
        .chain(impact.transitive_dependents.iter())
        .filter(|path| is_entrypoint_path(path))
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

fn is_entrypoint_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if is_test_file(path) {
        return false;
    }
    let component_match = lower.split(['/', '\\']).any(|component| {
        matches!(
            component,
            "api"
                | "bin"
                | "cli"
                | "commands"
                | "controllers"
                | "endpoints"
                | "handlers"
                | "routes"
        )
    });
    let stem_match = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            matches!(
                stem.to_ascii_lowercase().as_str(),
                "app"
                    | "application"
                    | "cli"
                    | "command"
                    | "handler"
                    | "index"
                    | "main"
                    | "router"
                    | "routes"
                    | "server"
            )
        })
        .unwrap_or(false);
    component_match || stem_match
}

fn test_matches_boundary(test_path: &str, boundary_path: &str) -> bool {
    let boundary_tokens = path_tokens(boundary_path);
    let test_tokens = path_tokens(test_path);
    boundary_tokens
        .iter()
        .any(|token| test_tokens.contains(token))
}

fn path_tokens(path: &str) -> BTreeSet<String> {
    path.replace('\\', "/")
        .split(['/', '.', '_', '-'])
        .map(str::to_ascii_lowercase)
        .filter(|token| {
            token.len() >= 3 && !matches!(token.as_str(), "src" | "lib" | "test" | "tests")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{ChangeStatus, ChangedFile};
    use crate::review::impact::{FileImpact, ImpactPaths};
    use crate::review::model::ReviewReport;
    use crate::review::signals::{BoundaryCategory, BoundarySignal};
    use std::path::PathBuf;

    fn changed(path: &str) -> ChangedFile {
        ChangedFile {
            path: PathBuf::from(path),
            status: ChangeStatus::Modified,
            ranges: Vec::new(),
            hunks: Vec::new(),
        }
    }

    fn report(changed_files: Vec<ChangedFile>, impact: ImpactPaths) -> ReviewReport {
        ReviewReport {
            summary: Default::default(),
            repo_root: PathBuf::from("/repo"),
            baseline_path: None,
            changed_files,
            blast_radius: Vec::new(),
            impact_paths: impact,
            ownership: Default::default(),
            ownership_diagnostics: Vec::new(),
            boundary_signals: vec![BoundarySignal {
                category: BoundaryCategory::AccessControl,
                path: "src/auth/policy.ts".to_string(),
                status: ChangeStatus::Modified,
                blast_radius: 1,
            }],
            boundary_missing_test: false,
            tiered_signals: Default::default(),
            timings: Default::default(),
            verification: Vec::new(),
            findings: Vec::new(),
        }
    }

    #[test]
    fn emits_boundary_entrypoint_and_test_contracts_with_bounded_evidence() {
        let report = report(
            vec![changed("src/auth/policy.ts"), changed("tests/auth.test.ts")],
            ImpactPaths {
                depth: 2,
                files: vec![FileImpact {
                    path: PathBuf::from("src/auth/policy.ts"),
                    direct_dependents: vec![PathBuf::from("src/routes/users.ts")],
                    transitive_dependents: vec![PathBuf::from("src/domain/user.ts")],
                }],
                ..Default::default()
            },
        );

        let deltas = security_deltas(&report);
        assert!(deltas.iter().any(|delta| {
            delta.family == ContractFamily::SecurityBoundary
                && delta.change == ContractChangeKind::BoundaryChanged
        }));
        assert!(deltas.iter().any(|delta| {
            delta.change == ContractChangeKind::EntryPointImpacted
                && delta.consumer_path == "src/routes/users.ts"
        }));
        assert!(deltas.iter().any(|delta| {
            delta.family == ContractFamily::TestCoverage
                && delta.change == ContractChangeKind::TestChanged
                && delta.consumer_path == "tests/auth.test.ts"
        }));
        assert!(
            deltas
                .iter()
                .all(|delta| { delta.confidence == Some(ContractConfidence::Limited) })
        );
    }

    #[test]
    fn missing_test_contract_is_explicit_and_never_broken() {
        let deltas = security_deltas(&report(
            vec![changed("src/auth/policy.ts")],
            ImpactPaths::default(),
        ));

        let missing = deltas
            .iter()
            .find(|delta| delta.change == ContractChangeKind::TestMissing)
            .expect("missing test contract");
        assert_eq!(missing.family, ContractFamily::TestCoverage);
        assert!(!missing.is_broken());
        assert!(missing.evidence.contains("cannot prove"));
    }

    #[test]
    fn similar_but_unrelated_test_name_is_not_claimed_as_coverage() {
        assert!(!test_matches_boundary(
            "tests/author.test.ts",
            "src/auth/policy.ts"
        ));
    }
}
