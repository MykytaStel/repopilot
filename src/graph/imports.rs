use std::collections::HashSet;

mod common;
mod go;
mod jvm;
mod python;
mod rust;
mod ts;

/// Extracts raw import strings from file content based on language.
/// Returns a deduplicated list.
pub fn extract_imports(content: &str, language: Option<&str>) -> Vec<String> {
    let set: HashSet<String> = match language {
        Some("Rust") => rust::extract(content),
        Some("TypeScript")
        | Some("TypeScript React")
        | Some("JavaScript")
        | Some("JavaScript React") => ts::extract(content, language),
        Some("Python") => python::extract(content),
        Some("Go") => go::extract(content),
        Some("Java") => jvm::extract_java(content),
        Some("Kotlin") => jvm::extract_kotlin(content),
        _ => return Vec::new(),
    };
    set.into_iter().collect()
}
