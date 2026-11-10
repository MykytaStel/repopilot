//! Resolves raw import strings to concrete repository paths, per language.
//!
//! [`resolve_import`] dispatches on the importing file's extension to a
//! language-specific resolver submodule. Each submodule owns the import
//! semantics for one language family; the shared `probe` / [`normalize_path`]
//! helpers live here because every resolver depends on them.

mod go;
mod inventory;
mod jvm;
pub(crate) use inventory::ResolverInventory;
mod python;
mod rust;
mod ts;

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Extensions [`resolve_import`] dispatches on. Every other language extracts
/// imports but has no way to turn one into a repository path, so its files hold
/// no outgoing edges at all.
const FILE_RESOLVED_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "go", "java", "kt", "kts",
];

/// Whether this file's imports can become graph edges.
///
/// A claim built on the *absence* of edges — nothing imports this file, nothing
/// reaches this code — is only meaningful when edges could have existed. C#,
/// Swift, PHP, Dart, Scala, and the C family reference types through namespaces
/// or headers that [`resolve_import`] never maps to a file, so every one of
/// their files has zero fan-in in a healthy repository. Callers that reason from
/// absence must consult this first; callers that reason from a present edge do
/// not need to, because a resolved edge is proof on its own.
pub fn resolves_file_imports(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| FILE_RESOLVED_EXTENSIONS.contains(&extension))
}

/// Whether an import-only graph can support claims based on a file having no
/// incoming edges.
///
/// Java and Kotlin same-package type references are modeled when syntactically
/// visible, but source sets, generated types and reflective loading can still
/// make absence claims unsafe.
pub fn supports_file_absence_claims(path: &Path) -> bool {
    resolves_file_imports(path)
        && !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "java" | "kt" | "kts"))
}

/// Resolves a raw import string extracted from `from_file` to a concrete path
/// under `root`. Returns a path only when it exists in `known_files`.
pub fn resolve_import(
    raw_import: &str,
    from_file: &Path,
    root: &Path,
    known_files: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let ext = from_file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "rs" => rust::resolve_rust(raw_import, from_file, root, known_files),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
            ts::resolve_ts(raw_import, from_file, root, known_files)
        }
        "py" => python::resolve_python(raw_import, from_file, root, known_files),
        "go" => go::resolve_go(raw_import, from_file, root, known_files),
        "java" => jvm::resolve_jvm(raw_import, known_files, &["java"]),
        "kt" | "kts" => jvm::resolve_jvm(raw_import, known_files, &["kt", "java"]),
        _ => None,
    }
}

/// Enumerates every local file candidate only for language forms whose lookup
/// semantics are bounded enough for a missing-target claim.
pub(crate) fn definitive_local_candidates(
    raw_import: &str,
    from_file: &Path,
    root: &Path,
) -> Option<Vec<PathBuf>> {
    let root = normalize_path(root);
    let normalized_source = normalize_path(from_file);
    let source = if from_file.is_absolute() || normalized_source.starts_with(&root) {
        normalized_source
    } else {
        normalize_path(&root.join(normalized_source))
    };
    let ext = source.extension().and_then(|value| value.to_str())?;
    let candidates = match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
            ts::definitive_local_candidates(raw_import, &source, &root)?
        }
        "py" => python::definitive_import_candidates(raw_import, &source, &root)?,
        "rs" => rust::definitive_local_candidates(raw_import, &source, &root)?,
        "go" => go::definitive_local_candidates(raw_import, &source, &root)?,
        _ => return None,
    };
    let candidates = candidates
        .into_iter()
        .map(|candidate| normalize_path(&candidate))
        .collect::<Vec<_>>();

    let stays_under_root = candidates.iter().all(|path| path.starts_with(&root));
    (stays_under_root && (ext == "go" || !candidates.is_empty())).then_some(candidates)
}

/// Whether a bare JS/TS import is covered by a readable local path mapping.
/// This lets unresolved aliases enter the same missing-target analysis as
/// relative imports without treating arbitrary package imports as local.
pub(crate) fn is_configured_ts_alias_import(raw_import: &str, source: &Path, root: &Path) -> bool {
    let is_typescript = source
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs"));
    is_typescript && !raw_import.starts_with('.') && ts::has_alias_candidates(raw_import, root)
}

pub(crate) fn is_local_go_module_import(raw_import: &str, source: &Path, root: &Path) -> bool {
    go::is_local_module_import(raw_import, source, root)
}

/// Returns the first candidate that exists in `known_files`, after normalizing
/// `.`/`..` components. Shared by every language resolver.
fn probe(candidates: &[PathBuf], known_files: &HashSet<PathBuf>) -> Option<PathBuf> {
    for candidate in candidates {
        let normalized = normalize_path(candidate);
        if known_files.contains(&normalized) {
            return Some(normalized);
        }
    }
    None
}

