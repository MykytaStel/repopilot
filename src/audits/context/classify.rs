use crate::audits::context::model::{
    AuditContext, FileRole, FrameworkKind, LanguageKind, ProgrammingParadigm, RuntimeKind,
};
use crate::knowledge::language::language_kind_for_file;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub fn classify_file(file: &FileFacts) -> AuditContext {
    let content = file.content.as_deref().unwrap_or("");
    let language = classify_language(file);
    let is_test = is_test_file(&file.path, file.has_inline_tests);

    let mut frameworks = Vec::new();
    classify_frameworks(&mut frameworks, &file.path, content, language);

    let mut roles = Vec::new();
    classify_roles(
        &mut roles,
        &file.path,
        content,
        language,
        &frameworks,
        is_test,
    );

    if roles.is_empty() {
        roles.push(FileRole::Unknown);
    }

    let mut paradigms = Vec::new();
    classify_paradigms(
        &mut paradigms,
        &file.path,
        content,
        language,
        &frameworks,
        &roles,
    );

    let mut runtimes = Vec::new();
    classify_runtimes(&mut runtimes, &file.path, content, language, &frameworks);

    AuditContext {
        language,
        frameworks,
        roles,
        paradigms,
        runtimes,
        is_test,
    }
}

fn classify_language(file: &FileFacts) -> LanguageKind {
    language_kind_for_file(file)
}

fn classify_frameworks(
    frameworks: &mut Vec<FrameworkKind>,
    path: &Path,
    content: &str,
    language: LanguageKind,
) {
    let normalized_content = content.to_lowercase();

    if is_js_or_ts(language) {
        if is_react_native_content(&normalized_content) {
            push_unique(frameworks, FrameworkKind::ReactNative);
        }

        if normalized_content.contains("from 'expo")
            || normalized_content.contains("from \"expo")
            || normalized_content.contains("expo-status-bar")
            || normalized_content.contains("expo-router")
        {
            push_unique(frameworks, FrameworkKind::Expo);
        }

        if is_react_file(path, &normalized_content) {
            push_unique(frameworks, FrameworkKind::React);
        }

        if normalized_content.contains("next/")
            || (path_contains_component(path, &["pages", "app"]) && is_tsx_or_jsx_file(path))
        {
            push_unique(frameworks, FrameworkKind::NextJs);
        }

        if normalized_content.contains("from 'vue'")
            || normalized_content.contains("from \"vue\"")
            || normalized_content.contains("@vue/")
        {
            push_unique(frameworks, FrameworkKind::Vue);
        }

        if normalized_content.contains("@angular/") {
            push_unique(frameworks, FrameworkKind::Angular);
        }

        if normalized_content.contains("from 'svelte")
            || normalized_content.contains("from \"svelte")
        {
            push_unique(frameworks, FrameworkKind::Svelte);
        }

        if normalized_content.contains("@nestjs/") {
            push_unique(frameworks, FrameworkKind::NestJs);
        }

        if normalized_content.contains("express") {
            push_unique(frameworks, FrameworkKind::Express);
        }

        if normalized_content.contains("express")
            || normalized_content.contains("from 'node:")
            || normalized_content.contains("from \"node:")
            || normalized_content.contains("process.env")
            || normalized_content.contains("process.exit")
        {
            push_unique(frameworks, FrameworkKind::NodeJs);
        }
    }

    if language == LanguageKind::CSharp {
        if is_unity_file(path, &normalized_content) {
            push_unique(frameworks, FrameworkKind::Unity);
        }

        if is_dotnet_file(path, &normalized_content) {
            push_unique(frameworks, FrameworkKind::DotNet);
        }
    }

    if language == LanguageKind::Python {
        if normalized_content.contains("django") {
            push_unique(frameworks, FrameworkKind::Django);
        }
        if normalized_content.contains("flask") {
            push_unique(frameworks, FrameworkKind::Flask);
        }
        if normalized_content.contains("fastapi") {
            push_unique(frameworks, FrameworkKind::FastApi);
        }
    }

    if language == LanguageKind::Go {
        if normalized_content.contains("github.com/gin-gonic/gin") {
            push_unique(frameworks, FrameworkKind::Gin);
        }
        if normalized_content.contains("github.com/labstack/echo") {
            push_unique(frameworks, FrameworkKind::Echo);
        }
        if normalized_content.contains("github.com/gofiber/fiber") {
            push_unique(frameworks, FrameworkKind::Fiber);
        }
    }

    if matches!(language, LanguageKind::Java | LanguageKind::Kotlin) {
        if normalized_content.contains("org.springframework")
            || normalized_content.contains("@springbootapplication")
        {
            push_unique(frameworks, FrameworkKind::Spring);
        }
        if normalized_content.contains("android.")
            || normalized_content.contains("androidx.")
            || path_contains_component(path, &["android"])
        {
            push_unique(frameworks, FrameworkKind::Android);
        }
    }

    if language == LanguageKind::Dart
        && (normalized_content.contains("package:flutter")
            || path_contains_component(path, &["lib", "widgets"]))
    {
        push_unique(frameworks, FrameworkKind::Flutter);
    }
}

