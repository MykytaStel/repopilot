//! Architecture classification model: the per-file role, module kind, and
//! language family, plus the combined [`ArchitectureContext`] that the
//! classifier produces for each file.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FileRole {
    Production,
    Test,
    Generated,
    Config,
    Documentation,
    Fixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModuleKind {
    Feature,
    Shared,
    Infrastructure,
    Domain,
    Ui,
    Cli,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LanguageFamily {
    CurlyBrace, // Rust, TypeScript, JavaScript, Java, Kotlin, C#, C, C++, Swift
    Python,
    Go,
    Shell,
    Markup, // HTML, CSS, SCSS, Markdown, JSON, TOML, YAML
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchitectureContext {
    pub file_role: FileRole,
    pub module_kind: ModuleKind,
    pub language_family: LanguageFamily,
    pub is_entrypoint: bool,
    pub is_public_api: bool,
}
