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

// Skip documentation trees at the package root, but keep `docs/` modules that
// live below a source root (`src`, `lib`, or `app`).
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
            && !components[..index]
                .iter()
                .any(|ancestor| SOURCE_ROOTS.contains(&ancestor.as_str()))
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
        // pytest and Django name the test, not the module: `test_workflows.py`.
        || stem.starts_with("test_")
        // JUnit names the class: `VetTests.java`, `VetControllerTests.java`.
        || (is_jvm(path) && (stem.ends_with("Test") || stem.ends_with("Tests")))
}

fn is_jvm(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("java" | "kt")
    )
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