fn classify_roles(
    roles: &mut Vec<FileRole>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
    is_test: bool,
) {
    let normalized_content = content.to_lowercase();

    if is_config_file(path) {
        push_unique(roles, FileRole::Config);
    }

    if is_generated_file(path, &normalized_content) {
        push_unique(roles, FileRole::Generated);
    }

    if is_test {
        push_unique(roles, FileRole::Test);

        if language == LanguageKind::Rust {
            push_unique(roles, FileRole::RustTest);
        }
    }

    if is_js_or_ts(language) {
        if is_react_hook_file(path, content) {
            push_unique(roles, FileRole::ReactHook);
            push_unique(roles, FileRole::FrameworkHook);
        }

        if frameworks.contains(&FrameworkKind::React) && is_react_component_file(path, content) {
            push_unique(roles, FileRole::ReactComponent);
            push_unique(roles, FileRole::FrameworkComponent);
        }
    }

    if language == LanguageKind::CSharp {
        if frameworks.contains(&FrameworkKind::Unity)
            && normalized_content.contains("monobehaviour")
        {
            push_unique(roles, FileRole::UnityMonoBehaviour);
        }

        if is_dotnet_controller(path, &normalized_content) {
            push_unique(roles, FileRole::DotNetController);
            push_unique(roles, FileRole::FrameworkController);
        }

        if is_dotnet_service(path, &normalized_content) {
            push_unique(roles, FileRole::DotNetService);
            push_unique(roles, FileRole::FrameworkService);
        }
    }

    if is_app_entrypoint(path, content, language) {
        push_unique(roles, FileRole::AppEntrypoint);
    }

    if matches!(language, LanguageKind::Python | LanguageKind::Go)
        && path_contains_component(path, &["cmd", "bin", "scripts"])
    {
        push_unique(roles, FileRole::Script);
    }

    if path_contains_component(
        path,
        &["domain", "domains", "model", "models", "entity", "entities"],
    ) {
        push_unique(roles, FileRole::Domain);
    }

    if path_contains_component(path, &["script", "scripts", "bin", "tools"]) {
        push_unique(roles, FileRole::Script);
    }
}

fn classify_paradigms(
    paradigms: &mut Vec<ProgrammingParadigm>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
    roles: &[FileRole],
) {
    let normalized_content = content.to_lowercase();

    if roles.contains(&FileRole::ReactComponent) {
        push_unique(paradigms, ProgrammingParadigm::DeclarativeUi);
        push_unique(paradigms, ProgrammingParadigm::Functional);
    }

    if roles.contains(&FileRole::ReactHook) {
        push_unique(paradigms, ProgrammingParadigm::Functional);
        push_unique(paradigms, ProgrammingParadigm::Reactive);
    }

    if frameworks.contains(&FrameworkKind::Unity) {
        push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
        push_unique(paradigms, ProgrammingParadigm::DataOriented);
    }

    if language == LanguageKind::CSharp
        && (normalized_content.contains("class ")
            || normalized_content.contains("interface ")
            || normalized_content.contains("record "))
    {
        push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
    }

    if language == LanguageKind::Rust {
        if normalized_content.contains("impl ")
            || normalized_content.contains("trait ")
            || normalized_content.contains("struct ")
            || normalized_content.contains("enum ")
        {
            push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
        }

        if normalized_content.contains(".map(")
            || normalized_content.contains(".filter(")
            || normalized_content.contains(".fold(")
            || normalized_content.contains(".and_then(")
            || normalized_content.contains(".unwrap_or_else(")
        {
            push_unique(paradigms, ProgrammingParadigm::Functional);
        }

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == "main.rs")
            .unwrap_or(false)
        {
            push_unique(paradigms, ProgrammingParadigm::Procedural);
        }
    }

    if is_js_or_ts(language) {
        if normalized_content.contains("function ")
            || normalized_content.contains("=>")
            || normalized_content.contains(".map(")
            || normalized_content.contains(".filter(")
            || normalized_content.contains(".reduce(")
        {
            push_unique(paradigms, ProgrammingParadigm::Functional);
        }

        if normalized_content.contains("class ") {
            push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
        }
    }

    if matches!(language, LanguageKind::Python | LanguageKind::Go)
        && (normalized_content.contains("def main(")
            || normalized_content.contains("func main(")
            || path_contains_component(path, &["cmd", "scripts"]))
    {
        push_unique(paradigms, ProgrammingParadigm::Procedural);
    }

    if matches!(
        language,
        LanguageKind::Java | LanguageKind::Kotlin | LanguageKind::CSharp
    ) && (normalized_content.contains("class ")
        || normalized_content.contains("interface ")
        || normalized_content.contains("record "))
    {
        push_unique(paradigms, ProgrammingParadigm::ObjectOriented);
    }

    if paradigms.len() > 1 {
        push_unique(paradigms, ProgrammingParadigm::Mixed);
    }

    if paradigms.is_empty() {
        push_unique(paradigms, ProgrammingParadigm::Unknown);
    }
}

