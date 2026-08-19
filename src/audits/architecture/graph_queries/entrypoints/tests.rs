use super::{ReachedWithoutImport, reached_without_import};
use std::path::Path;

fn reason(path: &str) -> Option<ReachedWithoutImport> {
    reached_without_import(Path::new(path), false)
}

fn reason_in_cli(path: &str) -> Option<ReachedWithoutImport> {
    reached_without_import(Path::new(path), true)
}

#[test]
fn the_zoo_sampled_false_positives_are_all_recognized() {
    // Catches losing any of the six findings that put this rule at 0.00
    // precision in the first strict sample.
    assert_eq!(
        reason("eslint.config.mjs"),
        Some(ReachedWithoutImport::ToolConfiguration)
    );
    assert_eq!(
        reason("settings.gradle.kts"),
        Some(ReachedWithoutImport::BuildScript)
    );
    assert_eq!(
        reason("wagtail/management/commands/purge_revisions.py"),
        Some(ReachedWithoutImport::FrameworkAutoload)
    );
    assert_eq!(
        reason("wagtail/users/wagtail_hooks.py"),
        Some(ReachedWithoutImport::FrameworkAutoload)
    );
    assert_eq!(
        reason("docs_src/query_params_str_validations/tutorial008_an_py310.py"),
        Some(ReachedWithoutImport::DocumentationExample)
    );
}

#[test]
fn executable_configuration_is_recognized_across_module_syntaxes() {
    // Catches pinning the recognizer to one extension: the same tool config is
    // written as .js, .cjs, .mjs, or .ts depending on the project's module mode.
    for path in [
        "vite.config.ts",
        "jest.config.cjs",
        "rollup.config.mjs",
        "tailwind.config.js",
        "karma.conf.js",
        ".eslintrc.js",
        ".prettierrc.cjs",
    ] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::ToolConfiguration),
            "{path}"
        );
    }
}

#[test]
fn gradle_scripts_are_recognized_in_both_dsls() {
    for path in ["settings.gradle", "build.gradle", "app/build.gradle.kts"] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::BuildScript),
            "{path}"
        );
    }
    assert_eq!(
        reason("gulpfile.js"),
        Some(ReachedWithoutImport::BuildScript)
    );
}

#[test]
fn django_command_loader_needs_both_path_components_in_order() {
    // Catches matching any `commands/` directory: a CQRS command handler is
    // ordinary importable code and must stay eligible for the rule.
    assert_eq!(
        reason("app/management/commands/sync.py"),
        Some(ReachedWithoutImport::FrameworkAutoload)
    );
    assert_eq!(
        reason("management/commands/sync.py"),
        Some(ReachedWithoutImport::FrameworkAutoload)
    );
    assert_eq!(reason("app/domain/commands/create_order.py"), None);
    assert_eq!(reason("app/commands/management/sync.py"), None);
}

#[test]
fn django_template_tag_libraries_are_loaded_by_name() {
    // `{% load wagtailusers_tags %}` names the library, not a module path, so
    // no Python file imports it.
    assert_eq!(
        reason("wagtail/users/templatetags/wagtailusers_tags.py"),
        Some(ReachedWithoutImport::FrameworkAutoload)
    );
    assert_eq!(reason("wagtail/users/views/groups.py"), None);
}

#[test]
fn file_system_routes_are_only_reserved_inside_a_router_tree() {
    // Catches treating every `page.tsx` as routed: outside `app/` or `pages/`
    // the name carries no framework meaning.
    assert_eq!(
        reason("app/dashboard/page.tsx"),
        Some(ReachedWithoutImport::FileSystemRoute)
    );
    assert_eq!(
        reason("pages/api/users.ts"),
        None,
        "an arbitrary module inside pages/ is still importable"
    );
    assert_eq!(reason("src/components/page.tsx"), None);
}

#[test]
fn expo_and_next_router_names_are_recognized() {
    // Catches covering only the Next.js App Router: Expo Router uses `_layout`
    // and `+not-found`, and the Pages Router uses `_app`/`_document`.
    for path in [
        "boilerplate/src/app/_layout.tsx",
        "src/app/+not-found.tsx",
        "pages/_app.tsx",
        "pages/_document.tsx",
    ] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::FileSystemRoute),
            "{path}"
        );
    }
}

#[test]
fn a_python_package_marker_is_never_dead() {
    // Importing `pkg.sub` executes `pkg/__init__.py` without any edge pointing
    // at it, so zero fan-in there proves nothing.
    assert_eq!(
        reason("wagtail/users/views/__init__.py"),
        Some(ReachedWithoutImport::PackageMarker)
    );
    assert_eq!(reason("wagtail/users/views/groups.py"), None);
}

#[test]
fn storybook_stories_are_collected_by_glob() {
    for path in [
        "wagtail/admin/templates/shared/status_tag.stories.tsx",
        "src/components/Button.stories.ts",
        "src/components/Button.story.jsx",
    ] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::FrameworkAutoload),
            "{path}"
        );
    }
    assert_eq!(reason("src/components/Button.tsx"), None);
}

