use crate::audits::context::LanguageKind;
use crate::knowledge::active_knowledge;
use crate::knowledge::model::LanguageProfile;
use crate::scan::facts::FileFacts;
use std::path::Path;

pub fn detect_language_for_path(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name().and_then(|name| name.to_str())?;
    let extension = path.extension().and_then(|extension| extension.to_str());

    active_knowledge().languages.iter().find_map(|language| {
        let file_name_matches = language
            .filenames
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(file_name));
        let extension_matches = extension.is_some_and(|extension| {
            language
                .extensions
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        });

        (file_name_matches || extension_matches).then_some(language.name.as_str())
    })
}

pub fn profile_by_id(id: &str) -> Option<&'static LanguageProfile> {
    active_knowledge()
        .languages
        .iter()
        .find(|language| language.id == id)
}

pub fn language_id_for_name(name: &str) -> Option<&'static str> {
    let normalized = normalize(name);
    active_knowledge()
        .languages
        .iter()
        .find(|language| {
            normalize(&language.name) == normalized
                || language
                    .aliases
                    .iter()
                    .any(|alias| normalize(alias) == normalized)
                || normalize(&language.id) == normalized
        })
        .map(|language| language.id.as_str())
}

pub fn language_kind_for_file(file: &FileFacts) -> LanguageKind {
    if let Some(language) = &file.language
        && let Some(kind) = language_kind_from_name(language)
    {
        return kind;
    }

    detect_language_for_path(&file.path)
        .and_then(language_kind_from_name)
        .unwrap_or(LanguageKind::Unknown)
}

pub fn language_kind_from_name(name: &str) -> Option<LanguageKind> {
    let profile = language_id_for_name(name).and_then(profile_by_id)?;

    Some(language_kind_from_id(&profile.id))
}

pub fn language_kind_from_id(id: &str) -> LanguageKind {
    match id {
        "rust" => LanguageKind::Rust,
        "typescript" | "typescript-react" => LanguageKind::TypeScript,
        "javascript" | "javascript-react" => LanguageKind::JavaScript,
        "csharp" => LanguageKind::CSharp,
        "python" => LanguageKind::Python,
        "go" => LanguageKind::Go,
        "java" => LanguageKind::Java,
        "kotlin" => LanguageKind::Kotlin,
        "swift" => LanguageKind::Swift,
        "c" => LanguageKind::C,
        "cpp" => LanguageKind::Cpp,
        "c-header" => LanguageKind::CHeader,
        "php" => LanguageKind::Php,
        "ruby" => LanguageKind::Ruby,
        "dart" => LanguageKind::Dart,
        "scala" => LanguageKind::Scala,
        "shell" => LanguageKind::Shell,
        "powershell" => LanguageKind::PowerShell,
        "sql" => LanguageKind::Sql,
        "html" => LanguageKind::Html,
        "css" => LanguageKind::Css,
        "scss" => LanguageKind::Scss,
        "elixir" => LanguageKind::Elixir,
        "erlang" => LanguageKind::Erlang,
        "haskell" => LanguageKind::Haskell,
        "ocaml" => LanguageKind::OCaml,
        "fsharp" => LanguageKind::FSharp,
        "r" => LanguageKind::R,
        "julia" => LanguageKind::Julia,
        "lua" => LanguageKind::Lua,
        "perl" => LanguageKind::Perl,
        "zig" => LanguageKind::Zig,
        "solidity" => LanguageKind::Solidity,
        "objective-c" => LanguageKind::ObjectiveC,
        "terraform" => LanguageKind::Terraform,
        "dockerfile" => LanguageKind::Dockerfile,
        "nix" => LanguageKind::Nix,
        "json" => LanguageKind::Json,
        "toml" => LanguageKind::Toml,
        "yaml" => LanguageKind::Yaml,
        "markdown" => LanguageKind::Markdown,
        _ => LanguageKind::Unknown,
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::model::SupportLevel;

    #[test]
    fn detects_language_from_bundled_extensions() {
        assert_eq!(
            detect_language_for_path(Path::new("src/main.rs")),
            Some("Rust")
        );
        assert_eq!(
            detect_language_for_path(Path::new("Dockerfile")),
            Some("Dockerfile")
        );
        assert_eq!(
            detect_language_for_path(Path::new("infra/main.tf")),
            Some("Terraform")
        );
    }

    #[test]
    fn exposes_support_levels() {
        assert_eq!(
            profile_by_id("rust").map(|profile| profile.support),
            Some(SupportLevel::RuleAware)
        );
        assert_eq!(
            profile_by_id("zig").map(|profile| profile.support),
            Some(SupportLevel::DetectOnly)
        );
    }
}
