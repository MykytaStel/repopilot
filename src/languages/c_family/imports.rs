use crate::analysis::parse::ParsedFile;
use std::collections::{BTreeMap, HashSet};

pub(super) fn eager(parsed: &ParsedFile) -> HashSet<String> {
    includes(parsed.content())
        .into_iter()
        .map(|(path, _, _)| path)
        .collect()
}

pub(super) fn spans(parsed: &ParsedFile) -> BTreeMap<String, (usize, usize)> {
    let mut spans = BTreeMap::new();
    for (path, line, _) in includes(parsed.content()) {
        spans.entry(path).or_insert((line, line));
    }
    spans
}

fn includes(content: &str) -> Vec<(String, usize, usize)> {
    let mut lexer = Lexer::default();
    let mut depth = 0usize;
    let mut found = Vec::new();
    let mut directive = String::new();
    let mut line_start = 1;
    let mut spliced_directive = false;

    for (start, line, spliced) in logical_lines(content) {
        if directive.trim().is_empty() {
            line_start = start;
        }
        lexer.append_code(&line, &mut directive);
        spliced_directive |= spliced;
        if lexer.in_block_comment {
            continue;
        }
        match directive_parts(&directive) {
            Some(("if" | "ifdef" | "ifndef", _)) => depth += 1,
            Some(("endif", _)) => depth = depth.saturating_sub(1),
            Some(("include", operand)) if depth == 0 && !spliced_directive => {
                if let Some(path) = quoted_include(operand) {
                    found.push((path, line_start, line_start));
                }
            }
            _ => {}
        }
        directive.clear();
        spliced_directive = false;
    }
    found
}

fn logical_lines(content: &str) -> impl Iterator<Item = (usize, std::borrow::Cow<'_, str>, bool)> {
    use std::borrow::Cow;

    let mut lines = content.lines().enumerate();
    std::iter::from_fn(move || {
        let (index, line) = lines.next()?;
        let Some(first) = line.trim_end().strip_suffix('\\') else {
            return Some((index + 1, Cow::Borrowed(line), false));
        };
        let mut joined = first.to_owned();
        for (_, continuation) in lines.by_ref() {
            if let Some(part) = continuation.trim_end().strip_suffix('\\') {
                joined.push_str(part);
            } else {
                joined.push_str(continuation);
                break;
            }
        }
        Some((index + 1, Cow::Owned(joined), true))
    })
}

#[derive(Default)]
struct Lexer {
    in_block_comment: bool,
    raw_end: Option<String>,
}

impl Lexer {
    fn append_code(&mut self, mut rest: &str, out: &mut String) {
        while !rest.is_empty() {
            if self.in_block_comment {
                let Some((_, after)) = rest.split_once("*/") else {
                    return;
                };
                self.in_block_comment = false;
                rest = after;
            } else if let Some(end) = &self.raw_end {
                out.push('x');
                let Some((_, after)) = rest.split_once(end.as_str()) else {
                    return;
                };
                self.raw_end = None;
                rest = after;
            } else if rest.starts_with("//") {
                return;
            } else if let Some(after) = rest.strip_prefix("/*") {
                self.in_block_comment = true;
                out.push(' ');
                rest = after;
            } else if let Some((after, delimiter)) = raw_string_start(rest, out) {
                self.raw_end = Some(format!("){delimiter}\""));
                out.push('x');
                rest = after;
            } else if rest.starts_with('"') || rest.starts_with('\'') {
                let end = quoted_token_end(rest);
                out.push_str(&rest[..end]);
                rest = &rest[end..];
            } else if let Some(first) = rest.chars().next() {
                out.push(first);
                rest = &rest[first.len_utf8()..];
            }
        }
    }
}

fn raw_string_start<'a>(input: &'a str, preceding: &str) -> Option<(&'a str, &'a str)> {
    if preceding.ends_with(|ch: char| ch.is_alphanumeric() || ch == '_') {
        return None;
    }
    let delimiter = ["R\"", "u8R\"", "uR\"", "UR\"", "LR\""]
        .into_iter()
        .find_map(|prefix| input.strip_prefix(prefix))?;
    let (delimiter, after) = delimiter.split_once('(')?;
    if delimiter.len() > 16
        || !delimiter
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b')' | b'\\'))
    {
        return None;
    }
    Some((after, delimiter))
}

