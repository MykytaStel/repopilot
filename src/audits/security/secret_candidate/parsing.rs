// Structural pre-classification helpers for the secret scanner: extracting the
// real value out of a source assignment and skipping inline test regions,
// before the value-classification heuristics in `helpers.rs` run. This file is
// `include!`d into `secret_candidate.rs`, so it uses plain comments, not a `//!`
// module doc.

/// Returns the content of the first `"..."`/`'...'` segment in `value`, falling
/// back to trimming surrounding quotes. This survives a trailing token after the
/// closing quote — `"$PGPASSWORD" \` (a shell line continuation) becomes
/// `$PGPASSWORD`, and `"short".to_string()` becomes `short` — so placeholder and
/// shell-expansion detection see the real value instead of a fragment that
/// looks high-entropy.
fn unwrap_first_quoted(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(rest) = value.strip_prefix(quote)
            && let Some(end) = rest.find(quote)
        {
            return &rest[..end];
        }
    }
    value.trim_matches('"').trim_matches('\'')
}

/// True when the value is a shell command substitution (`$(...)`, backticks) or
/// begins one. A captured command output or expansion is not a literal secret.
fn is_shell_expansion(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("$(") || value.starts_with('`') || value.contains("$(")
}

/// Lines carrying the public search-only `apiKey` inside an Algolia DocSearch
/// config block. The signature trio keeps the downgrade scoped to that widget,
/// so unrelated `apiKey` credentials elsewhere in the file stay High.
fn algolia_public_api_key_lines(content: &str) -> HashSet<usize> {
    let lines: Vec<_> = content.lines().collect();
    let mut public_key_lines = HashSet::new();
    let mut index = 0;

    while index < lines.len() {
        if !lines[index].to_ascii_lowercase().contains("algolia") {
            index += 1;
            continue;
        }

        let end = algolia_config_block_end(&lines, index);
        let block = lines[index..=end].join("\n").to_ascii_lowercase();
        if block.contains("appid") && block.contains("indexname") {
            for (offset, line) in lines[index..=end].iter().enumerate() {
                let lower = line.to_ascii_lowercase();
                if ["apikey", "api_key"]
                    .iter()
                    .any(|key| assigned_secret_confidence_for_key(line, &lower, key).is_some())
                {
                    public_key_lines.insert(index + offset + 1);
                }
            }
        }

        index = end + 1;
    }

    public_key_lines
}

fn algolia_config_block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0;
    let mut opened = false;
    let mut in_block_comment = false;

    for (index, line) in lines
        .iter()
        .enumerate()
        .skip(start)
        .take(80)
    {
        let sanitized = sanitize_c_style(line, &mut in_block_comment);
        let opens = sanitized.matches('{').count() as i32;
        let closes = sanitized.matches('}').count() as i32;

        opened |= opens > 0;
        depth += opens - closes;

        if opened && depth <= 0 {
            return index;
        }
    }

    start
}

/// True when `matched_key` is the public-facing Algolia search key field. Only
/// the `apiKey` family is the published search-only key; other secret keys in the
/// same file are unaffected.
fn is_algolia_public_key(matched_key: &str) -> bool {
    matches!(matched_key, "apikey" | "api_key")
}

/// The 1-based line numbers that fall inside a `#[cfg(test)]`/`#[test]`-gated
/// item in a Rust source file.
///
/// Secret candidates in an inline `#[cfg(test)] mod tests` block are almost
/// always fixtures (`password: "short"`, a fake-but-high-entropy token), yet
/// the file-path test skip only catches whole *test files* — an inline test
/// module living in a production `.rs` file slips through. Returns an empty set
/// for non-Rust files. Brace depth is tracked over comment-sanitized lines; the
/// common `#[cfg(test)]\nmod tests { … }` idiom is matched precisely, and a
/// gated *declaration* without a body (`#[cfg(test)] mod helpers;`) does not
/// open a region.
fn rust_cfg_test_lines(file: &FileFacts) -> HashSet<usize> {
    let mut gated = HashSet::new();
    if file.path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return gated;
    }
    let Some(content) = file.content.as_deref() else {
        return gated;
    };

    let mut depth: i32 = 0;
    let mut pending = false;
    let mut gate_depth: Option<i32> = None;
    let mut in_block_comment = false;

    for (index, raw) in content.lines().enumerate() {
        let line_no = index + 1;
        let sanitized = sanitize_c_style(raw, &mut in_block_comment);
        let trimmed = sanitized.trim();

        if gate_depth.is_some() {
            gated.insert(line_no);
        } else if trimmed.starts_with("#[cfg(test)]") || trimmed == "#[test]" {
            pending = true;
        }

        let opens = sanitized.matches('{').count() as i32;
        let closes = sanitized.matches('}').count() as i32;

        if pending && gate_depth.is_none() {
            if opens > 0 {
                gate_depth = Some(depth);
                gated.insert(line_no);
                pending = false;
            } else if trimmed.ends_with(';') && !trimmed.starts_with("#[") {
                // A gated declaration with no body (`mod helpers;`, `use ...;`)
                // — nothing to skip, and it must not arm the next `{`.
                pending = false;
            }
        }

        depth += opens - closes;

        if let Some(gate) = gate_depth
            && depth <= gate
        {
            gate_depth = None;
        }
    }

    gated
}
