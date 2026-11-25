use super::*;

fn build(sources: &[(&str, &str)]) -> SymbolIndex {
    let root = tempfile::tempdir().unwrap();
    SymbolIndex::from_sources(
        root.path(),
        sources.iter().map(|(path, content)| SourceModule {
            path: Path::new(path),
            content,
        }),
    )
    .unwrap()
}

fn reference<'a>(index: &'a SymbolIndex, name: &str) -> &'a ImportReference {
    index
        .references
        .iter()
        .find(|reference| reference.local_name == name)
        .unwrap()
}

#[test]
fn named_alias_and_default_imports_link_observed_exports() {
    let index = build(&[
        (
            "src/api.ts",
            "export function run() {} \nexport default function load() {}",
        ),
        (
            "src/app.ts",
            "import load, { run as execute } from './api.ts';",
        ),
    ]);

    for (local, exported) in [("load", "default"), ("execute", "run")] {
        let reference = reference(&index, local);
        assert_eq!(reference.imported_name, exported);
        assert!(reference.byte_end > reference.byte_start);
        let Resolution::UniqueObserved { export } = &reference.resolution else {
            panic!("expected unique observed export: {reference:?}");
        };
        assert_eq!(export.module, Path::new("src/api.ts"));
        assert_eq!(export.name, exported);
        assert_eq!(export.namespace, Namespace::Value);
    }
}

#[test]
fn type_imports_do_not_turn_value_facts_into_type_proof() {
    let index = build(&[
        (
            "api.ts",
            "export interface Config {} \nexport class Client {}",
        ),
        (
            "app.ts",
            "import type { Config, Client } from './api.ts';",
        ),
    ]);
    assert!(matches!(
        reference(&index, "Config").resolution,
        Resolution::UniqueObserved { .. }
    ));
    assert_eq!(
        reference(&index, "Client").resolution,
        Resolution::Unresolved {
            reason: UnresolvedReason::NamespaceUncertain,
        }
    );
}

#[test]
fn alternative_module_candidates_are_ambiguous_not_first_match() {
    let index = build(&[
        ("api.ts", "export const value = 1;"),
        ("api.js", "export const value = 2;"),
        ("app.ts", "import { value } from './api.js';"),
    ]);
    assert_eq!(
        reference(&index, "value").resolution,
        Resolution::AmbiguousModules {
            candidates: vec![PathBuf::from("api.js"), PathBuf::from("api.ts")],
        }
    );
}

#[test]
fn forwarded_and_wildcard_exports_are_not_direct_links() {
    let index = build(&[
        ("api.ts", "export { run } from './impl.ts';"),
        ("barrel.ts", "export * from './impl.ts';"),
        ("impl.ts", "export function run() {}"),
        (
            "app.ts",
            "import { run as a } from './api.ts'; import { run as b } from './barrel.ts';",
        ),
    ]);
    for name in ["a", "b"] {
        assert_eq!(
            reference(&index, name).resolution,
            Resolution::Unresolved {
                reason: UnresolvedReason::ForwardedExport,
            }
        );
    }
}

#[test]
fn missing_inputs_and_parse_errors_are_explicit() {
    let index = build(&[
        ("bad.ts", "export const = ;"),
        ("api.ts", "export const other = 1;"),
        ("large.ts", &" ".repeat(MAX_MODULE_BYTES + 1)),
        ("notes.py", "value = 1"),
        (
            "app.ts",
            "import { a } from './bad.ts'; import { b } from './missing.ts'; \
             import { c } from './api.ts'; import { d } from 'pkg'; \
             import { e } from './api'; import { f } from './large.ts';",
        ),
    ]);
    for (name, reason) in [
        ("a", UnresolvedReason::ModuleFactsUnavailable),
        ("b", UnresolvedReason::ModuleNotObserved),
        ("c", UnresolvedReason::ExportNotObserved),
        ("d", UnresolvedReason::UnsupportedSpecifier),
        ("e", UnresolvedReason::UnsupportedSpecifier),
        ("f", UnresolvedReason::ModuleFactsUnavailable),
    ] {
        assert_eq!(
            reference(&index, name).resolution,
            Resolution::Unresolved { reason },
        );
    }
    assert_eq!(
        index.modules[Path::new("bad.ts")].coverage,
        ModuleCoverage::SyntaxErrors
    );
    assert_eq!(
        index.modules[Path::new("large.ts")].coverage,
        ModuleCoverage::SourceBudgetExceeded
    );
    assert_eq!(
        index.modules[Path::new("notes.py")].coverage,
        ModuleCoverage::UnsupportedLanguage
    );
}

#[test]
fn deterministic_across_input_order_and_serialization() {
    let sources = [
        ("src/a.ts", "export const value = 1;"),
        ("src/b.ts", "import { value } from './a.ts';"),
    ];
    let first = build(&sources);
    let second = build(&[sources[1], sources[0]]);
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap(),
    );
}

#[test]
fn duplicate_or_escaping_module_paths_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    for path in ["../api.ts", "src/../api.ts", ""] {
        assert!(matches!(
            SymbolIndex::from_sources(
                root.path(),
                [SourceModule {
                    path: Path::new(path),
                    content: "",
                }]
            ),
            Err(IndexError::InvalidPath(_))
        ));
    }
    assert!(matches!(
        SymbolIndex::from_sources(
            root.path(),
            ["export const x = 1;", "export const x = 2;"].map(|content| SourceModule {
                path: Path::new("api.ts"),
                content,
            })
        ),
        Err(IndexError::DuplicatePath(_))
    ));
}
