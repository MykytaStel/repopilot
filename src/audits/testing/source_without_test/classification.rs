use std::path::Path;

const SOURCE_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt"];

pub(super) fn is_source_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    SOURCE_EXTENSIONS.contains(&ext) && !is_declaration_file(path) && !is_excluded_directory(path)
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

    // Python packaging/framework boilerplate and entrypoints: configuration and
    // wiring, not behaviour that warrants a unit test. `apps.py` is a Django
    // `AppConfig` registration stub; `manage.py`/`wsgi.py`/`asgi.py` are the
    // CLI and server entrypoints — the analogue of `__main__.py`.
    matches!(
        name,
        "setup.py"
            | "settings.py"
            | "conftest.py"
            | "__main__.py"
            | "apps.py"
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
                | "docs"
                | "docs_src"
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
