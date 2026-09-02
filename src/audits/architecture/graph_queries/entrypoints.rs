//! Files nothing imports *by design*, so zero fan-in is not evidence of death.
//!
//! `architecture.dead-module` reasons from an absence: no file imports this one.
//! That inference only holds when being imported is how the file was ever meant
//! to be reached. A large class of real files is reached another way — a tool
//! reads them by name, a build system executes them, a framework discovers them
//! by convention, a router maps them from their path — and their fan-in is zero
//! in every healthy repository.
//!
//! Sampled zoo evidence put the rule at 0.00 precision, and every sampled
//! finding was one of these: an ESLint flat config, a Gradle settings script, a
//! Django management command, a Wagtail hooks module, and a documentation
//! example. This module names those shapes so the rule stops claiming them.
//!
//! The recognizers are deliberately structural — a path convention a tool or
//! framework actually implements — rather than a list of well-known filenames.
//! A missed convention costs one false positive; an over-broad rule silently
//! hides real dead code, so each entry below names the loader that reads it.

use crate::audits::context::classify::helpers::path_contains_component;
use std::path::Path;

/// How a zero-fan-in file is reached when no import points at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReachedWithoutImport {
    /// Read by name by a development tool (`eslint.config.mjs`, `.babelrc.js`).
    ToolConfiguration,
    /// Executed by a build system (`settings.gradle.kts`, `gulpfile.js`).
    BuildScript,
    /// Discovered by a framework or tool convention — a directory scan, a glob,
    /// or a fixed module name (`management/commands/*.py`, `wagtail_hooks.py`,
    /// `conftest.py`, `*.stories.tsx`).
    FrameworkAutoload,
    /// A Python package's `__init__.py`, executed whenever any module inside the
    /// package is imported, with no edge pointing at it.
    PackageMarker,
    /// Mapped from its path by a file-system router (Next.js `app/page.tsx`).
    FileSystemRoute,
    /// Example or documentation source, compiled or imported by name from docs
    /// tooling and test suites rather than by a static import.
    DocumentationExample,
    /// A standalone script under `scripts/`, run by a person or a CI job.
    StandaloneScript,
    /// A TypeScript declaration file. It carries no runtime code at all, so it
    /// is never imported as a module and can never be dead.
    TypeDeclaration,
    /// Third-party code checked into the repository. Not project source.
    VendoredCode,
    /// Named by a bundler entry map, a `<script>` tag, or a worker constructor
    /// rather than by an import in project code.
    BrowserEntry,
    /// A package's executable, named by `package.json`'s `bin` field.
    PackageBinary,
    /// A command module in a CLI package, dispatched by file name.
    CommandModule,
}

