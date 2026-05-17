use super::helpers::{is_js_or_ts, normalize, path_contains_component, push_unique};
use crate::audits::context::model::{FrameworkKind, LanguageKind};
use std::path::Path;

pub fn classify_frameworks(
    frameworks: &mut Vec<FrameworkKind>,
    path: &Path,
    content: &str,
    language: LanguageKind,
) {
    // JS/TS, Python, Go, Dart: all patterns are naturally lowercase in source — no allocation.
    // C# and JVM: use PascalCase/camelCase identifiers that need case-folding.
    if is_js_or_ts(language) {
        classify_js_ts_frameworks(frameworks, path, content);
    }

    if language == LanguageKind::CSharp {
        classify_csharp_frameworks(frameworks, path, content);
    }

    if language == LanguageKind::Python {
        classify_python_frameworks(frameworks, content);
    }

    if language == LanguageKind::Go {
        classify_go_frameworks(frameworks, content);
    }

    if matches!(language, LanguageKind::Java | LanguageKind::Kotlin) {
        classify_jvm_frameworks(frameworks, path, content);
    }

    if language == LanguageKind::Dart
        && (content.contains("package:flutter")
            || path_contains_component(path, &["lib", "widgets"]))
    {
        push_unique(frameworks, FrameworkKind::Flutter);
    }
}

fn classify_js_ts_frameworks(frameworks: &mut Vec<FrameworkKind>, path: &Path, content: &str) {
    if is_react_native_content(content) {
        push_unique(frameworks, FrameworkKind::ReactNative);
    }

    if content.contains("from 'expo")
        || content.contains("from \"expo")
        || content.contains("expo-status-bar")
        || content.contains("expo-router")
    {
        push_unique(frameworks, FrameworkKind::Expo);
    }

    if is_react_file(path, content) {
        push_unique(frameworks, FrameworkKind::React);
    }

    if content.contains("next/")
        || (path_contains_component(path, &["pages", "app"]) && is_tsx_or_jsx_file(path))
    {
        push_unique(frameworks, FrameworkKind::NextJs);
    }

    classify_js_ui_frameworks(frameworks, content);
    classify_node_frameworks(frameworks, content);
}

fn classify_js_ui_frameworks(frameworks: &mut Vec<FrameworkKind>, content: &str) {
    if content.contains("from 'vue'")
        || content.contains("from \"vue\"")
        || content.contains("@vue/")
    {
        push_unique(frameworks, FrameworkKind::Vue);
    }

    if content.contains("@angular/") {
        push_unique(frameworks, FrameworkKind::Angular);
    }

    if content.contains("from 'svelte") || content.contains("from \"svelte") {
        push_unique(frameworks, FrameworkKind::Svelte);
    }
}

fn classify_node_frameworks(frameworks: &mut Vec<FrameworkKind>, content: &str) {
    if content.contains("@nestjs/") {
        push_unique(frameworks, FrameworkKind::NestJs);
    }

    if content.contains("express") {
        push_unique(frameworks, FrameworkKind::Express);
    }

    if content.contains("express")
        || content.contains("from 'node:")
        || content.contains("from \"node:")
        || content.contains("process.env")
        || content.contains("process.exit")
    {
        push_unique(frameworks, FrameworkKind::NodeJs);
    }
}

fn classify_csharp_frameworks(frameworks: &mut Vec<FrameworkKind>, path: &Path, content: &str) {
    // Compute lowercase only for C# where PascalCase identifiers need folding.
    let lower = content.to_lowercase();
    if is_unity_file(path, &lower) {
        push_unique(frameworks, FrameworkKind::Unity);
    }
    if is_dotnet_file(path, &lower) {
        push_unique(frameworks, FrameworkKind::DotNet);
    }
}

fn classify_python_frameworks(frameworks: &mut Vec<FrameworkKind>, content: &str) {
    if content.contains("django") {
        push_unique(frameworks, FrameworkKind::Django);
    }
    if content.contains("flask") {
        push_unique(frameworks, FrameworkKind::Flask);
    }
    if content.contains("fastapi") {
        push_unique(frameworks, FrameworkKind::FastApi);
    }
}

fn classify_go_frameworks(frameworks: &mut Vec<FrameworkKind>, content: &str) {
    if content.contains("github.com/gin-gonic/gin") {
        push_unique(frameworks, FrameworkKind::Gin);
    }
    if content.contains("github.com/labstack/echo") {
        push_unique(frameworks, FrameworkKind::Echo);
    }
    if content.contains("github.com/gofiber/fiber") {
        push_unique(frameworks, FrameworkKind::Fiber);
    }
}

fn classify_jvm_frameworks(frameworks: &mut Vec<FrameworkKind>, path: &Path, content: &str) {
    // Compute lowercase only for JVM where annotations use mixed case.
    let lower = content.to_lowercase();
    if lower.contains("org.springframework") || lower.contains("@springbootapplication") {
        push_unique(frameworks, FrameworkKind::Spring);
    }
    if lower.contains("android.")
        || lower.contains("androidx.")
        || path_contains_component(path, &["android"])
    {
        push_unique(frameworks, FrameworkKind::Android);
    }
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
        || content.contains("React.")
        || content.contains("React.FC")
}

pub fn is_tsx_or_jsx_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(normalize)
            .as_deref(),
        Some("tsx") | Some("jsx")
    )
}

pub fn is_react_component_file(path: &Path, content: &str) -> bool {
    use super::helpers::is_pascal_case;

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

pub fn is_react_hook_file(path: &Path, content: &str) -> bool {
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

pub fn is_dotnet_controller(path: &Path, content: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with("Controller.cs"))
        .unwrap_or(false)
        || content.contains("[apicontroller]")
        || content.contains(": controllerbase")
        || content.contains(": controller")
}

pub fn is_dotnet_service(path: &Path, content: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with("Service.cs"))
        .unwrap_or(false)
        || (content.contains("class ")
            && content.contains("service")
            && path_contains_component(path, &["services"]))
}
