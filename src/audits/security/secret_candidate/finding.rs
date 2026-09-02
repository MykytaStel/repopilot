// Included into `secret_candidate.rs` (see `include!`). Finding construction,
// value masking, and JWT-token detection — the output/format and token-shape
// helpers, kept separate from the generic value-classification heuristics in
// `helpers.rs`. Shares the parent module's imports and namespace.

fn build_finding(
    line_number: usize,
    path: &std::path::Path,
    matched_key: &str,
    snippet: String,
    confidence: Confidence,
) -> Finding {
    Finding {
        id: String::new(),
        rule_id: "security.secret-candidate".to_string(),
        recommendation: Finding::recommendation_for_rule_id("security.secret-candidate"),
        title: "Possible secret detected".to_string(),
        description: format!(
            "Line {line_number} in `{}` looks like it may contain a hardcoded secret (matched key: `{matched_key}`). Move real credentials to environment variables or a secrets manager, and rotate the credential if it was committed.",
            path.display()
        ),
        category: FindingCategory::Security,
        severity: Severity::High,
        confidence,
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

fn mask_secret_value(line: &str, matched_key: &str) -> String {
    let lower_line = line.to_ascii_lowercase();
    let lower_key = matched_key.to_ascii_lowercase();
    let Some(key_start) = lower_line.find(&lower_key) else {
        return format!("{line} [value masked]");
    };

    let key_end = key_start + matched_key.len();
    let after_key = &line[key_end..];
    let whitespace = after_key.len() - after_key.trim_start().len();
    let mut value_start = key_end + whitespace;
    let assignment = &line[value_start..];
    if assignment.starts_with('=') {
        value_start += 1;
    } else if assignment.starts_with(':') {
        value_start += 1;
        let after_colon = &line[value_start..];
        value_start += after_colon.len() - after_colon.trim_start().len();
        if line[value_start..].starts_with('=') {
            value_start += 1;
        }
    } else {
        return format!("{line} [value masked]");
    }

    let value = line[value_start..].trim_start();
    value_start += line[value_start..].len() - value.len();
    let (value_end, quote) = match value.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let end = value[quote.len_utf8()..]
                .find(quote)
                .map(|offset| quote.len_utf8() + offset + quote.len_utf8())
                .unwrap_or(value.len());
            (value_start + end, Some(quote))
        }
        Some(_) => {
            let end = value
                .find(|character: char| {
                    character.is_whitespace() || matches!(character, ';' | ',' | '}')
                })
                .unwrap_or(value.len());
            (value_start + end, None)
        }
        None => return format!("{line} [value masked]"),
    };

    let raw_value = &line[value_start..value_end];
    let unquoted = quote
        .map(|quote| raw_value.trim_start_matches(quote).trim_end_matches(quote))
        .unwrap_or(raw_value);
    if unquoted.chars().count() <= 3 {
        return format!("{line} [value masked]");
    }

    let prefix = unquoted.chars().take(3).collect::<String>();
    let replacement = match quote {
        Some(quote) => format!("{quote}{prefix}...***{quote}"),
        None => format!("{prefix}...***"),
    };
    format!("{}{}{}", &line[..value_start], replacement, &line[value_end..])
}
