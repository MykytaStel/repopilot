use super::{changed, coupling_graph, init_repo, source, write};
use crate::review::diff::{ChangeStatus, DiffTarget};
use crate::review::signals::api_contract::{
    ChangedReviewSources, RemovedExportSignal, SymbolKind, detect_removed_export_imports,
    extract_javascript_symbol_facts,
};
use tempfile::TempDir;

#[test]
fn sourced_re_export_of_the_same_name_is_not_a_removed_export() {
    // Catches claiming a break when the change only moved the implementation
    // and kept the name reachable through `export { name } from "..."`.
    let signals = detect(
        "export function loadUser() {}\n",
        "export { loadUser } from \"./users.ts\";\n",
        "import { loadUser } from \"./api.ts\";\n",
    );

    assert!(signals.is_empty(), "{signals:#?}");
}

#[test]
fn forwarded_default_alias_supplies_the_named_export() {
    // Catches dropping `default as name` forwards, whose exported side is the
    // alias and not the excluded `default` name.
    let signals = detect(
        "export function loadUser() {}\n",
        "export { default as loadUser } from \"./users.ts\";\n",
        "import { loadUser } from \"./api.ts\";\n",
    );

    assert!(signals.is_empty(), "{signals:#?}");
}

#[test]
fn star_re_export_suppresses_every_removal_claim() {
    // Catches proving a removal against a module that can still forward any
    // name from an unanalyzed source.
    let signals = detect(
        "export function loadUser() {}\n",
        "export * from \"./users.ts\";\n",
        "import { loadUser } from \"./api.ts\";\n",
    );

    assert!(signals.is_empty(), "{signals:#?}");
}

#[test]
fn namespace_re_export_supplies_only_its_own_alias() {
    // Catches widening `export * as alias from "..."` into an unbounded
    // forward, which would silence every real removal in the module.
    let facts = extract_javascript_symbol_facts(&source(
        "export * as api from \"./users.ts\";\nexport { helper as tool } from \"./util.ts\";\n",
    ))
    .expect("supported TypeScript source");
    assert!(!facts.wildcard_re_export, "{facts:#?}");
    assert_eq!(
        facts.re_exports,
        vec!["api".to_string(), "tool".to_string()]
    );

    let signals = detect(
        "export function loadUser() {}\n",
        "export * as api from \"./users.ts\";\n",
        "import { loadUser } from \"./api.ts\";\n",
    );
    assert_eq!(signals.len(), 1, "{signals:#?}");
}

#[test]
fn a_name_removed_under_every_kind_matches_any_caller_import_form() {
    // Catches matching the caller's import kind against the pre-change export
    // kind, which misses `import { Type }` and `import type { value }`.
    let type_export = detect(
        "export type UserRecord = { id: string };\n",
        "export type AccountRecord = { id: string };\n",
        "import { UserRecord } from \"./api.ts\";\n",
    );
    assert_eq!(type_export.len(), 1, "{type_export:#?}");
    assert_eq!(type_export[0].exported_name, "UserRecord");
    assert_eq!(type_export[0].symbol_kind, SymbolKind::Value);

    let value_export = detect(
        "export function loadUser() {}\n",
        "export function saveUser() {}\n",
        "import type { loadUser } from \"./api.ts\";\n",
    );
    assert_eq!(value_export.len(), 1, "{value_export:#?}");
    assert_eq!(value_export[0].symbol_kind, SymbolKind::Type);
}

#[test]
fn a_surviving_name_is_only_claimed_for_the_kind_that_disappeared() {
    // Catches turning the kind-agnostic match into a blanket one: a name that
    // still exists as a type satisfies a type-only caller.
    let value_caller = detect(
        "export const Money = 1;\n",
        "export type Money = number;\n",
        "import { Money } from \"./api.ts\";\n",
    );
    assert_eq!(value_caller.len(), 1, "{value_caller:#?}");

    let type_caller = detect(
        "export const Money = 1;\n",
        "export type Money = number;\n",
        "import type { Money } from \"./api.ts\";\n",
    );
    assert!(type_caller.is_empty(), "{type_caller:#?}");
}

fn detect(pre_source: &str, post_source: &str, caller_source: &str) -> Vec<RemovedExportSignal> {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    init_repo(root);
    write(root, "src/api.ts", post_source);
    write(root, "src/caller.ts", caller_source);
    let api = changed("src/api.ts", ChangeStatus::Modified);
    let pre = source(pre_source);
    let post = source(post_source);

    detect_removed_export_imports(
        root,
        DiffTarget::WorkingTree,
        &[ChangedReviewSources {
            file: &api,
            pre: Some(&pre),
            post: Some(&post),
        }],
        Some(&coupling_graph(&[("src/caller.ts", "src/api.ts")])),
    )
}
