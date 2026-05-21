fn assigned_value_after_key(after_key: &str) -> Option<&str> {
    let trimmed = after_key.trim_start();

    if let Some(value) = trimmed.strip_prefix('=') {
        return Some(value.trim_start());
    }

    let after_colon = trimmed.strip_prefix(':')?.trim_start();
    if let Some((_, value)) = after_colon.split_once('=') {
        return Some(value.trim_start());
    }

    Some(after_colon)
}

fn is_secret_literal(value: &str) -> bool {
    let value = value.trim().trim_end_matches([',', ';']).trim();
    let unquoted = value.trim_matches('"').trim_matches('\'');
    let lower = unquoted.to_lowercase();

    let is_placeholder = unquoted.is_empty()
        || is_env_var_reference(unquoted)
        || unquoted.starts_with("${")
        || unquoted.starts_with("{{")
        || unquoted.starts_with('<')
        || unquoted.starts_with('[')
        || unquoted.ends_with('…')
        || unquoted.ends_with("...")
        || matches!(
            lower.as_str(),
            "null"
                | "nil"
                | "none"
                | "your_key_here"
                | "short"
                | "password"
                | "test-password"
                | "test_password"
                | "dummy"
                | "example"
                | "changeme"
                | "replace_me"
                | "placeholder"
                | "todo"
                | "xxx"
                | "your_secret"
                | "your_token"
                | "your_api_key"
                | "insert_here"
                | "set_me"
                | "fixme"
        );

    // Known non-secret values: algorithm names, auth scheme keywords, etc.
    const NON_SECRET_VALUES: &[&str] = &[
        "bearer", "rs256", "hs256", "es256", "rs512", "hs512", "es512", "sha256", "sha512", "md5",
        "aes", "aes256", "rsa", "hmac", "basic", "digest", "oauth", "oauth2", "true", "false",
    ];

    if is_placeholder || NON_SECRET_VALUES.contains(&lower.as_str()) || unquoted.len() < 8 {
        return false;
    }

    // Low-entropy strings (repetitive English words) are not real secrets.
    // Real API keys/tokens have high character diversity (entropy > 3.0 bits/char).
    if shannon_entropy(unquoted) < 3.0 {
        return false;
    }

    if value.starts_with('"') || value.starts_with('\'') {
        return true;
    }

    is_unquoted_secret_token(unquoted)
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

/// Computes Shannon entropy in bits per character.
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

fn build_finding(
    line_number: usize,
    path: &std::path::Path,
    matched_key: &str,
    snippet: String,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "security.secret-candidate".to_string(),
        recommendation: Finding::recommendation_for_rule_id("security.secret-candidate"),
        title: "Possible secret detected".to_string(),
        description: format!(
            "Line {line_number} in `{}` looks like it may contain a hardcoded secret (matched key: `{matched_key}`). Review and move to environment variables or a secrets manager.",
            path.display()
        ),
        category: FindingCategory::Security,
        severity: Severity::High,
        confidence: Confidence::High,
        evidence: vec![Evidence {
            path: path.to_path_buf(),
            line_start: line_number,
            line_end: None,
            snippet,
        }],
        workspace_package: None,
        docs_url: None,
        provenance: Default::default(),
        risk: Default::default(),
    }
}

fn find_jwt_like_token(line: &str) -> Option<&str> {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '.')
        .find(|candidate| is_jwt_like_token(candidate))
}

fn is_jwt_like_token(candidate: &str) -> bool {
    if candidate.len() < 40 || !candidate.starts_with("eyJ") {
        return false;
    }

    let parts: Vec<_> = candidate.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| part.len() >= 8 && part.chars().all(is_base64url_char))
}

fn is_base64url_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn mask_token_in_line(line: &str, token: &str) -> String {
    let visible_prefix = token.chars().take(8).collect::<String>();
    line.replace(token, &format!("{visible_prefix}...***"))
}

fn mask_secret_value(line: &str) -> String {
    // Find the first = or : assignment and mask everything after the first 3 chars of value
    if let Some(pos) = line.find('=').or_else(|| line.find(':')) {
        let (key_part, value_part) = line.split_at(pos + 1);
        let value = value_part.trim().trim_matches('"').trim_matches('\'');
        if value.len() > 3 {
            return format!("{key_part} \"{}...***\"", &value[..3]);
        }
    }
    format!("{line} [value masked]")
}
