use crate::analysis::parse::ParsedFile;
use std::collections::{BTreeMap, HashSet};

pub(super) fn eager(parsed: &ParsedFile) -> HashSet<String> {
    extract(parsed.content())
}

pub(super) fn spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    extract_spans(parsed.content())
}

/// Line-based `using` directive extraction, mirroring the Java/Kotlin
/// extractors. Handles `using N;`, `global using N;`, `using static T;`,
/// and the alias form `using A = N.T;` (the aliased path is the edge).
/// `using (resource)` statements and `using var x = …` declarations are
/// resource management, not imports, and are skipped.
fn extract_spans(content: &str) -> BTreeMap<String, (usize, usize)> {
    let mut result = BTreeMap::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
            continue;
        }
        let rest = trimmed
            .strip_prefix("global using ")
            .or_else(|| trimmed.strip_prefix("using "));
        let Some(rest) = rest else { continue };
        let rest = rest
            .trim_start_matches("static ")
            .trim_end_matches(';')
            .trim();

        if rest.starts_with('(') || rest.starts_with("var ") {
            continue;
        }
        // Alias form: the aliased namespace/type path is the real edge.
        let path = match rest.split_once('=') {
            Some((_, aliased)) => aliased.trim(),
            None => rest,
        };
        if !path.is_empty() && is_namespace_path(path) {
            result.entry(path.to_string()).or_insert((i + 1, i + 1));
        }
    }
    result
}

fn is_namespace_path(path: &str) -> bool {
    !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
}

fn extract(content: &str) -> HashSet<String> {
    extract_spans(content).into_keys().collect()
}

#[cfg(test)]
mod tests {
    use super::extract_spans;

    #[test]
    fn extracts_using_directives_and_aliases() {
        let content = "using System.Text;\n\
                       global using Microsoft.AspNetCore.Mvc;\n\
                       using static System.Math;\n\
                       using Project = PetShop.Domain.Orders;\n";
        let spans = extract_spans(content);
        let keys: Vec<&str> = spans.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            [
                "Microsoft.AspNetCore.Mvc",
                "PetShop.Domain.Orders",
                "System.Math",
                "System.Text",
            ]
        );
        assert_eq!(spans["System.Text"], (1, 1));
    }

    #[test]
    fn skips_resource_management_using_forms() {
        let content = "using (var stream = File.Open(path))\n\
                       using var reader = new StreamReader(stream);\n\
                       // using System.Commented;\n";
        assert!(extract_spans(content).is_empty());
    }
}
