//! Shared tree-sitter parsing.
//!
//! Centralizes the tree-sitter parser instances and grammar selection that were
//! previously duplicated across the AST-based audits and the import graph. Each
//! thread keeps one reusable parser per grammar via `thread_local!`, so parsing
//! is cheap to repeat and safe under the parallel file pipeline.

use crate::analysis::SyntaxSummary;
use crate::scan::facts::FileFacts;
use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tree_sitter::{Language, Parser, Tree};

/// Process-global accumulator of time spent inside tree-sitter parsing, in
/// nanoseconds. Summed across every [`parse`] call on every worker thread, so it
/// reflects aggregate parse CPU time rather than wall-clock. Read as a delta
/// (`after - before`) around a stage to attribute parse cost to that stage; the
/// figure is exact when a single scan runs in the process (the CLI case) and an
/// over-estimate only if scans run concurrently in one process.
static PARSE_NANOS: AtomicU64 = AtomicU64::new(0);

/// Total nanoseconds spent parsing so far in this process. See [`PARSE_NANOS`].
pub(crate) fn parse_nanos_total() -> u64 {
    PARSE_NANOS.load(Ordering::Relaxed)
}

/// A source grammar RepoPilot can parse with tree-sitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParseLanguage {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
    Python,
    Go,
    Java,
    CSharp,
    Kotlin,
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
            "Go" => Some(Self::Go),
            "Java" => Some(Self::Java),
            "CSharp" | "C#" => Some(Self::CSharp),
            "Kotlin" => Some(Self::Kotlin),
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
    static GO_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_go::LANGUAGE.into()));
    static JAVA_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_java::LANGUAGE.into()));
    static CSHARP_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_c_sharp::LANGUAGE.into()));
    static KOTLIN_PARSER: RefCell<Parser> =
        RefCell::new(parser_for(tree_sitter_kotlin_ng::LANGUAGE.into()));
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
        ParseLanguage::Go => &GO_PARSER,
        ParseLanguage::Java => &JAVA_PARSER,
        ParseLanguage::CSharp => &CSHARP_PARSER,
        ParseLanguage::Kotlin => &KOTLIN_PARSER,
    };
    let start = Instant::now();
    let tree = parser.with(|cell| {
        let mut parser = cell.borrow_mut();
        parser.reset();
        parser.parse(content, None)
    });
    PARSE_NANOS.fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    tree
}

/// Convenience over [`parse`] that maps a language label first. Returns `None`
/// when the label has no parseable grammar or tree-sitter fails.
pub(crate) fn parse_label(content: &str, label: &str) -> Option<Tree> {
    parse(content, ParseLanguage::from_label(label)?)
}

/// A parse-once view over a single file's source.
///
/// The syntax tree is produced lazily on the first [`ParsedFile::tree`] call and
/// cached, so multiple audits inspecting the same file share one parse instead
/// of re-parsing per audit. Files that no audit inspects are never parsed.
pub struct ParsedFile<'a> {
    content: &'a str,
    language_label: Option<&'a str>,
    tree: OnceLock<Option<Tree>>,
}

impl<'a> ParsedFile<'a> {
    pub(crate) fn new(content: &'a str, language_label: Option<&'a str>) -> Self {
        Self {
            content,
            language_label,
            tree: OnceLock::new(),
        }
    }

    /// Builds a parse view from a file's facts, borrowing its content and
    /// detected language label. Parsing is deferred until [`ParsedFile::tree`].
    pub(crate) fn for_facts(file: &'a FileFacts) -> Self {
        Self::new(
            file.content.as_deref().unwrap_or(""),
            file.language.as_deref(),
        )
    }

    /// The borrowed source text this view parses. Consumers that walk the tree
    /// need the original bytes for `utf8_text`, and reading it here keeps the
    /// content and its syntax tree paired to a single parse.
    pub(crate) fn content(&self) -> &str {
        self.content
    }

    /// Whether the syntax tree has been materialized yet. Used by tests to assert
    /// that consumers sharing one view trigger at most one parse.
    #[cfg(test)]
    pub(crate) fn was_parsed(&self) -> bool {
        self.tree.get().is_some()
    }

    /// Lazily parses (once) and returns the syntax tree, or `None` when the file
    /// has no parseable grammar or tree-sitter cannot produce a tree.
    pub(crate) fn tree(&self) -> Option<&Tree> {
        self.tree
            .get_or_init(|| {
                self.language_label
                    .and_then(|label| parse_label(self.content, label))
            })
            .as_ref()
    }

