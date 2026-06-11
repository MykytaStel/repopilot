use std::path::{Path, PathBuf};

use crate::analysis::{ArchitectureContext, FileRole, LanguageFamily, ModuleKind};
use crate::findings::types::Confidence;
use crate::graph::ImportResolutionStats;
use crate::scan::config::{LayerSpec, ScanConfig};

use super::layers::LayerIndex;
use super::packages::PackageIndex;
use super::{NodeInfo, dead_module_finding, test_leak_finding};

fn complete_graph() -> ImportResolutionStats {
    ImportResolutionStats::default()
}

fn node(
    relative: &str,
    role: FileRole,
    is_entrypoint: bool,
    is_public_api: bool,
) -> NodeInfo<'static> {
    NodeInfo {
        relative: PathBuf::from(relative),
        context: ArchitectureContext {
            file_role: role,
            module_kind: ModuleKind::Unknown,
            language_family: LanguageFamily::CurlyBrace,
            is_entrypoint,
            is_public_api,
        },
        facts: None,
    }
}

fn prod(relative: &str) -> NodeInfo<'static> {
    node(relative, FileRole::Production, false, false)
}

#[test]
fn dead_module_flags_unreferenced_production_file() {
    let info = prod("src/orphan.ts");
    let finding = dead_module_finding(&info, Some(0), &complete_graph());
    let finding = finding.expect("unreferenced production file should be flagged");
    assert_eq!(finding.confidence, Confidence::High);
}

#[test]
fn dead_module_exempts_entrypoints_public_api_and_imported_files() {
    let resolution = complete_graph();
    assert!(
        dead_module_finding(
            &node("src/main.rs", FileRole::Production, true, false),
            Some(0),
            &resolution
        )
        .is_none()
    );
    assert!(
        dead_module_finding(
            &node("src/lib.rs", FileRole::Production, false, true),
            Some(0),
            &resolution
        )
        .is_none()
    );
    assert!(dead_module_finding(&prod("src/used.ts"), Some(2), &resolution).is_none());
    assert!(
        dead_module_finding(
            &node("src/foo.test.ts", FileRole::Test, false, false),
            Some(0),
            &resolution
        )
        .is_none()
    );
}

#[test]
fn dead_module_is_demoted_to_medium_when_graph_has_unresolved_imports() {
    let mut resolution = ImportResolutionStats::default();
    resolution.record(Path::new("src/other.ts"), "./missing-helper");

    let finding = dead_module_finding(&prod("src/orphan.ts"), Some(0), &resolution)
        .expect("dead module should still be reported when the graph is merely incomplete");

    assert_eq!(finding.confidence, Confidence::Medium);
    assert!(
        finding.evidence[0]
            .snippet
            .contains("unresolved relative import"),
        "snippet should explain the demotion: {}",
        finding.evidence[0].snippet
    );
}

#[test]
fn dead_module_is_suppressed_when_unresolved_import_could_target_it() {
    let mut resolution = ImportResolutionStats::default();
    resolution.record(Path::new("src/other.ts"), "../legacy/orphan");

    assert!(
        dead_module_finding(&prod("src/orphan.ts"), Some(0), &resolution).is_none(),
        "an unresolved import matching the candidate's name is a plausible importer"
    );
}

#[test]
fn test_leak_flags_production_importing_test_or_fixture() {
    let source = prod("src/app.ts");
    let root = Path::new("");
    let known = std::collections::HashSet::new();
    assert!(
        test_leak_finding(
            &source,
            &node("src/app.test.ts", FileRole::Test, false, false),
            root,
            &known
        )
        .is_some()
    );
    assert!(
        test_leak_finding(
            &source,
            &node("fixtures/data.ts", FileRole::Fixture, false, false),
            root,
            &known
        )
        .is_some()
    );
    // Production importing production, and tests importing tests, are fine.
    assert!(test_leak_finding(&source, &prod("src/util.ts"), root, &known).is_none());
    assert!(
        test_leak_finding(
            &node("src/app.test.ts", FileRole::Test, false, false),
            &node("src/helper.test.ts", FileRole::Test, false, false),
            root,
            &known
        )
        .is_none()
    );
}

fn layered_config() -> ScanConfig {
    ScanConfig {
        architecture_layers: vec![
            LayerSpec {
                name: "ui".into(),
                paths: vec!["src/ui/**".into()],
            },
            LayerSpec {
                name: "core".into(),
                paths: vec!["src/core/**".into()],
            },
        ],
        ..ScanConfig::default()
    }
}

