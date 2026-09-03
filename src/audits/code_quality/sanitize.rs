/// Strips comment content and string literal content from a line using C-style
/// comment syntax (`//` and `/* */`). Characters inside comments or string
/// literals are replaced with spaces so callers can safely match code patterns
/// without false positives from commented-out or stringified code.
///
/// `in_block_comment` must be carried across successive lines; callers own the
/// state and pass a mutable reference for each line in the file.
///
/// Handles mid-line block comments correctly (e.g. `foo(); /* comment */ bar()`).
/// Rust raw strings and nested block comments are not handled.
pub fn sanitize_c_style(line: &str, in_block_comment: &mut bool) -> String {
    CStyleSanitizer::new(line, in_block_comment).run()
}

struct CStyleSanitizer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    output: String,
    in_block_comment: &'a mut bool,
    in_string: bool,
    in_char: bool,
    escaped: bool,
}

impl<'a> CStyleSanitizer<'a> {
    fn new(line: &'a str, in_block_comment: &'a mut bool) -> Self {
        Self {
            chars: line.chars().peekable(),
            output: String::with_capacity(line.len()),
            in_block_comment,
            in_string: false,
            in_char: false,
            escaped: false,
        }
    }

    fn run(mut self) -> String {
        while let Some(ch) = self.chars.next() {
            if self.consume_block_comment(ch) || self.consume_literal(ch) {
                continue;
            }
            if self.is_line_comment_start(ch) {
                break;
            }
            if self.consume_literal_start(ch) {
                continue;
            }
            self.output.push(ch);
        }
        self.output
    }

    fn consume_block_comment(&mut self, ch: char) -> bool {
        if !*self.in_block_comment {
            return false;
        }
        if ch == '*' && self.chars.peek() == Some(&'/') {
            self.chars.next();
            *self.in_block_comment = false;
            self.output.push_str("  ");
        } else {
            self.output.push(' ');
        }
        true
    }

    fn consume_literal(&mut self, ch: char) -> bool {
        let delimiter = if self.in_string {
            Some('"')
        } else if self.in_char {
            Some('\'')
        } else {
            None
        };
        let Some(delimiter) = delimiter else {
            return false;
        };
        if self.escaped {
            self.escaped = false;
        } else if ch == '\\' {
            self.escaped = true;
        } else if ch == delimiter {
            self.in_string = false;
            self.in_char = false;
        }
        self.output.push(' ');
        true
    }

    fn is_line_comment_start(&mut self, ch: char) -> bool {
        ch == '/' && self.chars.peek() == Some(&'/')
    }

    fn consume_literal_start(&mut self, ch: char) -> bool {
        if ch == '/' && self.chars.peek() == Some(&'*') {
            self.chars.next();
            *self.in_block_comment = true;
            self.output.push_str("  ");
            return true;
        }
        if ch == '"' {
            self.in_string = true;
            self.output.push(' ');
            return true;
        }
        if ch == '\'' {
            self.in_char = true;
            self.output.push(' ');
            return true;
        }
        false
    }
}

/// Returns `Some(sanitized)` for Python code lines, or `None` if the line
/// is a comment or becomes empty after stripping the `#`-delimited comment
/// portion and string literal contents.
pub fn sanitize_python_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let code = strip_python_string_literals(line);
    if code.trim().is_empty() {
        None
    } else {
        Some(code)
    }
}

fn strip_python_string_literals(line: &str) -> String {
    let mut state = PythonStringState::default();
    for ch in line.chars() {
        if state.consume_string(ch) {
            continue;
        }
        if ch == '#' {
            break;
        }
        state.start_or_copy(ch);
    }
    state.result
}

#[derive(Default)]
struct PythonStringState {
    result: String,
    delimiter: Option<char>,
    escaped: bool,
}

impl PythonStringState {
    fn consume_string(&mut self, ch: char) -> bool {
        let Some(delimiter) = self.delimiter else {
            return false;
        };
        if self.escaped {
            self.escaped = false;
        } else if ch == '\\' {
            self.escaped = true;
        } else if ch == delimiter {
            self.delimiter = None;
            self.result.push(ch);
            return true;
        }
        self.result.push(' ');
        true
    }

    fn start_or_copy(&mut self, ch: char) {
        if matches!(ch, '"' | '\'') {
            self.delimiter = Some(ch);
        }
        self.result.push(ch);
    }
}
