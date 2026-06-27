fn assigned_value_after_key(after_key: &str) -> Option<&str> {
    let trimmed = after_key.trim_start();

    // `key::Rest` is a namespace/path qualifier (`Token::RecursiveSuffix`,
    // `Self::secret`), not a `key = value` / `key: value` assignment — the text
    // after `::` is a type or enum path, never a hardcoded secret.
    if trimmed.starts_with("::") {
        return None;
    }

    if let Some(value) = trimmed.strip_prefix('=') {
        return Some(value.trim_start());
    }

    let after_colon = trimmed.strip_prefix(':')?.trim_start();
    if let Some((_, value)) = after_colon.split_once('=') {
        return Some(value.trim_start());
    }

    Some(after_colon)
}

fn assigned_secret_confidence_for_key(
    line: &str,
    lower_line: &str,
    key: &str,
) -> Option<Confidence> {
    let mut search_start = 0;

    while let Some(offset) = lower_line[search_start..].find(key) {
        let key_start = search_start + offset;
        search_start = key_start + key.len();
        // Skip keys that appear inside a URL string literal — e.g. `token=`
        // inside a Firebase Storage download URL query string is not a secret.
        if key_is_inside_url_string(line, key_start) {
            continue;
        }
        let after_key = &line[key_start + key.len()..];
        if let Some(confidence) =
            assigned_value_after_key(after_key).and_then(secret_literal_confidence)
        {
            return Some(confidence);
        }
    }

    None
}

// Returns true when `key` at byte offset `key_start` falls inside a URL string
// literal on the line — e.g. `token=` inside a Firebase Storage download URL
// query string is a URL parameter, not a hardcoded secret assignment.
fn key_is_inside_url_string(line: &str, key_start: usize) -> bool {
    let prefix = &line[..key_start];
    if let Some(q) = prefix.rfind('"').or_else(|| prefix.rfind('\'')) {
        let between = &line[q + 1..key_start];
        return between.contains("https://") || between.contains("http://");
    }
    false
}

// Returns true when `value` is an env-var *name* used as a string literal —
// e.g. the `"GITHUB_TOKEN"` in `envvar="GITHUB_TOKEN"`. Env-var names are
// SCREAMING_SNAKE_CASE, so we require at least one underscore: that keeps real
// all-caps alphanumeric secrets (e.g. an AWS access key ID `AKIA…`, which has no
// underscore) detectable while still ignoring env-var name references.
fn looks_like_env_var_name(value: &str) -> bool {
    if value.len() < 3 || !value.contains('_') {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_uppercase() || first == '_')
        && value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

// Lowercase kebab/snake slugs are often storage identifiers, but can also be
// committed passphrases or service tokens. Keep them as low-confidence findings
// so strict profile retains recall while default output hides the noise.
fn is_human_readable_slug(value: &str) -> bool {
    let has_separator = value.contains('-') || value.contains('_');
    has_separator
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c == '_')
        && value.split(['-', '_']).all(|segment| !segment.is_empty())
}

pub(crate) fn secret_literal_confidence(value: &str) -> Option<Confidence> {
    let value = value.trim().trim_end_matches([',', ';']).trim();
    let unquoted = unwrap_first_quoted(value);
    let lower = unquoted.to_lowercase();

    const KNOWN_PLACEHOLDERS: &[&str] = &[
        "null",
        "nil",
        "none",
        "your_key_here",
        "short",
        "password",
        "test-password",
        "test_password",
        "dummy",
        "example",
        "changeme",
        "replace_me",
        "placeholder",
        "todo",
        "xxx",
        "your_secret",
        "your_token",
        "your_api_key",
        "insert_here",
        "set_me",
        "fixme",
    ];
    let is_placeholder = unquoted.is_empty()
        || is_env_var_reference(unquoted)
        || is_shell_expansion(unquoted)
        || looks_like_env_var_name(unquoted)
        || unquoted.starts_with("${")
        || unquoted.starts_with("{{")
        || unquoted.starts_with('<')
        || unquoted.starts_with('[')
        || unquoted.ends_with('…')
        || unquoted.ends_with("...")
        || looks_like_placeholder_value(unquoted)
        || KNOWN_PLACEHOLDERS.contains(&lower.as_str());

    // Known non-secret values: algorithm names, auth scheme keywords, etc.
    const NON_SECRET_VALUES: &[&str] = &[
        "bearer", "rs256", "hs256", "es256", "rs512", "hs512", "es512", "sha256", "sha512", "md5",
        "aes", "aes256", "rsa", "hmac", "basic", "digest", "oauth", "oauth2", "true", "false",
    ];

    if is_placeholder || NON_SECRET_VALUES.contains(&lower.as_str()) || unquoted.len() < 8 {
        return None;
    }

    // Low-entropy strings (repetitive English words) are not real secrets.
    // Real API keys/tokens have high character diversity (entropy > 3.0 bits/char).
    if shannon_entropy(unquoted) < 3.0 {
        return None;
    }

    if is_human_readable_slug(unquoted) {
        return Some(Confidence::Low);
    }

    if value.starts_with('"') || value.starts_with('\'') {
        return Some(Confidence::High);
    }

    is_unquoted_secret_token(unquoted).then_some(Confidence::High)
}

