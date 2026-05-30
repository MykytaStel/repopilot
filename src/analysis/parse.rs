//! Shared tree-sitter parsing.
//!
//! Centralizes the tree-sitter parser instances and grammar selection that were
//! previously duplicated across the AST-based audits and the import graph. Each
//! thread keeps one reusable parser per grammar via `thread_local!`, so parsing
//! is cheap to repeat and safe under the parallel file pipeline.

use std::cell::RefCell;
use tree_sitter::{Language, Parser, Tree};

/// A source grammar RepoPilot can parse with tree-sitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParseLanguage {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
    Python,
}

impl ParseLanguage {
    /// Maps a RepoPilot language label (as produced by language detection and
    /// stored on `FileFacts.language`) to a parseable grammar, if any.
    pub(crate) fn from_label(label: &str) -> Option<Self> {
        match label {
            "Rust" => Some(Self::Rust),
            "TypeScript" => Some(Self::TypeScript),
            "TypeScript React" => Some(Self::Tsx),
            "JavaScript" | "JavaScript React" => Some(Self::JavaScript),
            "Python" => Some(Self::Python),
            _ => None,
        }
    }
}

thread_local! {
    static RUST_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_rust::LANGUAGE.into()));
    static TS_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()));
    static TSX_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_typescript::LANGUAGE_TSX.into()));
    static JS_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_javascript::LANGUAGE.into()));
    static PYTHON_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_python::LANGUAGE.into()));
}

fn parser_for(language: Language) -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("tree-sitter grammar should load");
    parser
}

/// Parses `content` with the grammar for `language`, reusing this thread's
/// parser instance. Returns `None` only if tree-sitter fails to produce a tree.
pub(crate) fn parse(content: &str, language: ParseLanguage) -> Option<Tree> {
    let parser = match language {
        ParseLanguage::Rust => &RUST_PARSER,
        ParseLanguage::TypeScript => &TS_PARSER,
        ParseLanguage::Tsx => &TSX_PARSER,
        ParseLanguage::JavaScript => &JS_PARSER,
        ParseLanguage::Python => &PYTHON_PARSER,
    };
    parser.with(|cell| {
        let mut parser = cell.borrow_mut();
        parser.reset();
        parser.parse(content, None)
    })
}

/// Convenience over [`parse`] that maps a language label first. Returns `None`
/// when the label has no parseable grammar or tree-sitter fails.
pub(crate) fn parse_label(content: &str, label: &str) -> Option<Tree> {
    parse(content, ParseLanguage::from_label(label)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_language_labels() {
        assert_eq!(ParseLanguage::from_label("Rust"), Some(ParseLanguage::Rust));
        assert_eq!(
            ParseLanguage::from_label("TypeScript"),
            Some(ParseLanguage::TypeScript)
        );
        assert_eq!(
            ParseLanguage::from_label("TypeScript React"),
            Some(ParseLanguage::Tsx)
        );
        assert_eq!(
            ParseLanguage::from_label("JavaScript"),
            Some(ParseLanguage::JavaScript)
        );
        assert_eq!(
            ParseLanguage::from_label("JavaScript React"),
            Some(ParseLanguage::JavaScript)
        );
        assert_eq!(
            ParseLanguage::from_label("Python"),
            Some(ParseLanguage::Python)
        );
    }

    #[test]
    fn rejects_unparseable_labels() {
        assert_eq!(ParseLanguage::from_label("Go"), None);
        assert_eq!(ParseLanguage::from_label("Java"), None);
        assert_eq!(ParseLanguage::from_label(""), None);
    }

    #[test]
    fn parses_each_grammar() {
        assert!(parse("fn main() {}", ParseLanguage::Rust).is_some());
        assert!(parse("const x: number = 1;", ParseLanguage::TypeScript).is_some());
        assert!(parse("const x = <div />;", ParseLanguage::Tsx).is_some());
        assert!(parse("const x = 1;", ParseLanguage::JavaScript).is_some());
        assert!(parse("x = 1\n", ParseLanguage::Python).is_some());
    }

    #[test]
    fn reuses_thread_parser_across_calls() {
        // The thread-local parser is reset between calls, so repeated parses on
        // one thread must keep succeeding.
        assert!(parse("fn a() {}", ParseLanguage::Rust).is_some());
        assert!(parse("fn b() {}", ParseLanguage::Rust).is_some());
    }

    #[test]
    fn parse_label_matches_grammar_dispatch() {
        assert!(parse_label("fn main() {}", "Rust").is_some());
        assert!(parse_label("def f():\n    pass\n", "Python").is_some());
        assert!(parse_label("package main", "Go").is_none());
    }
}
