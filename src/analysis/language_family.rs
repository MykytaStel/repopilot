use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LanguageFamily {
    CurlyBrace, // Rust, TypeScript, JavaScript, Java, Kotlin, C#, C, C++, Swift
    Python,
    Go,
    Shell,
    Markup, // HTML, CSS, SCSS, Markdown, JSON, TOML, YAML
    Unknown,
}