fn looks_like_placeholder_value(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return true;
    }

    if lower.starts_with("replace-with-")
        || lower.starts_with("replace_with_")
        || lower.starts_with("your-")
        || lower.starts_with("your_")
        || lower.starts_with("example-")
        || lower.starts_with("example_")
        || lower.starts_with("sample-")
        || lower.starts_with("sample_")
        || lower.starts_with("fake-")
        || lower.starts_with("fake_")
        || lower.starts_with("dummy-")
        || lower.starts_with("dummy_")
        || lower.starts_with("placeholder-")
        || lower.starts_with("placeholder_")
    {
        return true;
    }

    let compact = compact_secret_name(&lower);
    if compact.is_empty() {
        return true;
    }

    const PLACEHOLDER_PREFIXES: &[&str] = &[
        "your",
        "replacewith",
        "replace",
        "example",
        "sample",
        "dummy",
        "fake",
        "mock",
        "demo",
        "placeholder",
        "insert",
        "set",
    ];

    PLACEHOLDER_PREFIXES
        .iter()
        .any(|prefix| compact.starts_with(prefix) && looks_like_secret_name(&compact))
}

fn is_unquoted_secret_token(value: &str) -> bool {
    if value.chars().any(|c| {
        c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '?' | '<' | '>')
    }) {
        return false;
    }

    if has_known_secret_prefix(value) {
        return true;
    }

    if looks_like_code_reference(value) {
        return false;
    }

    let has_letter = value.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    let has_symbol = value.chars().any(|c| !c.is_ascii_alphanumeric());

    value.len() >= 16 && has_letter && (has_digit || has_symbol)
}

fn is_env_var_reference(value: &str) -> bool {
    let value = value.trim();
    if let Some(name) = value.strip_prefix('$') {
        let name = name
            .strip_prefix('{')
            .and_then(|name| name.strip_suffix('}'))
            .unwrap_or(name);
        return !name.is_empty()
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    }

    false
}

fn has_known_secret_prefix(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("sk_")
        || lower.starts_with("sk_live_")
        || lower.starts_with("sk_test_")
        || lower.starts_with("ghp_")
        || lower.starts_with("gho_")
        || lower.starts_with("ghu_")
        || lower.starts_with("ghs_")
        || lower.starts_with("github_pat_")
        || lower.starts_with("xoxb-")
        || lower.starts_with("xoxp-")
        || lower.starts_with("ya29.")
        || value.starts_with("AKIA")
        || value.starts_with("ASIA")
        || value.starts_with("AIza")
}

fn looks_like_code_reference(value: &str) -> bool {
    let value = value
        .trim_start_matches('&')
        .trim_start_matches('*')
        .trim_start_matches('$');
    let has_path_separator = value.contains('.') || value.contains("::");
    let parts: Vec<&str> = if value.contains("::") {
        value.split("::").collect()
    } else {
        value.split('.').collect()
    };

    if parts.is_empty() || !parts.iter().all(|part| is_identifier(part)) {
        return false;
    }

    if has_path_separator || looks_like_secret_name(value) || value.contains('_') {
        return true;
    }

    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    let is_lowercase_word = value
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let is_uppercase_name = value
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');

    !has_digit && (is_lowercase_word || is_uppercase_name)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn looks_like_secret_name(value: &str) -> bool {
    let compact_value = compact_secret_name(value);
    SECRET_KEYS.iter().any(|key| {
        let compact_key = compact_secret_name(key);
        compact_value.contains(&compact_key)
    })
}

fn compact_secret_name(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let len = s.len() as f64;
    let mut freq = [0u32; 256];
    for &b in s.as_bytes() {
        freq[b as usize] += 1;
    }
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}
