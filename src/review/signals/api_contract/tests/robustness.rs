use super::{changed, coupling_graph, init_repo, source, write};
use crate::review::diff::{ChangeStatus, DiffTarget};
use crate::review::signals::api_contract::{
    ChangedReviewSources, detect_removed_export_imports, extract_javascript_symbol_facts,
};
use tempfile::TempDir;

#[test]
fn malformed_pre_exporter_cannot_produce_a_removed_export_signal() {
    assert_no_signal(
        "export function loadUser() {}\nconst broken = ;\n",
        "export function saveUser() {}\n",
        "import { loadUser } from \"./api.ts\";\n",
    );
}

#[test]
fn malformed_post_exporter_cannot_produce_a_removed_export_signal() {
    assert_no_signal(
        "export function loadUser() {}\n",
        "export function saveUser() {}\nconst broken = ;\n",
        "import { loadUser } from \"./api.ts\";\n",
    );
}

#[test]
fn malformed_caller_cannot_produce_a_removed_export_signal() {
    assert_no_signal(
        "export function loadUser() {}\n",
        "export function saveUser() {}\n",
        "import { loadUser } from \"./api.ts\";\nconst broken = ;\n",
    );
}

#[test]
fn namespace_exports_are_not_direct_module_exports() {
    let facts = extract_javascript_symbol_facts(&source(concat!(
        "namespace Internal { export const internalOnly = 1; }\n",
        "export namespace Public { export const publicMemberOnly = 1; }\n",
        "declare namespace Ambient { export const ambientOnly: number; }\n",
    )))
    .expect("supported TypeScript source");

    assert!(facts.exports.is_empty(), "{:#?}", facts.exports);
}

#[test]
fn default_semantic_export_and_import_specifiers_are_excluded() {
    for source_text in [
        concat!(
            "const value = 1;\n",
            "export { value as default };\n",
            "import { default as local } from \"./api.ts\";\n",
        ),
        concat!(
            "const value = 1;\n",
            "export { value as \"default\" };\n",
            "import { \"default\" as local } from \"./api.ts\";\n",
        ),
    ] {
        let facts = extract_javascript_symbol_facts(&source(source_text))
            .expect("supported TypeScript source");
        assert!(facts.exports.is_empty(), "{:#?}", facts.exports);
        assert!(facts.imports.is_empty(), "{:#?}", facts.imports);
    }
}

#[test]
fn default_semantic_specifiers_cannot_produce_a_removed_export_signal() {
    for (pre, caller) in [
        (
            "const value = 1;\nexport { value as default };\n",
            "import { default as local } from \"./api.ts\";\n",
        ),
        (
            "const value = 1;\nexport { value as \"default\" };\n",
            "import { \"default\" as local } from \"./api.ts\";\n",
        ),
    ] {
        assert_no_signal(pre, "const value = 1;\n", caller);
    }
}

#[test]
fn same_line_aliases_retain_distinct_exact_ast_spans() {
    let caller = "import { loadUser as first, loadUser as second } from \"./api.ts\";\n";
    let facts =
        extract_javascript_symbol_facts(&source(caller)).expect("supported TypeScript source");
    assert_eq!(facts.imports.len(), 2, "{:#?}", facts.imports);
    assert_eq!(facts.imports[0].line_start, facts.imports[1].line_start);
    assert_ne!(
        (facts.imports[0].byte_start, facts.imports[0].byte_end),
        (facts.imports[1].byte_start, facts.imports[1].byte_end),
    );

    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", "export function saveUser() {}\n");
    write(root, "src/caller.ts", caller);
    let api = changed("src/api.ts", ChangeStatus::Modified);
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
        Some(&coupling_graph(&[("src/caller.ts", "src/api.ts")])),
    );
    assert_eq!(signals.len(), 2, "{signals:#?}");
    assert_ne!(
        (signals[0].byte_start, signals[0].byte_end),
        (signals[1].byte_start, signals[1].byte_end),
    );
}

fn assert_no_signal(pre_source: &str, post_source: &str, caller_source: &str) {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", post_source);
    write(root, "src/caller.ts", caller_source);
    let api = changed("src/api.ts", ChangeStatus::Modified);
    let pre = source(pre_source);
    let post = source(post_source);
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

    assert!(signals.is_empty(), "{signals:#?}");
}