pub(super) fn reached_without_import(
    path: &Path,
    in_executable_package: bool,
) -> Option<ReachedWithoutImport> {
    let name = file_name(path);
    if name.ends_with(".d.ts") || name.ends_with(".d.mts") || name.ends_with(".d.cts") {
        return Some(ReachedWithoutImport::TypeDeclaration);
    }
    if path_contains_component(path, &["vendor", "vendors", "node_modules", "third_party"]) {
        return Some(ReachedWithoutImport::VendoredCode);
    }
    if name == "__init__.py" {
        return Some(ReachedWithoutImport::PackageMarker);
    }
    if is_documentation_example(path) {
        return Some(ReachedWithoutImport::DocumentationExample);
    }
    if is_browser_entry(path, &name) {
        return Some(ReachedWithoutImport::BrowserEntry);
    }
    if is_package_binary(path, &name) {
        return Some(ReachedWithoutImport::PackageBinary);
    }
    if in_executable_package && path_contains_component(path, &["commands"]) {
        return Some(ReachedWithoutImport::CommandModule);
    }
    if is_build_script(&name) {
        return Some(ReachedWithoutImport::BuildScript);
    }
    if is_tool_configuration(&name) {
        return Some(ReachedWithoutImport::ToolConfiguration);
    }
    if is_framework_autoload(path, &name) {
        return Some(ReachedWithoutImport::FrameworkAutoload);
    }
    if is_file_system_route(path, &name) {
        return Some(ReachedWithoutImport::FileSystemRoute);
    }
    if path_contains_component(path, &["scripts"]) {
        return Some(ReachedWithoutImport::StandaloneScript);
    }
    None
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// `<tool>.config.<ext>` and `.<tool>rc.<ext>` — the two conventions the
/// JavaScript ecosystem uses for executable configuration a tool loads by name.
fn is_tool_configuration(name: &str) -> bool {
    let Some(stem) = script_stem(name) else {
        return false;
    };
    stem.ends_with(".config")
        || stem.ends_with(".conf")
        || (stem.starts_with('.') && stem.ends_with("rc"))
}

/// Scripts a build system executes directly. Gradle accepts both its Groovy and
/// Kotlin DSL, and the JS task runners load their file by a fixed name.
fn is_build_script(name: &str) -> bool {
    if name.ends_with(".gradle") || name.ends_with(".gradle.kts") {
        return true;
    }
    matches!(script_stem(name).as_deref(), Some("gulpfile" | "gruntfile"))
}

/// Conventions where a framework or tool imports the module for you — a
/// directory scan, a glob, or a fixed module name — so no source file names it.
fn is_framework_autoload(path: &Path, name: &str) -> bool {
    // Storybook and its ecosystem collect `*.stories.*` by glob from a
    // configured directory; nothing in the application imports a story.
    if let Some(stem) = script_stem(name)
        && (stem.ends_with(".stories") || stem.ends_with(".story"))
    {
        return true;
    }
    if !name.ends_with(".py") {
        return false;
    }
    // Django loads every module under `<app>/management/commands/` and invokes
    // it by command name; the template engine loads `<app>/templatetags/` by
    // library name from `{% load %}`; the migration runner does the same for
    // `<app>/migrations/`.
    if has_directory_pair(path, "management", "commands")
        || path_contains_component(path, &["migrations", "templatetags"])
    {
        return true;
    }
    matches!(
        name,
        // pytest collects every conftest.py in the rootdir tree.
        "conftest.py"
            // Wagtail: `get_app_submodules("wagtail_hooks")` imports each app's
            // hooks module dynamically.
            | "wagtail_hooks.py"
            // Django app plumbing, reached from settings strings and the app
            // registry rather than from an import in project code.
            | "apps.py"
            | "admin.py"
            | "urls.py"
            | "wsgi.py"
            | "asgi.py"
    )
}

// Framework routing derives the route from a reserved path, so these modules
// need not be imported by name.
fn is_file_system_route(path: &Path, name: &str) -> bool {
    let Some(stem) = script_stem(name) else {
        return false;
    };
    let routed = matches!(
        stem.as_str(),
        "page"
            | "layout"
            | "_layout"
            | "route"
            | "template"
            | "loading"
            | "error"
            | "not-found"
            | "+not-found"
            | "+html"
            | "_app"
            | "_document"
    );
    // These names are reserved only inside a router tree, so an unrelated
    // `page.tsx` in `src/components/` still counts as importable.
    routed && path_contains_component(path, &["app", "pages"])
}

/// Code the browser loads without any module importing it: a bundler names it
/// in an entry map, a template names it in a `<script>` tag, or a worker
/// constructor names it by URL.
fn is_browser_entry(path: &Path, name: &str) -> bool {
    if path_contains_component(path, &["entrypoints", "static_src", "static"]) {
        return true;
    }
    // `subset-worker.chunk.ts`, `search.worker.ts` — a worker entry is loaded
    // through `new Worker(url)`, never through an import.
    script_stem(name).is_some_and(|stem| {
        stem.split(['.', '-'])
            .any(|token| token == "worker" || token == "sw")
    })
}

/// A package's `bin` entry, run by npm rather than imported.
fn is_package_binary(path: &Path, name: &str) -> bool {
    path_contains_component(path, &["bin"])
        || script_stem(name).is_some_and(|stem| stem == "bin" || stem == "cli")
}

/// Example and documentation source trees. Their files are compiled by docs
/// tooling, run as standalone examples, or imported by name from a test
/// parameter — never through a static import another module makes.
///
/// Such a tree sits near the repository or package root. Deeper down, the same
/// words are ordinary namespace segments — `com/example/...` is the canonical
/// Java package placeholder and Spring's PetClinic lives under
/// `org/springframework/samples/petclinic/` — so matching at any depth would
/// silence a whole application's worth of real source.
const EXAMPLE_TREE_MAX_DEPTH: usize = 2;

fn is_documentation_example(path: &Path) -> bool {
    const EXAMPLE_DIRECTORIES: &[&str] =
        &["docs_src", "doc_src", "docs-src", "examples", "samples"];
    path.to_string_lossy()
        .split(['/', '\\'])
        .take(EXAMPLE_TREE_MAX_DEPTH + 1)
        .any(|component| EXAMPLE_DIRECTORIES.contains(&component.to_lowercase().as_str()))
}

/// Two path components that must be adjacent, so an unrelated `commands/`
/// directory does not read as Django's command loader.
fn has_directory_pair(path: &Path, first: &str, second: &str) -> bool {
    let text = path.to_string_lossy().replace('\\', "/").to_lowercase();
    text.contains(&format!("/{first}/{second}/")) || text.starts_with(&format!("{first}/{second}/"))
}

/// The file name minus a single script extension, or `None` when the extension
/// is not one these conventions use.
fn script_stem(name: &str) -> Option<String> {
    for extension in [
        ".mts", ".cts", ".mjs", ".cjs", ".tsx", ".jsx", ".ts", ".js", ".kts",
    ] {
        if let Some(stem) = name.strip_suffix(extension) {
            return Some(stem.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests;
