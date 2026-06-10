//! Opt-in architecture rules (`architecture.layer-violation`,
//! `architecture.package-boundary-violation`) are driven entirely by config and
//! must stay silent until the user declares layers or package roots. The rule
//! fixture harness scans with the default config, so these config-dependent
//! rules are exercised here instead.

use repopilot::scan::config::{LayerSpec, ScanConfig};
use repopilot::scan::scanner::scan_path_with_config;
use std::fs;
use tempfile::TempDir;

fn write_file(temp: &TempDir, relative: &str, contents: &str) {
    let path = temp.path().join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create fixture dirs");
    }
    fs::write(path, contents).expect("failed to write fixture file");
}

fn emits(summary: &repopilot::scan::types::ScanSummary, rule_id: &str) -> bool {
    summary
        .artifacts
        .findings
        .iter()
        .any(|finding| finding.rule_id == rule_id)
}

fn base_config() -> ScanConfig {
    ScanConfig {
        include_low_signal: true,
        detect_missing_tests: false,
        ..ScanConfig::default()
    }
}

fn layers() -> Vec<LayerSpec> {
    vec![
        LayerSpec {
            name: "ui".into(),
            paths: vec!["src/ui/**".into()],
        },
        LayerSpec {
            name: "core".into(),
            paths: vec!["src/core/**".into()],
        },
    ]
}

#[test]
fn layer_violation_fires_only_against_declared_direction() {
    let temp = TempDir::new().expect("temp dir");
    // core (lower layer) importing ui (higher layer) reverses the declared order.
    write_file(
        &temp,
        "src/core/service.ts",
        "import { widget } from '../ui/widget';\nexport const s = widget;\n",
    );
    write_file(&temp, "src/ui/widget.ts", "export const widget = 1;\n");

    let with_layers = ScanConfig {
        architecture_layers: layers(),
        ..base_config()
    };
    let summary = scan_path_with_config(temp.path(), &with_layers).expect("scan");
    assert!(
        emits(&summary, "architecture.layer-violation"),
        "expected a layer violation for core -> ui"
    );

    // Without any declared layers the rule must be silent on the same tree.
    let summary = scan_path_with_config(temp.path(), &base_config()).expect("scan");
    assert!(
        !emits(&summary, "architecture.layer-violation"),
        "layer rule must be opt-in (no findings without config)"
    );
}

#[test]
fn layer_violation_allows_the_declared_direction() {
    let temp = TempDir::new().expect("temp dir");
    // ui (higher layer) importing core (lower layer) follows the declared order.
    write_file(
        &temp,
        "src/ui/page.ts",
        "import { service } from '../core/service';\nexport const p = service;\n",
    );
    write_file(&temp, "src/core/service.ts", "export const service = 1;\n");

    let with_layers = ScanConfig {
        architecture_layers: layers(),
        ..base_config()
    };
    let summary = scan_path_with_config(temp.path(), &with_layers).expect("scan");
    assert!(
        !emits(&summary, "architecture.layer-violation"),
        "ui -> core follows the declared layer order and must not be flagged"
    );
}

#[test]
fn package_boundary_fires_on_cross_package_internal_imports() {
    let temp = TempDir::new().expect("temp dir");
    write_file(
        &temp,
        "packages/web/src/use.ts",
        "import { secret } from '../../auth/src/internal';\nexport const u = secret;\n",
    );
    write_file(
        &temp,
        "packages/auth/src/internal.ts",
        "export const secret = 1;\n",
    );

    let with_roots = ScanConfig {
        package_roots: vec!["packages/*".into()],
        ..base_config()
    };
    let summary = scan_path_with_config(temp.path(), &with_roots).expect("scan");
    assert!(
        emits(&summary, "architecture.package-boundary-violation"),
        "expected a boundary violation reaching into another package's internals"
    );

    // Opt-in: no package roots -> no findings.
    let summary = scan_path_with_config(temp.path(), &base_config()).expect("scan");
    assert!(
        !emits(&summary, "architecture.package-boundary-violation"),
        "package boundary rule must be opt-in (no findings without config)"
    );
}

#[test]
fn package_boundary_allows_public_api_imports() {
    let temp = TempDir::new().expect("temp dir");
    write_file(
        &temp,
        "packages/web/src/use.ts",
        "import { token } from '../../auth/index';\nexport const u = token;\n",
    );
    // index.ts is the package's public API surface.
    write_file(&temp, "packages/auth/index.ts", "export const token = 1;\n");

    let with_roots = ScanConfig {
        package_roots: vec!["packages/*".into()],
        ..base_config()
    };
    let summary = scan_path_with_config(temp.path(), &with_roots).expect("scan");
    assert!(
        !emits(&summary, "architecture.package-boundary-violation"),
        "importing another package's public API must not be flagged"
    );
}
