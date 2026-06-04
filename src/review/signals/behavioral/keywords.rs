//! Keyword sets and token matching for the coarse (non-AST) removed-signal
//! fallback in [`super::removed`].
//!
//! The coarse path scans raw diff lines, so it must avoid lookalikes: `catch`
//! must not match `catchy`, `auth` must not match `author`. We tokenize each
//! line (splitting on non-alphanumerics and on lowercase→uppercase transitions,
//! mirroring the boundary classifier) and match whole tokens, plus a few
//! high-specificity prefixes for the longer auth words so `authenticate` and
//! `authorization` still count while `author` does not.

use std::collections::HashSet;

/// Error-handling keywords, matched by exact token equality.
const ERROR_TOKENS: &[&str] = &["catch", "except"];

/// High-specificity auth substrings whose collisions in real code are
/// negligible. Checked against the whole lowercased line (not tokens) so they
/// survive the camelCase split that would shred acronyms like `JWT`.
/// `authentic` covers authenticate/authentication; `authoriz` covers
/// authorize/authorization (but not the vaguer `authority`/`author`).
const AUTH_STRONG: &[&str] = &["oauth", "jwt", "rbac", "passport", "authentic", "authoriz"];

/// Short, ambiguous auth keywords, matched by exact token equality so they do
/// not fire on substrings (`auth` must not match `author`).
const AUTH_TOKENS: &[&str] = &[
    "auth",
    "login",
    "logout",
    "signin",
    "signout",
    "permission",
    "permissions",
    "session",
    "sessions",
    "role",
    "roles",
];

/// True when a removed/added line carries an error-handling keyword as a token.
pub(super) fn line_has_error_handling(line: &str) -> bool {
    let tokens = tokenize(line);
    ERROR_TOKENS.iter().any(|needle| tokens.contains(*needle))
}

/// True when a removed/added line carries an auth keyword — a high-specificity
/// substring, or one of the short ambiguous keywords as a whole token.
pub(super) fn line_has_auth(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if AUTH_STRONG.iter().any(|needle| lower.contains(needle)) {
        return true;
    }
    let tokens = tokenize(line);
    AUTH_TOKENS.iter().any(|needle| tokens.contains(*needle))
}

/// Split text into lowercase alphanumeric tokens, breaking on separators and on
/// lowercase→uppercase transitions so `requireAuth` yields `auth`.
fn tokenize(text: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();

    for ch in text.chars() {
        if !ch.is_ascii_alphanumeric() {
            flush(&mut current, &mut tokens);
            continue;
        }
        if ch.is_ascii_uppercase()
            && current
                .chars()
                .last()
                .is_some_and(|last| last.is_ascii_lowercase() || last.is_ascii_digit())
        {
            flush(&mut current, &mut tokens);
        }
        current.push(ch.to_ascii_lowercase());
    }
    flush(&mut current, &mut tokens);
    tokens
}

fn flush(current: &mut String, tokens: &mut HashSet<String>) {
    if !current.is_empty() {
        tokens.insert(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use super::{line_has_auth, line_has_error_handling};

    #[test]
    fn error_handling_matches_real_catch_not_lookalikes() {
        assert!(line_has_error_handling("} catch (e) {"));
        assert!(line_has_error_handling("        except ValueError:"));
        assert!(!line_has_error_handling("const catchy = makeCatcher();"));
        assert!(!line_has_error_handling("// nothing to see here"));
    }

    #[test]
    fn auth_matches_real_tokens_and_prefixes_not_author() {
        assert!(line_has_auth("if (!isAuthenticated(req)) return 401;"));
        assert!(line_has_auth("requireAuth(req, res);"));
        assert!(line_has_auth("checkPermission(user, 'admin');"));
        assert!(line_has_auth("const token = verifyJWT(header);"));
        // Lookalikes that must NOT fire.
        assert!(!line_has_auth("const author = post.author;"));
        assert!(!line_has_auth("import { Authority } from './authority';"));
    }
}