#[test]
fn layer_violation_flags_lower_layer_importing_higher_layer() {
    let index = LayerIndex::from_config(&layered_config());
    let root = Path::new("");
    let known = std::collections::HashSet::new();
    // core (index 1) importing ui (index 0) reverses the declared order.
    let finding = index.violation_finding(
        &prod("src/core/service.ts"),
        &prod("src/ui/widget.ts"),
        root,
        &known,
    );
    assert!(finding.is_some());
}

#[test]
fn layer_violation_allows_declared_direction_and_unlayered_files() {
    let index = LayerIndex::from_config(&layered_config());
    let root = Path::new("");
    let known = std::collections::HashSet::new();
    // ui importing core follows the declared order.
    assert!(
        index
            .violation_finding(
                &prod("src/ui/page.ts"),
                &prod("src/core/service.ts"),
                root,
                &known
            )
            .is_none()
    );
    // A file outside every layer is ignored.
    assert!(
        index
            .violation_finding(
                &prod("src/util/log.ts"),
                &prod("src/ui/widget.ts"),
                root,
                &known
            )
            .is_none()
    );
}

#[test]
fn layer_index_is_empty_without_config() {
    let index = LayerIndex::from_config(&ScanConfig::default());
    let root = Path::new("");
    let known = std::collections::HashSet::new();
    assert!(
        index
            .violation_finding(&prod("src/core/a.ts"), &prod("src/ui/b.ts"), root, &known)
            .is_none()
    );
}

fn packaged_config() -> ScanConfig {
    ScanConfig {
        package_roots: vec!["packages/*".into()],
        ..ScanConfig::default()
    }
}

#[test]
fn package_boundary_flags_cross_package_internal_import() {
    let index = PackageIndex::from_config(&packaged_config());
    let root = Path::new("");
    let known = std::collections::HashSet::new();
    let finding = index.violation_finding(
        &prod("packages/web/src/use.ts"),
        &prod("packages/auth/src/internal.ts"),
        root,
        &known,
    );
    assert!(finding.is_some());
}

#[test]
fn architecture_findings_use_import_line_as_evidence() {
    let mut known = std::collections::HashSet::new();
    known.insert(PathBuf::from("packages/auth/src/internal.ts"));

    let content = "\
const x = 1;
import { something } from \"../../auth/src/internal\";
export {};
";
    let facts = crate::scan::facts::FileFacts {
        path: PathBuf::from("packages/web/src/use.ts"),
        language: Some("TypeScript".to_string()),
        non_empty_lines: 3,
        branch_count: 0,
        imports: vec!["../../auth/src/internal".to_string()],
        content: Some(content.to_string()),
        has_inline_tests: false,
    };

    let source = NodeInfo {
        relative: PathBuf::from("packages/web/src/use.ts"),
        context: ArchitectureContext {
            file_role: FileRole::Production,
            module_kind: ModuleKind::Unknown,
            language_family: LanguageFamily::CurlyBrace,
            is_entrypoint: false,
            is_public_api: false,
        },
        facts: Some(&facts),
    };

    let target = prod("packages/auth/src/internal.ts");

    let index = PackageIndex::from_config(&packaged_config());
    // Use an absolute-ish root just to prove resolve_import handles the relative setup properly
    let root = Path::new("/var/repo");
    let finding = index.violation_finding(&source, &target, root, &known).expect("should find violation");

    assert_eq!(finding.evidence[0].line_start, 2, "evidence should point to line 2");
    assert_eq!(finding.evidence[0].line_end, None, "single line import has no line_end");
}

#[test]
fn package_boundary_allows_public_api_same_package_and_no_config() {
    let configured = PackageIndex::from_config(&packaged_config());
    let root = Path::new("");
    let known = std::collections::HashSet::new();
    // Importing another package's public API is allowed.
    assert!(
        configured
            .violation_finding(
                &prod("packages/web/src/use.ts"),
                &node("packages/auth/index.ts", FileRole::Production, false, true),
                root,
                &known
            )
            .is_none()
    );
    // Same package is allowed.
    assert!(
        configured
            .violation_finding(
                &prod("packages/auth/src/a.ts"),
                &prod("packages/auth/src/b.ts"),
                root,
                &known
            )
            .is_none()
    );
    // Without config the rule is silent.
    let unconfigured = PackageIndex::from_config(&ScanConfig::default());
    assert!(
        unconfigured
            .violation_finding(
                &prod("packages/web/src/use.ts"),
                &prod("packages/auth/src/internal.ts"),
                root,
                &known
            )
            .is_none()
    );
}
