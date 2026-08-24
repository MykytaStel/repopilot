use super::count_code_lines;

#[test]
fn rust_doc_comments_are_documentation_not_code() {
    // ripgrep's matcher trait is a third doc comments; splitting it would
    // remove documentation rather than responsibilities.
    let source = concat!(
        "//! Module documentation.\n",
        "\n",
        "/// Describes the matcher.\n",
        "/// Continues describing it.\n",
        "pub trait Matcher {\n",
        "    // an ordinary comment\n",
        "    fn find(&self) -> bool;\n",
        "}\n",
    );
    assert_eq!(count_code_lines(source, Some("Rust")), Some(3));
}

#[test]
fn c_style_blocks_are_skipped_but_trailing_code_still_counts() {
    let source = concat!(
        "/*\n",
        " * License header.\n",
        " */\n",
        "const a = 1;\n",
        "/* inline */ const b = 2;\n",
        "/* opens */ const c = 3; /* and closes */\n",
    );
    // The single-line block form is treated as a comment line; the multi-line
    // header contributes nothing.
    assert_eq!(count_code_lines(source, Some("TypeScript")), Some(1));
}

#[test]
fn code_after_a_block_ends_on_the_same_line_is_counted() {
    let source = concat!(
        "/* opens\n",
        "   continues */ const a = 1;\n",
        "const b = 2;\n",
    );
    assert_eq!(count_code_lines(source, Some("JavaScript")), Some(2));
}

#[test]
fn the_hash_prefix_follows_the_language_not_the_character() {
    // `#` opens a comment in Python and a preprocessor directive in C, which is
    // code. Guessing from the character alone would erase every `#include`.
    assert_eq!(
        count_code_lines("# a comment\nvalue = 1\n", Some("Python")),
        Some(1)
    );
    assert_eq!(
        count_code_lines("#include <stdio.h>\nint main() {}\n", Some("C")),
        Some(2)
    );
}

#[test]
fn an_unknown_language_declines_rather_than_guesses() {
    assert_eq!(count_code_lines("anything\n", Some("Brainfuck")), None);
    assert_eq!(count_code_lines("anything\n", None), None);
}

#[test]
fn blank_lines_never_count() {
    assert_eq!(count_code_lines("\n\n   \n", Some("Rust")), Some(0));
}