    pub(crate) fn syntax_summary(&self) -> SyntaxSummary {
        let Some(tree) = self.tree() else {
            return SyntaxSummary::unavailable();
        };
        let root = tree.root_node();

        SyntaxSummary {
            parsed: true,
            root_kind: Some(root.kind().to_string()),
            has_errors: root.has_error(),
            named_child_count: root.named_child_count(),
        }
    }
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
        assert_eq!(ParseLanguage::from_label("Go"), Some(ParseLanguage::Go));
        assert_eq!(ParseLanguage::from_label("Java"), Some(ParseLanguage::Java));
        assert_eq!(
            ParseLanguage::from_label("CSharp"),
            Some(ParseLanguage::CSharp)
        );
        assert_eq!(ParseLanguage::from_label("C#"), Some(ParseLanguage::CSharp));
        assert_eq!(
            ParseLanguage::from_label("Kotlin"),
            Some(ParseLanguage::Kotlin)
        );
    }

    #[test]
    fn rejects_unparseable_labels() {
        assert_eq!(ParseLanguage::from_label("Ruby"), None);
        assert_eq!(ParseLanguage::from_label("PHP"), None);
        assert_eq!(ParseLanguage::from_label(""), None);
    }

    #[test]
    fn parses_each_grammar() {
        assert!(parse("fn main() {}", ParseLanguage::Rust).is_some());
        assert!(parse("const x: number = 1;", ParseLanguage::TypeScript).is_some());
        assert!(parse("const x = <div />;", ParseLanguage::Tsx).is_some());
        assert!(parse("const x = 1;", ParseLanguage::JavaScript).is_some());
        assert!(parse("x = 1\n", ParseLanguage::Python).is_some());
        assert!(parse("package main\nfunc main() {}\n", ParseLanguage::Go).is_some());
        assert!(parse("public class Main {}\n", ParseLanguage::Java).is_some());
        assert!(parse("class Main {}\n", ParseLanguage::CSharp).is_some());
        assert!(parse("fun main() {}\n", ParseLanguage::Kotlin).is_some());
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
        assert!(parse_label("package main\n", "Go").is_some());
    }

    #[test]
    fn parsed_file_parses_once_and_caches() {
        let parsed = ParsedFile::new("fn main() {}", Some("Rust"));
        let first = parsed.tree().expect("rust source should parse") as *const Tree;
        let second = parsed.tree().expect("cached tree should be returned") as *const Tree;
        // Same cached tree instance on repeat access => parsed at most once.
        assert_eq!(first, second);
    }

    #[test]
    fn parsed_file_without_grammar_has_no_tree() {
        assert!(ParsedFile::new("class Main", Some("Ruby")).tree().is_none());
        assert!(ParsedFile::new("anything", None).tree().is_none());
    }

    #[test]
    fn syntax_summary_reuses_the_same_cached_tree() {
        let parsed = ParsedFile::new(
            "use crate::facts;
fn main() {}",
            Some("Rust"),
        );
        let first = parsed.tree().expect("tree") as *const Tree;
        let summary = parsed.syntax_summary();
        let second = parsed.tree().expect("same tree") as *const Tree;

        assert_eq!(first, second);
        assert!(summary.parsed);
        assert_eq!(summary.root_kind.as_deref(), Some("source_file"));
    }

    #[test]
    fn shared_view_is_parsed_once_across_import_extraction_and_audits() {
        use crate::graph::imports::extract_imports_from;

        let parsed = ParsedFile::new("use crate::foo::bar;\nfn main() {}", Some("Rust"));
        assert!(
            !parsed.was_parsed(),
            "no parse before any consumer reads it"
        );

        // First consumer: the import graph extracts module references.
        let imports = extract_imports_from(&parsed, Some("Rust"));
        assert!(
            parsed.was_parsed(),
            "import extraction materializes the tree"
        );
        assert!(imports.contains(&"crate::foo::bar".to_string()));

        // Second consumer: an AST audit reaches for the same syntax tree via the
        // exact call audits use. The `OnceCell` guarantees it is not re-parsed.
        assert!(parsed.tree().is_some());
        assert!(parsed.was_parsed());
    }
}
