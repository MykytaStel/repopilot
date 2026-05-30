use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::BTreeMap;
use std::path::Path;

use crate::audits::context::LanguageKind;
use crate::audits::context::classify::helpers::{
    is_app_entrypoint, is_config_file, is_generated_file, is_test_file, path_contains_component,
};
use crate::knowledge::language::language_kind_for_file;
use crate::scan::config::ScanConfig;
use crate::scan::facts::FileFacts;

use super::model::{ArchitectureContext, FileRole, LanguageFamily, ModuleKind};

pub struct ArchitectureClassifier {
    mappings: Vec<(ModuleKind, GlobSet)>,
}

impl ArchitectureClassifier {
    pub fn new(module_mappings: &BTreeMap<String, Vec<String>>) -> Self {
        let mut mappings = Vec::new();

        let order = [
            ("ui", ModuleKind::Ui),
            ("infrastructure", ModuleKind::Infrastructure),
            ("domain", ModuleKind::Domain),
            ("cli", ModuleKind::Cli),
            ("shared", ModuleKind::Shared),
            ("feature", ModuleKind::Feature),
        ];

        for &(kind_str, kind) in &order {
            if let Some(globs) = module_mappings.get(kind_str) {
                let mut builder = GlobSetBuilder::new();
                for glob_pattern in globs {
                    if let Ok(glob) = Glob::new(glob_pattern) {
                        builder.add(glob);
                    }
                }

                if let Ok(glob_set) = builder.build() {
                    mappings.push((kind, glob_set));
                }
            }
        }

        Self { mappings }
    }

    pub fn classify(&self, file: &FileFacts) -> ArchitectureContext {
        let content = file.content.as_deref().unwrap_or("");
        let language_kind = language_kind_for_file(file);
        let language_family = LanguageFamily::from_language_kind(language_kind);

        // 1. Determine FileRole
        let file_role = if is_config_file(&file.path) {
            FileRole::Config
        } else if is_generated_file(&file.path, content) {
            FileRole::Generated
        } else if path_contains_component(
            &file.path,
            &[
                "__fixtures__",
                "__mocks__",
                "__snapshots__",
                "fixture",
                "fixtures",
                "mock",
                "mocks",
                "snapshot",
                "snapshots",
            ],
        ) {
            FileRole::Fixture
        } else if path_contains_component(&file.path, &["doc", "docs"]) {
            FileRole::Documentation
        } else if is_test_file(&file.path, file.has_inline_tests) {
            FileRole::Test
        } else {
            FileRole::Production
        };

        // 2. Determine ModuleKind using glob matching
        let mut module_kind = ModuleKind::Unknown;
        for (kind, glob_set) in &self.mappings {
            if glob_set.is_match(&file.path) {
                module_kind = *kind;
                break;
            }
        }

        // 3. Determine is_entrypoint
        let is_entrypoint = is_app_entrypoint(&file.path, content, language_kind);

        // 4. Determine is_public_api
        let is_public_api = is_public_api_file(&file.path);

        ArchitectureContext {
            file_role,
            module_kind,
            language_family,
            is_entrypoint,
            is_public_api,
        }
    }
}

impl LanguageFamily {
    pub fn from_language_kind(kind: LanguageKind) -> Self {
        match kind {
            LanguageKind::Rust
            | LanguageKind::TypeScript
            | LanguageKind::JavaScript
            | LanguageKind::CSharp
            | LanguageKind::Java
            | LanguageKind::Kotlin
            | LanguageKind::Swift
            | LanguageKind::C
            | LanguageKind::Cpp
            | LanguageKind::CHeader
            | LanguageKind::ObjectiveC
            | LanguageKind::Scala
            | LanguageKind::Dart
            | LanguageKind::Php => LanguageFamily::CurlyBrace,

            LanguageKind::Python => LanguageFamily::Python,
            LanguageKind::Go => LanguageFamily::Go,

            LanguageKind::Shell | LanguageKind::PowerShell => LanguageFamily::Shell,

            LanguageKind::Html
            | LanguageKind::Css
            | LanguageKind::Scss
            | LanguageKind::Json
            | LanguageKind::Toml
            | LanguageKind::Yaml
            | LanguageKind::Markdown
            | LanguageKind::Terraform
            | LanguageKind::Dockerfile
            | LanguageKind::Nix => LanguageFamily::Markup,

            _ => LanguageFamily::Unknown,
        }
    }
}

fn is_public_api_file(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    file_name.eq_ignore_ascii_case("index.ts")
        || file_name.eq_ignore_ascii_case("index.js")
        || file_name.eq_ignore_ascii_case("index.tsx")
        || file_name.eq_ignore_ascii_case("index.jsx")
        || file_name.eq_ignore_ascii_case("mod.rs")
        || file_name.eq_ignore_ascii_case("lib.rs")
}

pub fn classify_file_architecture(file: &FileFacts, config: &ScanConfig) -> ArchitectureContext {
    let classifier = ArchitectureClassifier::new(&config.module_mappings);
    classifier.classify(file)
}
