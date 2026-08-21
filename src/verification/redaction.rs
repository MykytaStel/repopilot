#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedStream {
    pub excerpt: String,
    pub truncated: bool,
}

pub(crate) fn capture_and_redact(bytes: &[u8], observed_more: bool) -> CapturedStream {
    let text = String::from_utf8_lossy(bytes);
    let terminal_safe = strip_terminal_sequences(&text);
    let mut excerpt = String::new();
    let mut inside_private_key = false;

    for segment in terminal_safe.split_inclusive('\n') {
        let has_newline = segment.ends_with('\n');
        let line = segment.trim_end_matches('\n');
        if line.contains("-----BEGIN ") && line.contains("PRIVATE KEY-----") {
            inside_private_key = true;
            excerpt.push_str("[REDACTED PRIVATE KEY]");
        } else if inside_private_key {
            if line.contains("-----END ") && line.contains("PRIVATE KEY-----") {
                inside_private_key = false;
            }
        } else {
            excerpt.push_str(&redact_line(line));
        }
        if has_newline && !inside_private_key {
            excerpt.push('\n');
        }
    }

    CapturedStream {
        excerpt,
        truncated: observed_more,
    }
}

fn redact_line(line: &str) -> String {
    if let Some((prefix, _)) = sensitive_assignment(line) {
        return format!("{prefix}[REDACTED]");
    }
    line.split_whitespace()
        .map(|token| {
            let candidate = token.trim_matches(|ch: char| {
                !ch.is_ascii_alphanumeric() && ch != '.' && ch != '_' && ch != '-'
            });
            if is_jwt(candidate) {
                "[REDACTED JWT]"
            } else if is_provider_token(candidate) {
                "[REDACTED TOKEN]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sensitive_assignment(line: &str) -> Option<(&str, &str)> {
    for (separator, _) in line.match_indices(['=', ':']) {
        let before = line[..separator].trim_end();
        let key_start = before
            .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'))
            .map_or(0, |index| index + 1);
        let key = before[key_start..].to_ascii_lowercase();
        let sensitive = ["token", "password", "secret", "credential"]
            .iter()
            .any(|word| {
                key == *word
                    || key.ends_with(&format!("_{word}"))
                    || key.ends_with(&format!("-{word}"))
            });
        if sensitive {
            let value_start = line[separator + 1..]
                .find(|ch: char| !ch.is_whitespace())
                .map_or(line.len(), |offset| separator + 1 + offset);
            return Some(line.split_at(value_start));
        }
    }
    None
}

fn is_jwt(token: &str) -> bool {
    let parts = token.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            part.len() >= 4
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        })
}

fn is_provider_token(token: &str) -> bool {
    (token.starts_with("ghp_") && token.len() >= 24)
        || (token.starts_with("github_pat_") && token.len() >= 32)
        || (token.starts_with("sk-") && token.len() >= 24)
        || (token.starts_with("AKIA") && token.len() == 20)
}

fn strip_terminal_sequences(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.next() {
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    let mut previous_escape = false;
                    for next in chars.by_ref() {
                        if next == '\u{7}' || (previous_escape && next == '\\') {
                            break;
                        }
                        previous_escape = next == '\u{1b}';
                    }
                }
                Some(_) | None => {}
            }
        } else if ch == '\n' || ch == '\t' || (!ch.is_control() && ch != '\u{7f}') {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::capture_and_redact;

    #[test]
    fn strips_terminal_controls_and_masks_sensitive_assignments() {
        let captured = capture_and_redact(
            b"\x1b[31mfailed\x1b[0m\x07\ntoken=synthetic-secret\npassword: fake-password\n",
            false,
        );

        assert_eq!(
            captured.excerpt,
            "failed\ntoken=[REDACTED]\npassword: [REDACTED]\n"
        );
        assert!(!captured.truncated);
    }

    #[test]
    fn masks_private_keys_and_jwt_shapes() {
        let captured = capture_and_redact(
            b"-----BEGIN PRIVATE KEY-----\nZmFrZQ==\n-----END PRIVATE KEY-----\neyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJmYWtlIn0.c2lnbmF0dXJl\n",
            false,
        );

        assert!(!captured.excerpt.contains("ZmFrZQ"));
        assert!(!captured.excerpt.contains("eyJhbGci"));
        assert!(captured.excerpt.contains("[REDACTED PRIVATE KEY]"));
        assert!(captured.excerpt.contains("[REDACTED JWT]"));
    }

    #[test]
    fn converts_invalid_utf8_only_after_bounding() {
        let captured = capture_and_redact(&[b'o', b'k', 0xff], true);

        assert_eq!(captured.excerpt, "ok\u{fffd}");
        assert!(captured.truncated);
    }

    #[test]
    fn masks_provider_token_shapes_without_literal_secret_fixtures() {
        let synthetic = format!("{}{}", "ghp_", "A".repeat(36));
        let captured = capture_and_redact(format!("provider {synthetic}\n").as_bytes(), false);

        assert_eq!(captured.excerpt, "provider [REDACTED TOKEN]\n");
        assert!(!captured.excerpt.contains(&synthetic));
    }

    #[test]
    fn masks_sensitive_assignment_embedded_in_a_log_prefix() {
        let captured = capture_and_redact(b"error: API_TOKEN = synthetic-secret\n", false);

        assert_eq!(captured.excerpt, "error: API_TOKEN = [REDACTED]\n");
    }
}