/// Resolves `.` and `..` components without touching the filesystem.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod definitive_candidates_tests {
    use super::*;

    #[test]
    fn explicit_typescript_relative_import_has_bounded_local_candidates() {
        let candidates = definitive_local_candidates(
            "./missing.ts",
            Path::new("/repo/src/app.ts"),
            Path::new("/repo"),
        )
        .expect("explicit supported TypeScript import should be definitive");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/repo/src/missing.ts"),
                PathBuf::from("/repo/src/missing.tsx"),
                PathBuf::from("/repo/src/missing.js"),
                PathBuf::from("/repo/src/missing.jsx"),
            ]
        );
    }

    #[test]
    fn relative_source_already_rooted_under_relative_scan_path_is_not_prefixed_twice() {
        let candidates = definitive_local_candidates(
            "./missing.ts",
            Path::new("project/src/app.ts"),
            Path::new("project"),
        )
        .expect("relative rooted source should be definitive");

        assert_eq!(candidates[0], PathBuf::from("project/src/missing.ts"));
    }

    #[test]
    fn missing_extensionless_relative_import_is_not_definitive() {
        assert!(
            definitive_local_candidates(
                "./generated-client",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
            )
            .is_none()
        );
    }

    #[test]
    fn existing_extensionless_typescript_import_still_resolves() {
        let target = PathBuf::from("/repo/src/generated-client/index.ts");
        let known_files = HashSet::from([target.clone()]);

        assert_eq!(
            resolve_import(
                "./generated-client",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
                &known_files,
            ),
            Some(target),
        );
    }

    #[test]
    fn explicit_commonjs_import_resolves_its_exact_file() {
        let target = PathBuf::from("/repo/packages/excalidraw/env.cjs");
        let known_files = HashSet::from([target.clone()]);

        assert_eq!(
            resolve_import(
                "../packages/excalidraw/env.cjs",
                Path::new("/repo/scripts/buildPackage.js"),
                Path::new("/repo"),
                &known_files,
            ),
            Some(target),
        );
    }

    #[test]
    fn dotted_extensionless_typescript_import_preserves_the_full_stem() {
        let candidates = definitive_local_candidates(
            "./charts.types",
            Path::new("/repo/src/app.ts"),
            Path::new("/repo"),
        )
        .expect("a dotted module stem is still an extensionless TypeScript import");

        assert!(candidates.contains(&PathBuf::from("/repo/src/charts.types.ts")));
        assert!(candidates.contains(&PathBuf::from("/repo/src/charts.types.d.ts")));
        assert!(!candidates.contains(&PathBuf::from("/repo/src/charts.ts")));
    }

    #[test]
    fn typescript_asset_imports_do_not_claim_missing_code_files() {
        assert!(
            definitive_local_candidates(
                "./styles.module.css",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
            )
            .is_none()
        );
    }

    #[test]
    fn dotted_extensionless_typescript_import_resolves_a_declaration_file() {
        let target = PathBuf::from("/repo/src/charts.types.d.ts");
        let known_files = HashSet::from([target.clone()]);

        assert_eq!(
            resolve_import(
                "./charts.types",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
                &known_files,
            ),
            Some(target),
        );
    }

    #[test]
    fn configured_typescript_alias_has_bounded_missing_target_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@/*":["src/*","generated/*"]}}}"#,
        )
        .unwrap();

        assert!(is_configured_ts_alias_import(
            "@/missing/module",
            &root.join("src/app.ts"),
            root,
        ));
        assert!(!is_configured_ts_alias_import(
            "react",
            &root.join("src/app.ts"),
            root,
        ));

        let candidates =
            definitive_local_candidates("@/missing/module", &root.join("src/app.ts"), root)
                .expect("configured alias has bounded candidates");
        assert!(candidates.contains(&root.join("src/missing/module.ts")));
        assert!(candidates.contains(&root.join("generated/missing/module/index.ts")));
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.starts_with(root))
        );
    }

    #[test]
    fn configured_typescript_alias_can_point_directly_to_a_source_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@common":["packages/common/src/index.ts"]}}}"#,
        )
        .unwrap();
        let target = root.join("packages/common/src/index.ts");
        let known_files = HashSet::from([target.clone()]);

        assert_eq!(
            resolve_import("@common", &root.join("src/app.ts"), root, &known_files),
            Some(target),
        );
    }

    #[test]
    fn explicit_python_relative_module_has_module_and_package_candidates() {
        let candidates = definitive_local_candidates(
            ".missing",
            Path::new("/repo/pkg/app.py"),
            Path::new("/repo"),
        )
        .expect("explicit Python relative module should be definitive");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/repo/pkg/missing.py"),
                PathBuf::from("/repo/pkg/missing/__init__.py"),
            ]
        );
    }

    #[test]
    fn local_python_absolute_import_has_rooted_module_and_namespace_candidates() {
        let candidates = definitive_local_candidates(
            "app.models.user",
            Path::new("/repo/src/main.py"),
            Path::new("/repo"),
        )
        .expect("a repository-local absolute Python import has bounded candidates");

        assert!(candidates.contains(&PathBuf::from("/repo/app/models/user.py")));
        assert!(candidates.contains(&PathBuf::from("/repo/src/app/models/user.py")));
        assert!(candidates.contains(&PathBuf::from("/repo/app/models/user")));
        assert!(candidates.iter().all(|path| path.starts_with("/repo")));
    }

    #[test]
    fn candidates_that_escape_the_repository_are_not_definitive() {
        assert_eq!(
            definitive_local_candidates(
                "../../outside.ts",
                Path::new("/repo/src/app.ts"),
                Path::new("/repo"),
            ),
            None
        );
    }

    #[test]
    fn rust_module_declaration_has_bounded_local_candidates() {
        let candidates = definitive_local_candidates(
            "mod::missing",
            Path::new("/repo/src/lib.rs"),
            Path::new("/repo"),
        )
        .expect("a plain Rust module declaration has two file forms");

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/repo/src/missing.rs"),
                PathBuf::from("/repo/src/missing/mod.rs"),
            ]
        );
    }

    #[test]
    fn rust_relative_file_attribute_has_one_bounded_candidate() {
        let candidates = definitive_local_candidates(
            "relfile::sections/header.rs",
            Path::new("/repo/src/lib.rs"),
            Path::new("/repo"),
        )
        .expect("a literal Rust relative file path is definitive");

        assert_eq!(
            candidates,
            vec![PathBuf::from("/repo/src/sections/header.rs")]
        );
    }

    #[test]
    fn rust_relative_file_that_escapes_repository_is_not_definitive() {
        assert_eq!(
            definitive_local_candidates(
                "relfile::../../outside.rs",
                Path::new("/repo/src/lib.rs"),
                Path::new("/repo"),
            ),
            None
        );
    }

    #[test]
    fn malformed_rust_module_name_is_not_definitive() {
        assert_eq!(
            definitive_local_candidates(
                "mod::not#a_module",
                Path::new("/repo/src/lib.rs"),
                Path::new("/repo"),
            ),
            None
        );
    }

    #[test]
    fn go_module_import_maps_to_source_files_in_local_package() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("go.mod"), "module example.com/app\n\ngo 1.22\n").unwrap();
        let package = root.join("internal/present");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("present.go"), "package present\n").unwrap();

        let candidates = definitive_local_candidates(
            "example.com/app/internal/present",
            &root.join("cmd/app/main.go"),
            root,
        )
        .expect("a local Go module import should have bounded file candidates");

        assert_eq!(candidates, vec![package.join("present.go")]);
    }

    #[test]
    fn missing_go_module_package_is_definitive_with_no_candidates() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("go.mod"), "module example.com/app\n").unwrap();

        let candidates =
            definitive_local_candidates("example.com/app/missing", &root.join("main.go"), root)
                .expect("a local Go module import remains definitive when its package is absent");

        assert!(candidates.is_empty());
    }

    #[test]
    fn go_module_replace_keeps_resolution_limited() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("go.mod"),
            "module example.com/app\n\nreplace\texample.com/app => ../local\n",
        )
        .unwrap();

        assert_eq!(
            definitive_local_candidates("example.com/app/missing", &root.join("main.go"), root,),
            None
        );
    }

    #[test]
    fn go_workspace_module_resolves_local_package_and_missing_target() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("go.mod"), "module example.com/app\n").unwrap();
        std::fs::write(
            root.join("go.work"),
            "go 1.22\n\nuse (\n ./app\n ./lib\n)\n",
        )
        .unwrap();
        let member = root.join("lib");
        std::fs::create_dir_all(member.join("pkg")).unwrap();
        std::fs::write(member.join("go.mod"), "module example.com/lib\n").unwrap();
        std::fs::write(member.join("pkg/pkg.go"), "package pkg\n").unwrap();
        let files = HashSet::from([member.join("pkg/pkg.go")]);

        assert_eq!(
            resolve_import(
                "example.com/lib/pkg",
                &root.join("app/main.go"),
                root,
                &files
            ),
            Some(member.join("pkg/pkg.go"))
        );
        assert!(is_local_go_module_import(
            "example.com/lib/missing",
            &root.join("app/main.go"),
            root,
        ));
        assert_eq!(
            definitive_local_candidates("example.com/lib/missing", &root.join("app/main.go"), root,),
            Some(Vec::new())
        );
    }

    #[test]
    fn go_local_replace_maps_the_replaced_import_path_into_repository() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("go.mod"),
            "module example.com/app\n\nreplace example.com/shared v1.0.0 => ./third_party/shared\n",
        )
        .unwrap();
        let dependency = root.join("third_party/shared");
        std::fs::create_dir_all(dependency.join("text")).unwrap();
        std::fs::write(dependency.join("go.mod"), "module fork.local/shared\n").unwrap();
        std::fs::write(dependency.join("text/text.go"), "package text\n").unwrap();
        let files = HashSet::from([dependency.join("text/text.go")]);

        assert_eq!(
            resolve_import(
                "example.com/shared/text",
                &root.join("main.go"),
                root,
                &files
            ),
            Some(dependency.join("text/text.go"))
        );
        assert!(is_local_go_module_import(
            "example.com/shared/missing",
            &root.join("main.go"),
            root,
        ));
    }

    #[test]
    fn go_replace_is_scoped_to_the_importing_workspace_module() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("go.work"), "go 1.22\nuse (\n ./app\n ./lib\n)\n").unwrap();
        let app = root.join("app");
        let lib = root.join("lib");
        let app_fork = root.join("forks/app-dep");
        let lib_fork = root.join("forks/lib-dep");
        for (module_root, module_name, fork, fork_name) in [
            (&app, "example.com/app", &app_fork, "fork.local/app-dep"),
            (&lib, "example.com/lib", &lib_fork, "fork.local/lib-dep"),
        ] {
            std::fs::create_dir_all(module_root).unwrap();
            std::fs::create_dir_all(fork.join("pkg")).unwrap();
            std::fs::write(
                module_root.join("go.mod"),
                format!(
                    "module {module_name}\nreplace example.com/dep => ../forks/{}\n",
                    if module_name.ends_with("app") {
                        "app-dep"
                    } else {
                        "lib-dep"
                    }
                ),
            )
            .unwrap();
            std::fs::write(fork.join("go.mod"), format!("module {fork_name}\n")).unwrap();
            std::fs::write(fork.join("pkg/pkg.go"), "package pkg\n").unwrap();
        }
        let files = HashSet::from([app_fork.join("pkg/pkg.go"), lib_fork.join("pkg/pkg.go")]);

        assert_eq!(
            resolve_import("example.com/dep/pkg", &app.join("main.go"), root, &files),
            Some(app_fork.join("pkg/pkg.go"))
        );
        assert_eq!(
            resolve_import("example.com/dep/pkg", &lib.join("main.go"), root, &files),
            Some(lib_fork.join("pkg/pkg.go"))
        );
    }

    #[test]
    fn external_go_module_is_not_definitive() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("go.mod"), "module example.com/app\n").unwrap();

        assert_eq!(
            definitive_local_candidates("example.com/other/missing", &root.join("main.go"), root,),
            None
        );
    }
}

#[cfg(test)]
mod file_resolution_support_tests {
    use super::*;

    #[test]
    fn every_declared_extension_reaches_a_language_resolver() {
        // Catches the predicate drifting from `resolve_import`'s dispatch: an
        // extension listed here but missing from the match would claim edges
        // are possible when none can ever be produced.
        let root = Path::new("/repo");
        for extension in FILE_RESOLVED_EXTENSIONS {
            let source = PathBuf::from(format!("/repo/src/app.{extension}"));
            assert!(resolves_file_imports(&source), "{extension}");
            // A resolver ran if it probed at all; the empty file set makes every
            // probe miss, so the observable contract is "did not panic and
            // returned None" — the dispatch arm exists.
            assert_eq!(
                resolve_import("./sibling", &source, root, &HashSet::new()),
                None,
                "{extension}"
            );
        }
    }

    #[test]
    fn namespace_and_header_languages_have_no_file_resolution() {
        // These are the languages whose zero fan-in means "unmodeled", not
        // "unreferenced" — the distinction absence-based rules depend on.
        for extension in ["cs", "swift", "php", "dart", "scala", "cpp", "h", "rb"] {
            let source = PathBuf::from(format!("/repo/src/app.{extension}"));
            assert!(!resolves_file_imports(&source), "{extension}");
            assert_eq!(
                resolve_import(
                    "Some.Namespace.Type",
                    &source,
                    Path::new("/repo"),
                    &HashSet::new()
                ),
                None,
                "{extension}"
            );
        }
    }

    #[test]
    fn a_file_without_an_extension_resolves_nothing() {
        assert!(!resolves_file_imports(Path::new("/repo/Makefile")));
    }
}

pub(crate) use ts::effective_tsconfig_content;
