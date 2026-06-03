use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::behavioral::{BehavioralKind, BehavioralSignal};
use crate::review::signals::content::ReviewSource;
use tree_sitter::{Node, Tree};

/// Detects newly removed behavioral signals (error handling, auth check, test deleted or emptied).
pub fn detect_behavioral_removed(
    file: &ChangedFile,
    pre_source: Option<&ReviewSource>,
    post_source: Option<&ReviewSource>,
) -> Vec<BehavioralSignal> {
    let mut signals = Vec::new();
    let path_str = file.path_string();

    // 1. Check TestDeletedOrEmptied
    let is_test = crate::audits::context::classify::helpers::is_test_file(&file.path, false);
    if is_test {
        if file.status == ChangeStatus::Deleted {
            signals.push(BehavioralSignal {
                kind: BehavioralKind::TestDeletedOrEmptied,
                path: path_str.clone(),
                line: 1,
                detail: "Test file was deleted".to_string(),
            });
            return signals;
        } else if file.status == ChangeStatus::Modified || file.status == ChangeStatus::Renamed {
            let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let pre_tests = pre_source.and_then(|s| count_test_cases(s, ext));
            let post_tests = post_source.and_then(|s| count_test_cases(s, ext));
            if let (Some(pre), Some(post)) = (pre_tests, post_tests) {
                if pre > 0 && post == 0 {
                    signals.push(BehavioralSignal {
                        kind: BehavioralKind::TestDeletedOrEmptied,
                        path: path_str.clone(),
                        line: 1,
                        detail: "All test cases were removed from test file".to_string(),
                    });
                }
            } else if let Some(post_src) = post_source {
                if post_src.content().trim().is_empty() {
                    signals.push(BehavioralSignal {
                        kind: BehavioralKind::TestDeletedOrEmptied,
                        path: path_str.clone(),
                        line: 1,
                        detail: "Test file was emptied".to_string(),
                    });
                }
            }
        }
    }

    if matches!(file.status, ChangeStatus::Added | ChangeStatus::Untracked) {
        return signals;
    }

    let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut ast_success = false;

    if let (Some(pre_src), Some(post_src)) = (pre_source, post_source) {
        if let (Some(pre_tree), Some(post_tree)) =
            (pre_src.parsed().tree(), post_src.parsed().tree())
        {
            let pre_try = count_try_blocks(pre_tree, pre_src.content(), file, ext, true);
            let post_try = count_try_blocks(post_tree, post_src.content(), file, ext, false);

            if pre_try > post_try {
                signals.push(BehavioralSignal {
                    kind: BehavioralKind::ErrorHandlingRemoved,
                    path: path_str.clone(),
                    line: get_first_removed_line(file).unwrap_or(1),
                    detail: format!(
                        "Error handling block(s) removed (try/catch blocks: {} -> {})",
                        pre_try, post_try
                    ),
                });
            }

            let pre_auth = count_auth_checks(pre_tree, pre_src.content(), file, ext, true);
            let post_auth = count_auth_checks(post_tree, post_src.content(), file, ext, false);

            if pre_auth > post_auth {
                signals.push(BehavioralSignal {
                    kind: BehavioralKind::AuthCheckRemoved,
                    path: path_str.clone(),
                    line: get_first_removed_line(file).unwrap_or(1),
                    detail: format!(
                        "Authentication/authorization check removed (auth calls: {} -> {})",
                        pre_auth, post_auth
                    ),
                });
            }

            ast_success = true;
        }
    }

    if !ast_success {
        let mut try_removed = false;
        let mut auth_removed = false;

        for hunk in &file.hunks {
            for line in &hunk.removed_lines {
                let line_lower = line.to_lowercase();
                if line_lower.contains("catch")
                    || line_lower.contains("except:")
                    || line_lower.contains("except ")
                {
                    let added_has_catch = hunk.added_lines.iter().any(|l| {
                        let l_low = l.to_lowercase();
                        l_low.contains("catch")
                            || l_low.contains("except:")
                            || l_low.contains("except ")
                    });
                    if !added_has_catch {
                        try_removed = true;
                    }
                }
                if line_lower.contains("auth")
                    || line_lower.contains("login")
                    || line_lower.contains("jwt")
                    || line_lower.contains("permission")
                {
                    let added_has_auth = hunk.added_lines.iter().any(|l| {
                        let l_low = l.to_lowercase();
                        l_low.contains("auth")
                            || l_low.contains("login")
                            || l_low.contains("jwt")
                            || l_low.contains("permission")
                    });
                    if !added_has_auth {
                        auth_removed = true;
                    }
                }
            }
        }

        if try_removed {
            signals.push(BehavioralSignal {
                kind: BehavioralKind::ErrorHandlingRemoved,
                path: path_str.clone(),
                line: get_first_removed_line(file).unwrap_or(1),
                detail: "Error handling removed (coarse fallback)".to_string(),
            });
        }
        if auth_removed {
            signals.push(BehavioralSignal {
                kind: BehavioralKind::AuthCheckRemoved,
                path: path_str.clone(),
                line: get_first_removed_line(file).unwrap_or(1),
                detail: "Authentication/authorization check removed (coarse fallback)".to_string(),
            });
        }
    }

    signals
}

