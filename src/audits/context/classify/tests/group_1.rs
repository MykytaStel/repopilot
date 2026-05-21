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
