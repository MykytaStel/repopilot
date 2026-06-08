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

/// Leading verbs that mutate/assign an auth artifact rather than *check* it.
/// `applyRole`, `setSession`, and `grantPermission` change authz state; they are
/// not auth verifications and must not count as a removed auth check.
const MUTATION_VERBS: &[&str] = &[
    "set", "apply", "assign", "create", "delete", "remove", "update", "add", "grant", "revoke",
    "clear", "drop", "insert", "save", "store", "write", "build", "make", "register", "init",
    "reset", "destroy", "unset", "put",
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

/// True when a call's callee — its `object.method` path with arguments stripped —
/// names an authentication/authorization *check*.
///
/// Used by the AST removed-signal path so that only the callee, never the
/// arguments or string literals, decides whether a call is an auth check:
/// `getUserProfile(session.userId)` and `log("authenticate")` do not count.
/// Strong auth tokens (`jwt`, `authentic`, …) always qualify; the shorter
/// ambiguous nouns (`role`, `session`, …) qualify only when the leading token is
/// not a mutation verb, so `applyRole(x)`/`setSession(s)` are excluded.
pub(super) fn callee_is_auth_check(callee: &str) -> bool {
    let lower = callee.to_ascii_lowercase();
    if AUTH_STRONG.iter().any(|needle| lower.contains(needle)) {
        return true;
    }
    let tokens = ordered_tokens(callee);
    if !tokens
        .iter()
        .any(|token| AUTH_TOKENS.contains(&token.as_str()))
    {
        return false;
    }
    // The leading token is the verb; a mutation verb means this changes the auth
    // artifact rather than checking it.
    !tokens
        .first()
        .is_some_and(|first| MUTATION_VERBS.contains(&first.as_str()))
}

/// Split text into ordered lowercase alphanumeric tokens, breaking on separators
/// and on lowercase→uppercase transitions so `requireAuth` yields `[require,
/// auth]`. Order is preserved so callers can inspect the leading (verb) token.
fn ordered_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
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

/// Whole-token set, for membership checks that do not care about order.
fn tokenize(text: &str) -> HashSet<String> {
    ordered_tokens(text).into_iter().collect()
}

fn flush(current: &mut String, tokens: &mut Vec<String>) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use super::{callee_is_auth_check, line_has_auth, line_has_error_handling};

    #[test]
    fn callee_auth_check_matches_checks_not_args_or_mutations() {
        // Real auth checks (callee names) still qualify.
        assert!(callee_is_auth_check("checkPermission"));
        assert!(callee_is_auth_check("isAuthenticated"));
        assert!(callee_is_auth_check("req.isAuthenticated"));
        assert!(callee_is_auth_check("requireAuth"));
        assert!(callee_is_auth_check("verifyJWT"));
        assert!(callee_is_auth_check("hasRole"));
        // Callees that are not auth checks must not qualify.
        assert!(!callee_is_auth_check("getUserProfile")); // arg `session.userId` is not the callee
        assert!(!callee_is_auth_check("log")); // arg "authenticate" is not the callee
        assert!(!callee_is_auth_check("applyRole")); // mutation, not a check
        assert!(!callee_is_auth_check("setSession"));
        assert!(!callee_is_auth_check("grantPermission"));
        // Lookalikes must not fire.
        assert!(!callee_is_auth_check("getAuthor"));
        assert!(!callee_is_auth_check("authority"));
    }

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