fn quoted_token_end(input: &str) -> usize {
    let delimiter = input.as_bytes()[0];
    let mut escaped = false;
    for (index, byte) in input.bytes().enumerate().skip(1) {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == delimiter {
            return index + 1;
        }
    }
    input.len()
}

fn directive_parts(code: &str) -> Option<(&str, &str)> {
    let trimmed = code.trim_start();
    let directive = trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("%:"))?
        .trim_start();
    let end = directive
        .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .unwrap_or(directive.len());
    Some((&directive[..end], &directive[end..]))
}

fn quoted_include(operand: &str) -> Option<String> {
    let path = operand.trim_start().strip_prefix('"')?;
    let (raw, remainder) = path.split_once('"')?;
    if raw.is_empty() || raw.contains('\\') || raw.starts_with('/') || !remainder.trim().is_empty()
    {
        return None;
    }
    Some(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::parse::ParsedFile;

    #[test]
    fn extracts_only_direct_quoted_includes_with_stable_spans() {
        let parsed = ParsedFile::new(
            "/* ignored\n#include \"comment.h\" */\n  # include \"local/file.hpp\" // note\n#include <vector>\n#include HEADER\n#include \"local/file.hpp\"\n",
            Some("C++"),
        );
        assert_eq!(
            eager(&parsed),
            HashSet::from(["local/file.hpp".to_string()])
        );
        assert_eq!(spans(&parsed).get("local/file.hpp"), Some(&(3, 3)));
    }

    #[test]
    fn ignores_spliced_directives_and_include_lookalikes() {
        let parsed = ParsedFile::new(
            "#include \"one.h\" \\\n\n#include_next \"two.h\"\ncode #include \"three.h\"\n",
            Some("C"),
        );
        assert!(eager(&parsed).is_empty());
    }

    #[test]
    fn tracks_block_comments_that_begin_after_a_directive() {
        let parsed = ParsedFile::new(
            "#include \"one.h\" /* comment\n#include \"ignored.h\"\n*/\n#include \"two.h\"\n",
            Some("C"),
        );
        assert_eq!(
            eager(&parsed),
            HashSet::from(["one.h".to_string(), "two.h".to_string()])
        );
    }

    #[test]
    fn ignores_includes_in_nested_conditional_compilation_blocks() {
        let parsed = ParsedFile::new(
            "#include \"before.h\"\n#if FEATURE\n#include \"if.h\"\n#if OTHER\n#include \"nested.h\"\n#else\n#include \"nested_else.h\"\n#endif\n#elif FALLBACK\n#include \"elif.h\"\n#else\n#include \"else.h\"\n#endif\n#include \"after.h\"\n",
            Some("C++"),
        );
        assert_eq!(
            eager(&parsed),
            HashSet::from(["before.h".to_string(), "after.h".to_string()])
        );
    }

    #[test]
    fn ignores_spliced_conditional_directives_and_their_include_branches() {
        let parsed = ParsedFile::new(
            "#if FEATURE \\\n+    && OTHER\n#include \"conditional.h\"\n#endif\n#include \"direct.h\"\n",
            Some("C"),
        );
        assert_eq!(eager(&parsed), HashSet::from(["direct.h".to_string()]));
    }

    #[test]
    fn v028_lexes_spliced_and_comment_separated_conditionals_and_raw_strings() {
        let parsed = ParsedFile::new(
            concat!(
                "#i\\\nf 0\n#include \"spliced.h\"\n#endif\n",
                "#/* gap */if 0\n#include \"conditional.h\"\n#endif\n",
                "# /* spanning\n comment */ if 0\n#include \"commented.h\"\n#endif\n",
                "auto literal = u8R\"tag(\n#include \"raw.h\"\n)tag\";\n",
                "#/* gap */include\"only.h\"\n",
            ),
            Some("C++"),
        );
        assert_eq!(eager(&parsed), HashSet::from(["only.h".to_string()]));
    }
}
