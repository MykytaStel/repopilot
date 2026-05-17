mod frameworks;
mod helpers;
mod paradigms;
mod roles;
mod runtimes;
mod signals;

use crate::audits::context::model::{AuditContext, FileRole, LanguageKind};
use crate::knowledge::language::language_kind_for_file;
use crate::scan::facts::FileFacts;
use helpers::is_test_file;
use signals::ContextSignals;

pub fn classify_file(file: &FileFacts) -> AuditContext {
    let content = file.content.as_deref().unwrap_or("");
    let language = classify_language(file);
    let is_test = is_test_file(&file.path, file.has_inline_tests);
    let signals = ContextSignals::detect(&file.path, language, content);

    let mut frameworks = Vec::new();
    frameworks::classify_frameworks(&mut frameworks, &file.path, content, language);

    let mut roles = Vec::new();
    roles::classify_roles(
        &mut roles,
        &file.path,
        content,
        language,
        &frameworks,
        &signals,
        is_test,
    );

    if roles.is_empty() {
        roles.push(FileRole::Unknown);
    }

    let mut paradigms = Vec::new();
    paradigms::classify_paradigms(
        &mut paradigms,
        &file.path,
        content,
        language,
        &frameworks,
        &roles,
        &signals,
    );

    let mut runtimes = Vec::new();
    runtimes::classify_runtimes(
        &mut runtimes,
        &file.path,
        content,
        language,
        &frameworks,
        &signals,
    );

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audits::context::model::{
        FileRole, FrameworkKind, LanguageKind, ProgrammingParadigm, RuntimeKind,
    };
    use crate::scan::facts::FileFacts;
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

    #[test]
    fn classifies_functional_language_context() {
        let file = facts(
            "src/Pipeline.hs",
            Some("Haskell"),
            "module Pipeline where\nrun xs = map (+1) xs\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Haskell);
        assert!(context.has_paradigm(ProgrammingParadigm::Functional));
        assert!(context.is_functional_code());
    }

    #[test]
    fn classifies_infrastructure_files_as_declarative_context() {
        let file = facts(
            "infra/terraform/main.tf",
            Some("Terraform"),
            "resource \"aws_s3_bucket\" \"assets\" {}\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Terraform);
        assert!(context.has_role(FileRole::Infrastructure));
        assert!(context.has_runtime(RuntimeKind::Infrastructure));
        assert!(context.has_paradigm(ProgrammingParadigm::Declarative));
        assert!(context.is_declarative_code());
        assert!(context.is_infrastructure_code());
        assert!(!context.is_production_code());
    }

    #[test]
    fn classifies_ci_workflow_yaml_as_infrastructure_context() {
        let file = facts(
            ".github/workflows/ci.yml",
            Some("YAML"),
            "name: CI\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Yaml);
        assert!(context.has_role(FileRole::Infrastructure));
        assert!(context.has_runtime(RuntimeKind::Infrastructure));
        assert!(context.has_paradigm(ProgrammingParadigm::Declarative));
        assert!(context.is_infrastructure_code());
        assert!(!context.is_production_code());
    }

    #[test]
    fn classifies_docker_compose_file_as_infrastructure_context() {
        let file = facts(
            "docker-compose.yml",
            Some("YAML"),
            "services:\n  api:\n    image: repopilot/api:latest\n",
            false,
        );

        let context = classify_file(&file);

        assert!(context.has_role(FileRole::Infrastructure));
        assert!(context.has_runtime(RuntimeKind::Infrastructure));
        assert!(context.has_paradigm(ProgrammingParadigm::Declarative));
    }

    #[test]
    fn classifies_json_as_declarative_but_not_infrastructure_by_default() {
        let file = facts(
            "fixtures/report.json",
            Some("JSON"),
            "{ \"schema_version\": \"0.12\" }\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Json);
        assert!(context.has_paradigm(ProgrammingParadigm::Declarative));
        assert!(!context.has_role(FileRole::Infrastructure));
        assert!(!context.has_runtime(RuntimeKind::Infrastructure));
    }

    #[test]
    fn classifies_generic_yaml_toml_as_declarative_but_not_infrastructure_by_default() {
        for (path, language, content, expected) in [
            (
                "config/settings.yml",
                "YAML",
                "name: repopilot\n",
                LanguageKind::Yaml,
            ),
            (
                "config/settings.toml",
                "TOML",
                "name = \"repopilot\"\n",
                LanguageKind::Toml,
            ),
        ] {
            let context = classify_file(&facts(path, Some(language), content, false));

            assert_eq!(context.language, expected);
            assert!(context.has_paradigm(ProgrammingParadigm::Declarative));
            assert!(!context.has_role(FileRole::Infrastructure));
            assert!(!context.has_runtime(RuntimeKind::Infrastructure));
        }
    }

    #[test]
    fn classifies_helm_path_yaml_as_infrastructure_context() {
        let file = facts(
            "deploy/helm/values.yaml",
            Some("YAML"),
            "image:\n  repository: repopilot/api\n",
            false,
        );

        let context = classify_file(&file);

        assert!(context.has_role(FileRole::Infrastructure));
        assert!(context.has_runtime(RuntimeKind::Infrastructure));
        assert!(context.has_paradigm(ProgrammingParadigm::Declarative));
    }

    #[test]
    fn classifies_elixir_as_functional_first_context() {
        let file = facts(
            "lib/pipeline.ex",
            Some("Elixir"),
            "defmodule Pipeline do\n  def run(items), do: Enum.map(items, & &1.id)\nend\n",
            false,
        );

        let context = classify_file(&file);

        assert_eq!(context.language, LanguageKind::Elixir);
        assert!(context.has_paradigm(ProgrammingParadigm::Functional));
        assert!(context.is_functional_code());
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
