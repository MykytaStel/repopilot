use std::path::Path;

const SOURCE_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt"];

pub(super) fn is_source_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    SOURCE_EXTENSIONS.contains(&ext)
        && !is_declaration_file(path)
        && !is_excluded_directory(path)
        && !is_documentation_path(path)
}

/// Documentation / example source that does not warrant a unit test.
///
/// A `docs_src/` tree is an unambiguous documentation name and is skipped wherever
/// it appears. A bare `docs/` directory is trickier: at a package root (repo root
/// or a monorepo package such as `packages/<pkg>/docs/`) it holds rendered
/// examples, but nested inside a source tree (`src/docs/`, `lib/docs/`,
/// `app/docs/`) it is an ordinary `docs` domain module — common in a
/// document-management app — and must stay visible. We therefore treat `docs/` as
/// documentation only when the segment immediately above it is *not* a source
/// root. This is position-independent, so it holds regardless of any scan-root
/// prefix on the path.
fn is_documentation_path(path: &Path) -> bool {
    const SOURCE_ROOTS: &[&str] = &["src", "lib", "app"];
    let components: Vec<String> = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();

    if components.iter().any(|component| component == "docs_src") {
        return true;
    }

    components.iter().enumerate().any(|(index, component)| {
        component == "docs"
            && !index
                .checked_sub(1)
                .map(|parent| components[parent].as_str())
                .is_some_and(|parent| SOURCE_ROOTS.contains(&parent))
    })
}

pub(super) fn is_test_file(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    stem == "tests"
        || stem == "test"
        || stem.ends_with("_test")
        || stem.ends_with(".test")
        || stem.ends_with(".spec")
}

pub(super) fn is_low_signal_wrapper(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    if matches!(name, "mod.rs" | "lib.rs" | "main.rs" | "build.rs") {
        return true;
    }

    if name.ends_with("stub.rs") || name.ends_with("mock.rs") || name.ends_with("fakes.rs") {
        return true;
    }

    if matches!(
        name,
        "index.ts"
            | "index.tsx"
            | "index.js"
            | "index.jsx"
            | "main.ts"
            | "main.tsx"
            | "main.js"
            | "main.jsx"
            | "__init__.py"
    ) {
        return true;
    }

    if matches!(
        name,
        "types.ts"
            | "types.js"
            | "constants.ts"
            | "constants.js"
            | "tokens.ts"
            | "tokens.js"
            | "theme.ts"
            | "theme.js"
            | "colors.ts"
            | "colors.js"
            | "enums.ts"
            | "enums.js"
            | "globals.ts"
            | "globals.js"
    ) {
        return true;
    }

    if name.ends_with(".types.ts")
        || name.ends_with(".type.ts")
        || name.ends_with(".config.ts")
        || name.ends_with(".config.tsx")
        || name.ends_with(".config.js")
        || name.ends_with(".config.jsx")
        || name.ends_with(".config.mjs")
        || name.ends_with(".config.cjs")
        || name.ends_with(".constants.ts")
        || name.ends_with(".tokens.ts")
        || name.ends_with(".d.ts")
    {
        return true;
    }

    // Python packaging/framework entrypoints: CLI/server wiring, not behaviour
    // that warrants a unit test. `manage.py`/`wsgi.py`/`asgi.py` are the standard
    // Django entrypoints — the analogue of `__main__.py`. `apps.py` is *not*
    // listed: the filename alone is not evidence of a declarative `AppConfig`
    // stub (it can hold a `ready()` with real startup behaviour, or be an
    // ordinary `apps` module), and skipping it by name would hide untested
    // production code.
    matches!(
        name,
        "setup.py"
            | "settings.py"
            | "conftest.py"
            | "__main__.py"
            | "manage.py"
            | "wsgi.py"
            | "asgi.py"
    )
}

fn is_declaration_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts")
}

fn is_excluded_directory(path: &Path) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            "tests"
                | "test"
                | "__tests__"
                | "spec"
                | "fixtures"
                | "bin"
                | "scripts"
                | "script"
                | "tools"
                | "tool"
                | "examples"
                | "example"
                | "types"
                | "@types"
                | "generated"
                | "__generated__"
                | "gen"
                | "codegen"
                | "mocks"
                | "__mocks__"
                | "assets"
                | "public"
                | "migrations"
        )
    })
}
