/// Strips comment content and string literal content from a line using C-style
/// comment syntax (`//` and `/* */`). Characters inside comments or string
/// literals are replaced with spaces so callers can safely match code patterns
/// without false positives from commented-out or stringified code.
///
/// `in_block_comment` must be carried across successive lines; callers own the
/// state and pass a mutable reference for each line in the file.
///
/// Handles mid-line block comments correctly (e.g. `foo(); /* comment */ bar()`).
/// Does not handle Rust raw strings or nested block comments — a future
/// parser-level pass is the right place for those.
pub fn sanitize_c_style(line: &str, in_block_comment: &mut bool) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block_comment = false;
                output.push(' ');
                output.push(' ');
            } else {
                output.push(' ');
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            output.push(' ');
            continue;
        }

        if in_char {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_char = false;
            }
            output.push(' ');
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            break;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            *in_block_comment = true;
            output.push(' ');
            output.push(' ');
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(' ');
            continue;
        }

        if ch == '\'' {
            in_char = true;
            output.push(' ');
            continue;
        }

        output.push(ch);
    }

    output
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
    let mut result = String::with_capacity(line.len());
    let mut string_delimiter: Option<char> = None;
    let mut escaped = false;

    for ch in line.chars() {
        if let Some(delim) = string_delimiter {
            if escaped {
                escaped = false;
                result.push(' ');
                continue;
            }
            if ch == '\\' {
                escaped = true;
                result.push(' ');
                continue;
            }
            if ch == delim {
                string_delimiter = None;
                result.push(ch);
            } else {
                result.push(' ');
            }
            continue;
        }

        if ch == '#' {
            break;
        }

        if matches!(ch, '"' | '\'') {
            string_delimiter = Some(ch);
            result.push(ch);
        } else {
            result.push(ch);
        }
    }

    result
}