fn classify_runtimes(
    runtimes: &mut Vec<RuntimeKind>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
) {
    let normalized_content = content.to_lowercase();

    if frameworks.contains(&FrameworkKind::ReactNative) || frameworks.contains(&FrameworkKind::Expo)
    {
        push_unique(runtimes, RuntimeKind::ReactNative);
    } else if frameworks.contains(&FrameworkKind::React)
        || frameworks.contains(&FrameworkKind::NextJs)
        || frameworks.contains(&FrameworkKind::Vue)
        || frameworks.contains(&FrameworkKind::Angular)
        || frameworks.contains(&FrameworkKind::Svelte)
    {
        push_unique(runtimes, RuntimeKind::Browser);
    }

    if frameworks.contains(&FrameworkKind::NodeJs)
        || normalized_content.contains("process.env")
        || normalized_content.contains("from 'node:")
        || normalized_content.contains("from \"node:")
    {
        push_unique(runtimes, RuntimeKind::Node);
    }

    if frameworks.contains(&FrameworkKind::Unity) {
        push_unique(runtimes, RuntimeKind::Unity);
    }

    if frameworks.contains(&FrameworkKind::DotNet) {
        push_unique(runtimes, RuntimeKind::DotNet);
    }

    if language == LanguageKind::Rust {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();

        if file_name == "main.rs" {
            push_unique(runtimes, RuntimeKind::RustCli);
        } else {
            push_unique(runtimes, RuntimeKind::RustLibrary);
        }
    }

    if language == LanguageKind::Python {
        push_unique(runtimes, RuntimeKind::Python);
    }

    if language == LanguageKind::Go {
        push_unique(runtimes, RuntimeKind::Go);
    }

    if matches!(
        language,
        LanguageKind::Java | LanguageKind::Kotlin | LanguageKind::Scala
    ) {
        push_unique(runtimes, RuntimeKind::Jvm);
    }

    if language == LanguageKind::Shell || language == LanguageKind::PowerShell {
        push_unique(runtimes, RuntimeKind::Shell);
    }

    if matches!(
        language,
        LanguageKind::C | LanguageKind::Cpp | LanguageKind::CHeader | LanguageKind::Swift
    ) {
        push_unique(runtimes, RuntimeKind::Native);
    }

    if frameworks.contains(&FrameworkKind::Android) {
        push_unique(runtimes, RuntimeKind::Android);
    }

    if runtimes.is_empty() {
        push_unique(runtimes, RuntimeKind::Unknown);
    }
}

fn is_js_or_ts(language: LanguageKind) -> bool {
    matches!(
        language,
        LanguageKind::TypeScript | LanguageKind::JavaScript
    )
}

fn is_react_native_content(content: &str) -> bool {
    content.contains("react-native")
        || content.contains("@react-native")
        || content.contains("from 'react-native'")
        || content.contains("from \"react-native\"")
}

fn is_react_file(path: &Path, content: &str) -> bool {
    is_tsx_or_jsx_file(path)
        || content.contains("from 'react'")
        || content.contains("from \"react\"")
        || content.contains("react.")
        || content.contains("react.fc")
}

