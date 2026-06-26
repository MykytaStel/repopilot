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
fn classifies_gradle_testing_module_as_test_support() {
    // `core/testing/` holds test doubles under a production source set (`src/main`),
    // so it is not a test file but earns the TestSupport marker for opted-in rules.
    let file = facts(
        "core/testing/src/main/kotlin/com/app/core/testing/util/TestSyncManager.kt",
        Some("Kotlin"),
        "class TestSyncManager : SyncManager {\n  override fun requestSync(): Unit = TODO()\n}\n",
        false,
    );

    let context = classify_file(&file);

    assert!(context.has_role(FileRole::TestSupport));
    assert!(
        !context.has_role(FileRole::Test),
        "a src/main module is not a test file"
    );
    assert!(!context.has_role(FileRole::BuildTooling));
}

#[test]
fn classifies_build_logic_as_build_tooling() {
    let file = facts(
        "build-logic/convention/src/main/kotlin/com/app/KotlinAndroid.kt",
        Some("Kotlin"),
        "internal fun configure() {\n  when (x) { else -> TODO(\"Unsupported\") }\n}\n",
        false,
    );

    let context = classify_file(&file);

    assert!(context.has_role(FileRole::BuildTooling));
    assert!(!context.has_role(FileRole::TestSupport));
}

#[test]
fn classifies_feature_screen_as_plain_production_code() {
    // Real app UI carries neither marker, so its `TODO()` stays default-visible.
    let file = facts(
        "feature/topic/impl/src/main/kotlin/com/app/feature/topic/TopicScreen.kt",
        Some("Kotlin"),
        "fun TopicScreen(state: TopicUiState) {\n  when (state) { is Error -> TODO() }\n}\n",
        false,
    );

    let context = classify_file(&file);

    assert!(!context.has_role(FileRole::TestSupport));
    assert!(!context.has_role(FileRole::BuildTooling));
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
