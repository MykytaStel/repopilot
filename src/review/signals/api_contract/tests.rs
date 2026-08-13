use super::{
    ChangedReviewSources, SymbolKind, detect_removed_export_imports,
    extract_javascript_symbol_facts,
};
use crate::review::diff::{ChangeStatus, ChangedFile, DiffTarget};
use crate::review::signals::content::ReviewSource;
use crate::scan::types::CouplingGraph;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

mod robustness;

#[test]
fn extracts_named_value_type_and_binding_alias_facts() {
    // Catches an extractor that misses direct named declarations, type facts,
    // import aliases, or the imported statement's source span.
    let source = ReviewSource::new(
        concat!(
            "export function loadUser() {}\n",
            "export type UserId = string;\n",
            "import { loadUser as load, type UserId } from \"./api.ts\";\n",
        )
        .to_string(),
        Some("TypeScript".to_string()),
    );
    let facts = extract_javascript_symbol_facts(&source).expect("supported source");
    assert!(
        facts
            .exports
            .iter()
            .any(|fact| fact.name == "loadUser" && fact.kind == SymbolKind::Value)
    );
    assert!(
        facts
            .exports
            .iter()
            .any(|fact| fact.name == "UserId" && fact.kind == SymbolKind::Type)
    );
    assert!(facts.imports.iter().any(|fact| {
        fact.imported_name == "loadUser"
            && fact.local_name == "load"
            && fact.module_specifier == "./api.ts"
            && fact.line_start == 3
    }));
}

#[test]
fn extracts_local_export_clauses_and_type_only_modifiers() {
    // Catches an extractor that wrongly requires `from` for local exports or
    // loses statement/specifier-level TypeScript type modifiers.
    let source = ReviewSource::new(
        concat!(
            "export { saveUser as save, type UserId };\n",
            "export type { Account as AccountType };\n",
            "import type { Account as LocalAccount } from \"./models.ts\";\n",
        )
        .to_string(),
        Some("TypeScript".to_string()),
    );
    let facts = extract_javascript_symbol_facts(&source).expect("supported source");

    assert!(facts.exports.iter().any(|fact| {
        fact.name == "save" && fact.kind == SymbolKind::Value && fact.line_start == 1
    }));
    assert!(facts.exports.iter().any(|fact| {
        fact.name == "UserId" && fact.kind == SymbolKind::Type && fact.line_start == 1
    }));
    assert!(facts.exports.iter().any(|fact| {
        fact.name == "AccountType" && fact.kind == SymbolKind::Type && fact.line_start == 2
    }));
    assert!(facts.imports.iter().any(|fact| {
        fact.imported_name == "Account"
            && fact.local_name == "LocalAccount"
            && fact.kind == SymbolKind::Type
            && fact.module_specifier == "./models.ts"
    }));
}

#[test]
fn ignores_non_direct_module_forms_and_text_lookalikes() {
    // Catches a text/regex fallback or a traversal that treats excluded module
    // forms as direct named facts.
    let source = ReviewSource::new(
        concat!(
            "// export const Commented = 1; import { Commented } from \"./comment.ts\";\n",
            "const text = 'export { StringOnly }; import { StringOnly } from \\\"./string.ts\\\"';\n",
            "export default function defaultOnly() {}\n",
            "export * from \"./barrel.ts\";\n",
            "export { reExported } from \"./remote.ts\";\n",
            "import defaultOnly from \"./default.ts\";\n",
            "import * as namespaceOnly from \"./namespace.ts\";\n",
            "import(\"./dynamic.ts\");\n",
            "const commonJs = require(\"./commonjs.ts\");\n",
        )
        .to_string(),
        Some("TypeScript".to_string()),
    );
    let facts = extract_javascript_symbol_facts(&source).expect("supported source");

    assert!(facts.exports.is_empty());
    assert!(facts.imports.is_empty());
}
#[test]
fn removed_export_with_surviving_resolved_caller_is_reported() {
    // Catches a missed surviving direct import of a removed value export.
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(
        root,
        "src/caller.ts",
        "import { loadUser } from \"./api.ts\";\n",
    );

    let api = changed("src/api.ts", ChangeStatus::Modified);
    let pre = ReviewSource::new(
        "export function loadUser() {}\n".to_string(),
        Some("TypeScript".to_string()),
    );
    let post = ReviewSource::new(
        "export function saveUser() {}\n".to_string(),
        Some("TypeScript".to_string()),
    );
    let sources = [ChangedReviewSources {
        file: &api,
        pre: Some(&pre),
        post: Some(&post),
    }];

    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &sources,
        Some(&coupling_graph(&[("src/caller.ts", "src/api.ts")])),
    );

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].exporter_path, PathBuf::from("src/api.ts"));
    assert_eq!(signals[0].importer_path, PathBuf::from("src/caller.ts"));
    assert_eq!(signals[0].exported_name, "loadUser");
    assert_eq!(signals[0].local_name, "loadUser");
    assert_eq!(signals[0].symbol_kind, SymbolKind::Value);
    assert_eq!(signals[0].module_specifier, "./api.ts");
    assert_eq!((signals[0].line_start, signals[0].line_end), (1, 1));
}

