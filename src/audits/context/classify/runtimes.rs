use super::helpers::push_unique;
use crate::audits::context::model::{FrameworkKind, LanguageKind, RuntimeKind};
use std::path::Path;

pub fn classify_runtimes(
    runtimes: &mut Vec<RuntimeKind>,
    path: &Path,
    content: &str,
    language: LanguageKind,
    frameworks: &[FrameworkKind],
) {
    // All patterns are lowercase in their respective source languages — no allocation needed.
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
        || content.contains("process.env")
        || content.contains("from 'node:")
        || content.contains("from \"node:")
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
