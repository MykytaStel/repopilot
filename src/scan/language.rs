use std::path::Path;

pub fn detect_language(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?;

    match extension {
        "rs" => Some("Rust"),
        "js" => Some("JavaScript"),
        "jsx" => Some("JavaScript React"),
        "ts" => Some("TypeScript"),
        "tsx" => Some("TypeScript React"),
        "py" => Some("Python"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "kt" => Some("Kotlin"),
        "swift" => Some("Swift"),
        "cs" => Some("C#"),
        "cpp" | "cc" | "cxx" => Some("C++"),
        "c" => Some("C"),
        "h" | "hpp" => Some("C/C++ Header"),
        "html" => Some("HTML"),
        "css" => Some("CSS"),
        "scss" => Some("SCSS"),
        "json" => Some("JSON"),
        "toml" => Some("TOML"),
        "yaml" | "yml" => Some("YAML"),
        "md" => Some("Markdown"),
        _ => None,
    }
}