#[test]
fn example_trees_are_recognized_only_near_a_root() {
    // Catches matching `examples`/`samples` at any depth: those words are
    // ordinary namespace segments inside a package path, and suppressing there
    // hides an entire application's source.
    for path in [
        "docs_src/tutorial.py",
        "examples/basic.rs",
        "packages/core/examples/usage.ts",
    ] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::DocumentationExample),
            "{path}"
        );
    }
    for path in [
        "src/main/java/org/springframework/samples/petclinic/owner/Owner.java",
        "src/main/java/com/example/app/Service.java",
    ] {
        assert_eq!(reason(path), None, "{path}");
    }
}

#[test]
fn standalone_scripts_are_run_not_imported() {
    assert_eq!(
        reason("scripts/get-translator-credits.py"),
        Some(ReachedWithoutImport::StandaloneScript)
    );
    assert_eq!(
        reason("tools/scripts/release.ts"),
        Some(ReachedWithoutImport::StandaloneScript)
    );
    assert_eq!(reason("src/scripting/engine.ts"), None);
}

#[test]
fn ordinary_source_files_stay_eligible_for_the_rule() {
    // The false-negative guard: broadening any recognizer here silently hides
    // real dead code, so the common shapes must keep returning None.
    for path in [
        "src/lib/user_service.ts",
        "src/config.ts",
        "src/config/database.ts",
        "wagtail/models.py",
        "wagtail/users/views.py",
        "internal/server/handler.go",
        "app/services/billing.rb",
    ] {
        assert_eq!(reason(path), None, "{path}");
    }
}

#[test]
fn python_autoload_names_do_not_leak_into_other_languages() {
    // `urls`, `apps`, and `admin` are ordinary module names elsewhere; only the
    // Python files carry Django's loader semantics.
    assert_eq!(reason("src/urls.ts"), None);
    assert_eq!(reason("src/admin.js"), None);
    assert_eq!(
        reason("project/urls.py"),
        Some(ReachedWithoutImport::FrameworkAutoload)
    );
}

#[test]
fn type_declarations_carry_no_runtime_code() {
    // A `.d.ts` declares ambient types and compiles to nothing, so "no module
    // imports it" says nothing about dead code.
    for path in [
        "client/src/custom.d.ts",
        "client/storybook/stories.d.ts",
        "types/global.d.mts",
    ] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::TypeDeclaration),
            "{path}"
        );
    }
    assert_eq!(reason("client/src/custom.ts"), None);
}

#[test]
fn vendored_third_party_code_is_not_project_source() {
    for path in [
        "wagtail/admin/static_src/js/vendor/bootstrap-modal.js",
        "third_party/lib/parser.py",
    ] {
        assert_eq!(
            reason(path),
            Some(ReachedWithoutImport::VendoredCode),
            "{path}"
        );
    }
}

#[test]
fn browser_entries_are_named_by_a_bundler_template_or_worker() {
    // wagtail's webpack config builds its entry map from
    // `./client/src/entrypoints/${app}/${module}.js`, and its templates load
    // `static_src` assets through a `{% versioned_static %}` script tag.
    assert_eq!(
        reason("client/src/entrypoints/admin/page-chooser.js"),
        Some(ReachedWithoutImport::BrowserEntry)
    );
    assert_eq!(
        reason("wagtail/images/static_src/wagtailimages/js/add-multiple.js"),
        Some(ReachedWithoutImport::BrowserEntry)
    );
    assert_eq!(
        reason("packages/excalidraw/subset/subset-worker.chunk.ts"),
        Some(ReachedWithoutImport::BrowserEntry)
    );
    assert_eq!(
        reason("src/search.worker.ts"),
        Some(ReachedWithoutImport::BrowserEntry)
    );
    // A module that merely mentions the word is ordinary code.
    assert_eq!(reason("src/workerPool.ts"), None);
    assert_eq!(reason("src/staticData.ts"), None);
}

#[test]
fn package_binaries_are_run_by_the_package_manager() {
    assert_eq!(
        reason("packages/cli/bin.js"),
        Some(ReachedWithoutImport::PackageBinary)
    );
    assert_eq!(
        reason("packages/cli/bin/run.js"),
        Some(ReachedWithoutImport::PackageBinary)
    );
    assert_eq!(reason("src/binary_search.ts"), None);
}

#[test]
fn command_modules_need_the_package_to_declare_an_executable() {
    // The CQRS guard: a `commands/` directory in an ordinary library is
    // importable code, and only a package that declares a `bin` dispatches its
    // command modules by file name.
    assert_eq!(
        reason_in_cli("src/commands/cache.ts"),
        Some(ReachedWithoutImport::CommandModule)
    );
    assert_eq!(
        reason("src/commands/cache.ts"),
        None,
        "the same path in a library package stays eligible"
    );
    assert_eq!(reason_in_cli("src/domain/order.ts"), None);
}
