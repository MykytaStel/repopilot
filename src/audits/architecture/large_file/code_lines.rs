//! Counts the lines of a file that are actually code.
//!
//! `architecture.large-file` says "this file has N non-empty lines of code",
//! and it was counting comments as code. ripgrep's `crates/matcher/src/lib.rs`
//! reports 1269 that way, but 480 of its lines are `///` and `//!` doc comments
//! on a trait definition: a thoroughly documented file, not a large one.
//! Splitting it would remove documentation, not responsibilities.
//!
//! Comment syntax is per language, and the same character means opposite things
//! across them — `#` opens a comment in Python and a preprocessor directive in
//! C — so the prefix set is chosen from the language rather than guessed.

/// Line-comment prefixes for a language label, and whether it uses C-style
/// `/* … */` blocks.
struct CommentSyntax {
    line_prefixes: &'static [&'static str],
    block_comments: bool,
}

fn syntax_for(language: &str) -> Option<CommentSyntax> {
    let syntax = match language {
        "Rust" | "TypeScript" | "TypeScript React" | "JavaScript" | "JavaScript React" | "Go"
        | "Java" | "Kotlin" | "C#" | "C" | "C++" | "Swift" | "Scala" | "PHP" | "Dart" => {
            CommentSyntax {
                line_prefixes: &["//"],
                block_comments: true,
            }
        }
        "Python" | "Ruby" | "Shell" | "YAML" | "TOML" => CommentSyntax {
            line_prefixes: &["#"],
            block_comments: false,
        },
        "SQL" | "Haskell" | "Lua" => CommentSyntax {
            line_prefixes: &["--"],
            block_comments: false,
        },
        _ => return None,
    };
    Some(syntax)
}

/// Non-empty lines that are not comment-only, or `None` when the language's
/// comment syntax is unknown and the caller should keep its existing count
/// rather than guess.
pub(super) fn count_code_lines(content: &str, language: Option<&str>) -> Option<usize> {
    let syntax = syntax_for(language?)?;
    let mut count = 0usize;
    let mut in_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            // A block that ends mid-line leaves code behind on the same line.
            if let Some(rest) = trimmed.split_once("*/") {
                in_block = false;
                if !rest.1.trim().is_empty() {
                    count += 1;
                }
            }
            continue;
        }
        if syntax
            .line_prefixes
            .iter()
            .any(|prefix| trimmed.starts_with(prefix))
        {
            continue;
        }
        if syntax.block_comments && trimmed.starts_with("/*") {
            // `/* … */` on one line is a comment; an unterminated one opens a block.
            if !trimmed.contains("*/") {
                in_block = true;
            }
            continue;
        }
        // A continuation line of a block comment, conventionally `* text`.
        if syntax.block_comments && trimmed.starts_with('*') {
            continue;
        }
        count += 1;
    }

    Some(count)
}

#[cfg(test)]
mod tests;