fn is_tsx_or_jsx_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(normalize)
            .as_deref(),
        Some("tsx") | Some("jsx")
    )
}

fn is_react_component_file(path: &Path, content: &str) -> bool {
    if is_tsx_or_jsx_file(path) {
        return true;
    }

    let file_stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    is_pascal_case(file_stem)
        && (content.contains("return <")
            || content.contains("</")
            || content.contains("React.FC")
            || content.contains("React.memo")
            || content.contains("memo(")
            || content.contains("forwardRef(")
            || content.contains("React.createElement"))
}

fn is_react_hook_file(path: &Path, content: &str) -> bool {
    let file_stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    file_stem.starts_with("use")
        && file_stem
            .chars()
            .nth(3)
            .map(|character| character.is_uppercase())
            .unwrap_or(false)
        && (content.contains("useState")
            || content.contains("useEffect")
            || content.contains("useMemo")
            || content.contains("useCallback")
            || content.contains("useReducer")
            || content.contains("function use")
            || content.contains("const use"))
}

fn is_unity_file(path: &Path, content: &str) -> bool {
    content.contains("using unityengine")
        || content.contains("monobehaviour")
        || path_contains_component(path, &["assets"])
}

fn is_dotnet_file(path: &Path, content: &str) -> bool {
    content.contains("using microsoft.aspnetcore")
        || content.contains("webapplication.createbuilder")
        || content.contains("[apicontroller]")
        || path_contains_component(path, &["controllers", "services"])
}

fn is_dotnet_controller(path: &Path, content: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with("Controller.cs"))
        .unwrap_or(false)
        || content.contains("[apicontroller]")
        || content.contains(": controllerbase")
        || content.contains(": controller")
}

fn is_dotnet_service(path: &Path, content: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with("Service.cs"))
        .unwrap_or(false)
        || (content.contains("class ")
            && content.contains("service")
            && path_contains_component(path, &["services"]))
}

fn is_config_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize)
        .unwrap_or_default();

    matches!(
        file_name.as_str(),
        "package.json"
            | "tsconfig.json"
            | "vite.config.ts"
            | "vite.config.js"
            | "next.config.js"
            | "next.config.mjs"
            | "cargo.toml"
            | "cargo.lock"
            | "projectsettings.asset"
            | "dockerfile"
            | "containerfile"
            | "go.mod"
            | "go.sum"
            | "pyproject.toml"
            | "requirements.txt"
            | "build.gradle"
            | "settings.gradle"
            | "pom.xml"
    ) || (file_name.starts_with("appsettings") && file_name.ends_with(".json"))
}

fn is_app_entrypoint(path: &Path, content: &str, language: LanguageKind) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(normalize)
        .unwrap_or_default();
    let normalized_content = content.to_lowercase();

    matches!(
        file_name.as_str(),
        "main.rs"
            | "main.go"
            | "main.py"
            | "app.py"
            | "program.cs"
            | "main.java"
            | "main.kt"
            | "index.ts"
            | "index.js"
            | "main.ts"
            | "main.js"
    ) || (language == LanguageKind::Python
        && normalized_content.contains("if __name__ == \"__main__\""))
        || (language == LanguageKind::Go && normalized_content.contains("func main("))
        || (language == LanguageKind::Rust && normalized_content.contains("fn main("))
}

fn is_generated_file(path: &Path, content: &str) -> bool {
    path_contains_component(
        path,
        &[
            "generated",
            "__generated__",
            "gen",
            "codegen",
            "target",
            "build",
        ],
    ) || content.contains("@generated")
        || content.contains("code generated")
        || content.contains("generated by")
}

fn is_test_file(path: &Path, has_inline_tests: bool) -> bool {
    if has_inline_tests {
        return true;
    }

    let path_text = path.to_string_lossy().to_lowercase();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase())
        .unwrap_or_default();

    path_text.starts_with("tests/")
        || path_text.contains("/tests/")
        || path_text.contains("\\tests\\")
        || path_text.contains("/__tests__/")
        || path_text.contains("\\__tests__\\")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".test.tsx")
        || file_name.ends_with(".test.js")
        || file_name.ends_with(".test.jsx")
        || file_name.ends_with(".spec.ts")
        || file_name.ends_with(".spec.tsx")
        || file_name.ends_with(".spec.js")
        || file_name.ends_with(".spec.jsx")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with("_test.go")
        || file_name.ends_with("_test.py")
        || file_name.starts_with("test_")
        || file_name.ends_with("test.java")
        || file_name.ends_with("tests.java")
        || file_name.ends_with("test.kt")
        || file_name.ends_with("tests.kt")
        || file_name.ends_with("test.cs")
        || file_name.ends_with("tests.cs")
}

