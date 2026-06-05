use super::keywords;
use super::removed_ast::{count_auth_checks, count_test_cases, count_try_blocks};
use crate::review::diff::{ChangeStatus, ChangedFile};
use crate::review::signals::behavioral::{
    BehavioralKind, BehavioralSignal, BehavioralSignalSource,
};
use crate::review::signals::content::ReviewSource;

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
                source: BehavioralSignalSource::Ast,
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
                        source: BehavioralSignalSource::Ast,
                    });
                }
            } else if let Some(post_src) = post_source {
                if post_src.content().trim().is_empty() {
                    signals.push(BehavioralSignal {
                        kind: BehavioralKind::TestDeletedOrEmptied,
                        path: path_str.clone(),
                        line: 1,
                        detail: "Test file was emptied".to_string(),
                        source: BehavioralSignalSource::Ast,
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
                    source: BehavioralSignalSource::Ast,
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
                    source: BehavioralSignalSource::Ast,
                });
            }

            ast_success = true;
        }
    }

    if !ast_success {
        // Coarse fallback: no parse tree, so scan raw diff lines. Token matching
        // (not substring) keeps `catch` off `catchy` and `auth` off `author`.
        // Signals carry `BehavioralSignalSource::CoarseFallback` so the tiered
        // view demotes them — confidence keys off the source, not the detail text.
        let mut try_removed = false;
        let mut auth_removed = false;

        for hunk in &file.hunks {
            let added_has_error = hunk
                .added_lines
                .iter()
                .any(|line| keywords::line_has_error_handling(line));
            let added_has_auth = hunk
                .added_lines
                .iter()
                .any(|line| keywords::line_has_auth(line));
            for line in &hunk.removed_lines {
                if !added_has_error && keywords::line_has_error_handling(line) {
                    try_removed = true;
                }
                if !added_has_auth && keywords::line_has_auth(line) {
                    auth_removed = true;
                }
            }
        }

        if try_removed {
            signals.push(BehavioralSignal {
                kind: BehavioralKind::ErrorHandlingRemoved,
                path: path_str.clone(),
                line: get_first_removed_line(file).unwrap_or(1),
                detail: "Error handling removed (coarse fallback)".to_string(),
                source: BehavioralSignalSource::CoarseFallback,
            });
        }
        if auth_removed {
            signals.push(BehavioralSignal {
                kind: BehavioralKind::AuthCheckRemoved,
                path: path_str.clone(),
                line: get_first_removed_line(file).unwrap_or(1),
                detail: "Authentication/authorization check removed (coarse fallback)".to_string(),
                source: BehavioralSignalSource::CoarseFallback,
            });
        }
    }

    signals
}

fn get_first_removed_line(file: &ChangedFile) -> Option<usize> {
    file.hunks.iter().find_map(|h| h.old_range.map(|r| r.start))
}