fn count_test_cases(source: &ReviewSource, ext: &str) -> Option<usize> {
    let parsed = source.parsed();
    let tree = parsed.tree()?;
    let content = source.content();
    let mut count = 0;
    walk_tests(tree.root_node(), content, ext, &mut count);
    Some(count)
}

fn walk_tests(node: Node<'_>, content: &str, ext: &str, count: &mut usize) {
    let kind = node.kind();
    let text = node.utf8_text(content.as_bytes()).unwrap_or("");

    match ext {
        "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => {
            if kind == "call_expression" {
                if let Some(callee) = node.child_by_field_name("function") {
                    if let Ok(callee_text) = callee.utf8_text(content.as_bytes()) {
                        let val = callee_text.trim();
                        if val == "test" || val == "it" || val == "describe" {
                            *count += 1;
                        }
                    }
                }
            }
        }
        "py" => {
            if kind == "function_definition" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(name_text) = name_node.utf8_text(content.as_bytes()) {
                        if name_text.starts_with("test_") {
                            *count += 1;
                        }
                    }
                }
            }
        }
        "go" => {
            if kind == "function_declaration" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(name_text) = name_node.utf8_text(content.as_bytes()) {
                        if name_text.starts_with("Test") {
                            *count += 1;
                        }
                    }
                }
            }
        }
        "rs" => {
            if kind == "function_item" && text.contains("#[test]") {
                *count += 1;
            }
        }
        "java" | "kt" | "kts" => {
            if (kind == "method_declaration" || kind == "function_declaration")
                && text.contains("@Test")
            {
                *count += 1;
            }
        }
        "cs" if kind == "method_declaration"
            && (text.contains("[Test]")
                || text.contains("[TestMethod]")
                || text.contains("[Fact]")) =>
        {
            *count += 1;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tests(child, content, ext, count);
    }
}

fn count_try_blocks(
    tree: &Tree,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
) -> usize {
    let mut count = 0;
    walk_try(tree.root_node(), content, file, ext, is_pre, &mut count);
    count
}

fn walk_try(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
    count: &mut usize,
) {
    let line = node.start_position().row + 1;
    let inside = if is_pre {
        file.hunks.iter().any(|h| {
            if let Some(r) = h.old_range {
                line >= r.start && line <= r.end
            } else {
                false
            }
        })
    } else {
        file.contains_line(line)
    };

    if inside {
        let kind = node.kind();
        let text = node.utf8_text(content.as_bytes()).unwrap_or("");

        let is_try = match ext {
            "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => kind == "try_statement",
            "py" => kind == "try_statement",
            "java" | "cs" => kind == "try_statement",
            "kt" | "kts" => kind == "try_expression",
            "go" => kind == "if_statement" && text.contains("err != nil"),
            "rs" => {
                (kind == "match_expression" && text.contains("Err("))
                    || (kind == "if_let_expression" && text.contains("Err("))
                    || (kind == "call_expression"
                        && (text.contains(".unwrap_or") || text.contains(".map_err")))
            }
            _ => false,
        };
        if is_try {
            *count += 1;
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_try(child, content, file, ext, is_pre, count);
    }
}

fn count_auth_checks(
    tree: &Tree,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
) -> usize {
    let mut count = 0;
    walk_auth(tree.root_node(), content, file, ext, is_pre, &mut count);
    count
}

fn walk_auth(
    node: Node<'_>,
    content: &str,
    file: &ChangedFile,
    ext: &str,
    is_pre: bool,
    count: &mut usize,
) {
    let line = node.start_position().row + 1;
    let inside = if is_pre {
        file.hunks.iter().any(|h| {
            if let Some(r) = h.old_range {
                line >= r.start && line <= r.end
            } else {
                false
            }
        })
    } else {
        file.contains_line(line)
    };

    if inside {
        let kind = node.kind();
        let is_call = match ext {
            "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" => {
                kind == "call_expression"
            }
            "py" => kind == "call",
            "go" => kind == "call_expression",
            "rs" => kind == "call_expression",
            "java" | "kt" | "kts" => kind == "method_invocation" || kind == "call_expression",
            "cs" => kind == "invocation_expression",
            _ => false,
        };
        if is_call {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                let text_lower = text.to_lowercase();
                if text_lower.contains("auth")
                    || text_lower.contains("login")
                    || text_lower.contains("jwt")
                    || text_lower.contains("permission")
                    || text_lower.contains("session")
                    || text_lower.contains("role")
                {
                    *count += 1;
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_auth(child, content, file, ext, is_pre, count);
    }
}

fn get_first_removed_line(file: &ChangedFile) -> Option<usize> {
    file.hunks.iter().find_map(|h| h.old_range.map(|r| r.start))
}