fn path_contains_component(path: &Path, targets: &[&str]) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|value| {
                let normalized = normalize(value);
                targets.iter().any(|target| normalized == *target)
            })
            .unwrap_or(false)
    })
}

fn is_pascal_case(value: &str) -> bool {
    value
        .chars()
        .next()
        .map(|character| character.is_uppercase())
        .unwrap_or(false)
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn classifies_react_tsx_component() {
        let file = facts(
            "src/components/ProfileCard.tsx",
            Some("TypeScript"),
            "import React from 'react';\nexport function ProfileCard() { return <View />; }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::TypeScript);
        assert!(context.has_framework(FrameworkKind::React));
        assert!(context.has_role(FileRole::ReactComponent));
        assert!(context.has_paradigm(ProgrammingParadigm::DeclarativeUi));
        assert!(context.has_paradigm(ProgrammingParadigm::Functional));
        assert!(context.has_runtime(RuntimeKind::Browser));
        assert!(!context.is_test);
    }

    #[test]
    fn classifies_react_hook() {
        let file = facts(
            "src/hooks/useProfile.ts",
            Some("TypeScript"),
            "import { useEffect, useState } from 'react';\nexport function useProfile() { useEffect(() => {}, []); }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::TypeScript);
        assert!(context.has_framework(FrameworkKind::React));
        assert!(context.has_role(FileRole::ReactHook));
        assert!(context.has_paradigm(ProgrammingParadigm::Functional));
        assert!(context.has_paradigm(ProgrammingParadigm::Reactive));
        assert!(context.has_runtime(RuntimeKind::Browser));
    }

    #[test]
    fn classifies_unity_monobehaviour() {
        let file = facts(
            "Assets/Scripts/PlayerController.cs",
            Some("CSharp"),
            "using UnityEngine;\npublic class PlayerController : MonoBehaviour { void Update() {} }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::CSharp);
        assert!(context.has_framework(FrameworkKind::Unity));
        assert!(context.has_role(FileRole::UnityMonoBehaviour));
        assert!(context.has_paradigm(ProgrammingParadigm::ObjectOriented));
        assert!(context.has_paradigm(ProgrammingParadigm::DataOriented));
        assert!(context.has_runtime(RuntimeKind::Unity));
    }

    #[test]
    fn classifies_dotnet_controller() {
        let file = facts(
            "src/Controllers/UsersController.cs",
            Some("CSharp"),
            "using Microsoft.AspNetCore.Mvc;\n[ApiController]\npublic class UsersController : ControllerBase {}\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::CSharp);
        assert!(context.has_framework(FrameworkKind::DotNet));
        assert!(context.has_role(FileRole::DotNetController));
        assert!(context.has_paradigm(ProgrammingParadigm::ObjectOriented));
        assert!(context.has_runtime(RuntimeKind::DotNet));
    }

    #[test]
    fn classifies_dotnet_service() {
        let file = facts(
            "src/Services/UserService.cs",
            Some("CSharp"),
            "public class UserService { public Task Run() => Task.CompletedTask; }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::CSharp);
        assert!(context.has_framework(FrameworkKind::DotNet));
        assert!(context.has_role(FileRole::DotNetService));
        assert!(context.has_runtime(RuntimeKind::DotNet));
    }

    #[test]
    fn classifies_rust_inline_test_file() {
        let file = facts(
            "src/domain/user.rs",
            Some("Rust"),
            "#[cfg(test)]\nmod tests { #[test] fn works() {} }\n",
            true,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Rust);
        assert!(context.has_role(FileRole::RustTest));
        assert!(context.has_role(FileRole::Test));
        assert!(context.has_runtime(RuntimeKind::RustLibrary));
        assert!(context.is_test);
    }

    #[test]
    fn classifies_rust_main_as_cli_runtime() {
        let file = facts(
            "src/main.rs",
            Some("Rust"),
            "fn main() { println!(\"hello\"); }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Rust);
        assert!(context.has_runtime(RuntimeKind::RustCli));
        assert!(context.has_paradigm(ProgrammingParadigm::Procedural));
    }

    #[test]
    fn classifies_rust_lib_as_library_runtime() {
        let file = facts("src/lib.rs", Some("Rust"), "pub fn parse() {}\n", false);

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Rust);
        assert!(context.has_runtime(RuntimeKind::RustLibrary));
        assert!(!context.is_test);
    }

    #[test]
    fn classifies_rust_domain_file_role() {
        let file = facts(
            "src/domain/user.rs",
            Some("Rust"),
            "pub struct User { id: String }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Rust);
        assert!(context.has_role(FileRole::Domain));
        assert!(context.has_runtime(RuntimeKind::RustLibrary));
    }

    #[test]
    fn classifies_rust_test_path() {
        let file = facts(
            "tests/parser_test.rs",
            Some("Rust"),
            "#[test]\nfn parses() {}\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Rust);
        assert!(context.has_role(FileRole::Test));
        assert!(context.has_role(FileRole::RustTest));
        assert!(context.is_test);
    }

    #[test]
    fn classifies_rust_iterator_pipeline_as_functional_without_marking_it_bad() {
        let file = facts(
            "src/domain/users.rs",
            Some("Rust"),
            "let names = users.iter().filter(|user| user.is_active).map(|user| user.name.clone()).collect::<Vec<_>>();\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Rust);
        assert!(context.is_functional_code());
        assert!(!context.has_role(FileRole::Config));
    }

    #[test]
    fn classifies_node_runtime_from_process_env_and_node_imports() {
        for content in [
            "const value = process.env.NODE_ENV;\n",
            "import fs from \"node:fs\";\n",
            "import path from 'node:path';\n",
        ] {
            let file = facts("src/server.ts", Some("TypeScript"), content, false);

            let context = classify_file(&file);

            assert!(context.has_framework(FrameworkKind::NodeJs));
            assert!(context.has_runtime(RuntimeKind::Node));
        }
    }

    #[test]
    fn classifies_python_go_and_jvm_contexts() {
        let python = classify_file(&facts(
            "app/views.py",
            Some("Python"),
            "from fastapi import FastAPI\napp = FastAPI()\n",
            false,
        ));
        assert_eq!(python.language, LanguageKind::Python);
        assert!(python.has_framework(FrameworkKind::FastApi));
        assert!(python.has_runtime(RuntimeKind::Python));

        let go = classify_file(&facts(
            "cmd/server/main.go",
            Some("Go"),
            "package main\nimport \"github.com/gin-gonic/gin\"\nfunc main() {}\n",
            false,
        ));
        assert_eq!(go.language, LanguageKind::Go);
        assert!(go.has_framework(FrameworkKind::Gin));
        assert!(go.has_runtime(RuntimeKind::Go));
        assert!(go.has_role(FileRole::Script));

        let java = classify_file(&facts(
            "src/main/java/com/acme/UserService.java",
            Some("Java"),
            "import org.springframework.stereotype.Service;\npublic class UserService {}\n",
            false,
        ));
        assert_eq!(java.language, LanguageKind::Java);
        assert!(java.has_framework(FrameworkKind::Spring));
        assert!(java.has_paradigm(ProgrammingParadigm::ObjectOriented));
        assert!(java.has_runtime(RuntimeKind::Jvm));
    }

    #[test]
    fn classifies_generated_files_as_non_production() {
        let file = facts(
            "src/generated/schema.rs",
            Some("Rust"),
            "// generated by schema tool\npub fn value() {}\n",
            false,
        );

        let context = classify_file(&file);

        assert!(context.has_role(FileRole::Generated));
        assert!(!context.is_production_code());
    }

    #[test]
    fn classifies_config_file() {
        let file = facts(
            "tsconfig.json",
            None,
            "{ \"compilerOptions\": {} }\n",
            false,
        );

        let context = classify_file(&file);

        assert!(context.has_role(FileRole::Config));
        assert!(context.has_runtime(RuntimeKind::Unknown));
        assert!(!context.is_production_code());
    }

    fn facts(
        path: &str,
        language: Option<&str>,
        content: &str,
        has_inline_tests: bool,
    ) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            language: language.map(str::to_string),
            lines_of_code: content.lines().count(),
            branch_count: 0,
            imports: Vec::new(),
            content: Some(content.to_string()),
            has_inline_tests,
        }
    }
}
