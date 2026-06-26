// A Rust enum-path qualifier (`Token::RecursiveSuffix`) is not a
// `token: <secret>` assignment — the `::` after the keyword names a type.
// (Regression: ripgrep's glob parser was flagged here.)
enum Token {
    RecursivePrefix,
    RecursiveSuffix,
    RecursiveZeroOrMore,
}

fn classify(token: &Token) -> bool {
    match token {
        Token::RecursivePrefix => true,
        Token::RecursiveSuffix => true,
        Token::RecursiveZeroOrMore => false,
    }
}
