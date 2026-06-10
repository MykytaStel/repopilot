use std::path::PathBuf;

use crate::analysis::{ArchitectureContext, FileRole, LanguageFamily, ModuleKind};
use crate::scan::config::{LayerSpec, ScanConfig};

use super::layers::LayerIndex;
use super::packages::PackageIndex;
use super::{NodeInfo, dead_module_finding, test_leak_finding};

fn node(relative: &str, role: FileRole, is_entrypoint: bool, is_public_api: bool) -> NodeInfo {
    NodeInfo {
        relative: PathBuf::from(relative),
        context: ArchitectureContext {
            file_role: role,
            module_kind: ModuleKind::Unknown,
            language_family: LanguageFamily::CurlyBrace,
            is_entrypoint,
            is_public_api,
        },
    }
}

fn prod(relative: &str) -> NodeInfo {
    node(relative, FileRole::Production, false, false)
}

#[test]
fn dead_module_flags_unreferenced_production_file() {
    let info = prod("src/orphan.ts");
    assert!(dead_module_finding(&info, Some(0)).is_some());
}

#[test]
fn dead_module_exempts_entrypoints_public_api_and_imported_files() {
    assert!(
        dead_module_finding(
            &node("src/main.rs", FileRole::Production, true, false),
            Some(0)
        )
        .is_none()
    );
    assert!(
        dead_module_finding(
            &node("src/lib.rs", FileRole::Production, false, true),
            Some(0)
        )
        .is_none()
    );
    assert!(dead_module_finding(&prod("src/used.ts"), Some(2)).is_none());
    assert!(
        dead_module_finding(
            &node("src/foo.test.ts", FileRole::Test, false, false),
            Some(0)
        )
        .is_none()
    );
}

#[test]
fn test_leak_flags_production_importing_test_or_fixture() {
    let source = prod("src/app.ts");
    assert!(
        test_leak_finding(
            &source,
            &node("src/app.test.ts", FileRole::Test, false, false)
        )
        .is_some()
    );
    assert!(
        test_leak_finding(
            &source,
            &node("fixtures/data.ts", FileRole::Fixture, false, false)
        )
        .is_some()
    );
    // Production importing production, and tests importing tests, are fine.
    assert!(test_leak_finding(&source, &prod("src/util.ts")).is_none());
    assert!(
        test_leak_finding(
            &node("src/app.test.ts", FileRole::Test, false, false),
            &node("src/helper.test.ts", FileRole::Test, false, false),
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
    // core (index 1) importing ui (index 0) reverses the declared order.
    let finding = index.violation_finding(&prod("src/core/service.ts"), &prod("src/ui/widget.ts"));
    assert!(finding.is_some());
}

#[test]
fn layer_violation_allows_declared_direction_and_unlayered_files() {
    let index = LayerIndex::from_config(&layered_config());
    // ui importing core follows the declared order.
    assert!(
        index
            .violation_finding(&prod("src/ui/page.ts"), &prod("src/core/service.ts"))
            .is_none()
    );
    // A file outside every layer is ignored.
    assert!(
        index
            .violation_finding(&prod("src/util/log.ts"), &prod("src/ui/widget.ts"))
            .is_none()
    );
}

#[test]
fn layer_index_is_empty_without_config() {
    let index = LayerIndex::from_config(&ScanConfig::default());
    assert!(
        index
            .violation_finding(&prod("src/core/a.ts"), &prod("src/ui/b.ts"))
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
    let finding = index.violation_finding(
        &prod("packages/web/src/use.ts"),
        &prod("packages/auth/src/internal.ts"),
    );
    assert!(finding.is_some());
}

#[test]
fn package_boundary_allows_public_api_same_package_and_no_config() {
    let configured = PackageIndex::from_config(&packaged_config());
    // Importing another package's public API is allowed.
    assert!(
        configured
            .violation_finding(
                &prod("packages/web/src/use.ts"),
                &node("packages/auth/index.ts", FileRole::Production, false, true),
            )
            .is_none()
    );
    // Same package is allowed.
    assert!(
        configured
            .violation_finding(
                &prod("packages/auth/src/a.ts"),
                &prod("packages/auth/src/b.ts"),
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
            )
            .is_none()
    );
}
