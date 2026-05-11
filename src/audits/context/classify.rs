use crate::audits::context::model::{
    AuditContext, FileRole, FrameworkKind, LanguageKind, ProgrammingParadigm, RuntimeKind,
};
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
    if let Some(language) = &file.language {
        let normalized = normalize(language);

        match normalized.as_str() {
            "rust" => return LanguageKind::Rust,
            "typescript" | "typescript react" | "tsx" => return LanguageKind::TypeScript,
            "javascript" | "javascript react" | "jsx" => return LanguageKind::JavaScript,
            "csharp" | "c#" | "cs" | "c sharp" => return LanguageKind::CSharp,
            "python" => return LanguageKind::Python,
            "go" => return LanguageKind::Go,
            _ => {}
        }
    }

    match file
        .path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(normalize)
        .as_deref()
    {
        Some("rs") => LanguageKind::Rust,
        Some("ts") | Some("tsx") | Some("mts") => LanguageKind::TypeScript,
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => LanguageKind::JavaScript,
        Some("cs") => LanguageKind::CSharp,
        Some("py") => LanguageKind::Python,
        Some("go") => LanguageKind::Go,
        _ => LanguageKind::Unknown,
    }
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

        if is_react_file(path, &normalized_content) {
            push_unique(frameworks, FrameworkKind::React);
        }

        if normalized_content.contains("next/")
            || (path_contains_component(path, &["pages", "app"]) && is_tsx_or_jsx_file(path))
        {
            push_unique(frameworks, FrameworkKind::NextJs);
        }

        if normalized_content.contains("express")
            || normalized_content.contains("from 'node:")
            || normalized_content.contains("from \"node:")
            || normalized_content.contains("process.env")
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

    if is_test {
        push_unique(roles, FileRole::Test);

        if language == LanguageKind::Rust {
            push_unique(roles, FileRole::RustTest);
        }
    }

    if is_js_or_ts(language) {
        if is_react_hook_file(path, content) {
            push_unique(roles, FileRole::ReactHook);
        }

        if frameworks.contains(&FrameworkKind::React) && is_react_component_file(path, content) {
            push_unique(roles, FileRole::ReactComponent);
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
        }

        if is_dotnet_service(path, &normalized_content) {
            push_unique(roles, FileRole::DotNetService);
        }
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

    if frameworks.contains(&FrameworkKind::ReactNative) {
        push_unique(runtimes, RuntimeKind::ReactNative);
    } else if frameworks.contains(&FrameworkKind::React)
        || frameworks.contains(&FrameworkKind::NextJs)
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
    ) || (file_name.starts_with("appsettings") && file_name.ends_with(".json"))
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
