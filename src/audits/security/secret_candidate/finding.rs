// Included into `secret_candidate.rs` (see `include!`). Finding construction,
// value masking, and JWT-token detection — the output/format and token-shape
// helpers, kept separate from the generic value-classification heuristics in
// `helpers.rs`. Shares the parent module's imports and namespace.

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
            "Line {line_number} in `{}` looks like it may contain a hardcoded secret (matched key: `{matched_key}`). Move real credentials to environment variables or a secrets manager, and rotate the credential if it was committed.",
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