#[test]
fn removed_type_export_with_surviving_type_import_is_reported() {
    // Catches a detector that treats erased TypeScript imports as value imports.
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export type User = string;\n");
    write(
        root,
        "src/caller.ts",
        "import type { User } from \"./api.ts\";\n",
    );
    let api = changed("src/api.ts", ChangeStatus::Modified);
    let pre = source("export type User = string;\n");
    let post = source("export type Account = string;\n");

    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &api,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&coupling_graph(&[("src/caller.ts", "src/api.ts")])),
    );

    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].exported_name, "User");
    assert_eq!(signals[0].symbol_kind, SymbolKind::Type);
}

#[test]
fn unsupported_mjs_or_cjs_exporter_or_importer_is_not_reported() {
    // Catches treating every AST-supported JavaScript extension as in scope.
    for (exporter, importer, import_source) in [
        (
            "src/api.mjs",
            "src/caller.js",
            "import { loadUser } from \"./api.mjs\";\n",
        ),
        (
            "src/api.js",
            "src/caller.cjs",
            "import { loadUser } from \"./api.js\";\n",
        ),
    ] {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        init_repo(root);
        write(root, exporter, "export function loadUser() {}\n");
        write(root, importer, import_source);
        let api = changed(exporter, ChangeStatus::Modified);
        let pre = source("export function loadUser() {}\n");
        let post = source("export function saveUser() {}\n");

        let signals = detect_removed_export_imports(
            root,
            DiffTarget::WorkingTree,
            &[ChangedReviewSources {
                file: &api,
                pre: Some(&pre),
                post: Some(&post),
            }],
            Some(&coupling_graph(&[(importer, exporter)])),
        );

        assert!(signals.is_empty(), "{exporter} -> {importer}");
    }
}
#[test]
fn removed_exports_require_a_surviving_direct_resolved_value_import() {
    // Catches graph-only, alias, name-only, or type/value mismatches.
    assert_no_removed_export(
        "import { saveUser } from \"./api.ts\";\n",
        &[("src/caller.ts", "src/api.ts")],
        ChangeStatus::Modified,
        Some("export function saveUser() {}\n"),
        true,
    );
    assert_no_removed_export(
        "import { loadUser } from \"./api.ts\";\n",
        &[],
        ChangeStatus::Modified,
        None,
        true,
    );
    assert_no_removed_export(
        "import { loadUser } from \"@api\";\n",
        &[("src/caller.ts", "src/api.ts")],
        ChangeStatus::Modified,
        None,
        true,
    );
    assert_no_removed_export(
        "import { loadUser } from \"./other.ts\";\n",
        &[
            ("src/caller.ts", "src/api.ts"),
            ("src/caller.ts", "src/other.ts"),
        ],
        ChangeStatus::Modified,
        None,
        true,
    );
    assert_no_removed_export(
        "import { loadUser } from \"./api.ts\";\n",
        &[("src/caller.ts", "src/api.ts")],
        ChangeStatus::Modified,
        None,
        false,
    );

    for status in [ChangeStatus::Deleted, ChangeStatus::Renamed] {
        assert_no_removed_export(
            "import { loadUser } from \"./api.ts\";\n",
            &[("src/caller.ts", "src/api.ts")],
            status,
            None,
            true,
        );
    }

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export type User = string;\n");
    write(
        root,
        "src/caller.ts",
        "import { User } from \"./api.ts\";\n",
    );
    let api = changed("src/api.ts", ChangeStatus::Modified);
    let pre = source("export type User = string;\n");
    let post = source("export type Other = string;\n");
    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &api,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&coupling_graph(&[("src/caller.ts", "src/api.ts")])),
    );
    assert!(signals.is_empty());
}
fn assert_no_removed_export(
    caller_source: &str,
    edges: &[(&str, &str)],
    status: ChangeStatus,
    changed_caller_post: Option<&str>,
    with_graph: bool,
) {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function loadUser() {}\n");
    write(root, "src/caller.ts", caller_source);
    write(root, "src/other.ts", "export function loadUser() {}\n");
    let api = changed("src/api.ts", status);
    let pre = source("export function loadUser() {}\n");
    let post = source("export function saveUser() {}\n");
    let caller = changed("src/caller.ts", ChangeStatus::Modified);
    let caller_post = changed_caller_post.map(source);
    let mut sources = vec![ChangedReviewSources {
        file: &api,
        pre: Some(&pre),
        post: Some(&post),
    }];
    if let Some(caller_post) = caller_post.as_ref() {
        sources.push(ChangedReviewSources {
            file: &caller,
            pre: None,
            post: Some(caller_post),
        });
    }
    let graph = coupling_graph(edges);
    let signals = detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &sources,
        with_graph.then_some(&graph),
    );
    assert!(signals.is_empty());
}
fn source(content: &str) -> ReviewSource {
    ReviewSource::new(content.to_string(), Some("TypeScript".to_string()))
}

fn coupling_graph(edges: &[(&str, &str)]) -> CouplingGraph {
    let mut graph = CouplingGraph::default();
    for (importer, exporter) in edges {
        let importer = PathBuf::from(importer);
        let exporter = PathBuf::from(exporter);
        graph.nodes.insert(importer.clone());
        graph.nodes.insert(exporter.clone());
        graph.edges.entry(importer).or_default().insert(exporter);
    }
    graph
}

fn changed(path: &str, status: ChangeStatus) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        status,
        ranges: Vec::new(),
        hunks: Vec::new(),
    }
}

fn init_repo(root: &Path) {
    let output = Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output()
        .expect("run git init");
    assert!(output.status.success());
}

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(path, content).expect("write source");
}
