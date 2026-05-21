fn git_diff_against_head(repo_root: &Path, pathspec: Option<&str>) -> Result<String, GitDiffError> {
    let mut args = vec!["diff", "--unified=0", "--no-ext-diff", "HEAD", "--"];
    if let Some(pathspec) = pathspec {
        args.push(pathspec);
    }

    git_output(repo_root, &args, "git diff --unified=0 --no-ext-diff HEAD")
}

fn git_diff_between_refs(
    repo_root: &Path,
    base: &str,
    head: &str,
    pathspec: Option<&str>,
) -> Result<String, GitDiffError> {
    let range = format!("{base}...{head}");
    let mut args = vec!["diff", "--unified=0", "--no-ext-diff", range.as_str(), "--"];
    if let Some(pathspec) = pathspec {
        args.push(pathspec);
    }

    git_output(
        repo_root,
        &args,
        &format!("git diff --unified=0 --no-ext-diff {range}"),
    )
}

fn load_untracked_files(
    repo_root: &Path,
    pathspec: Option<&str>,
) -> Result<Vec<ChangedFile>, GitDiffError> {
    let mut args = vec!["ls-files", "--others", "--exclude-standard", "-z", "--"];
    if let Some(pathspec) = pathspec {
        args.push(pathspec);
    }

    let output = git_output(repo_root, &args, "git ls-files --others --exclude-standard")?;

    output
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| {
            let line_count = fs::read_to_string(repo_root.join(path))
                .map(|content| content.lines().count())
                .unwrap_or(0);
            let ranges = if line_count == 0 {
                Vec::new()
            } else {
                vec![ChangedRange {
                    start: 1,
                    end: line_count,
                }]
            };

            Ok(ChangedFile {
                path: PathBuf::from(path),
                status: ChangeStatus::Untracked,
                ranges,
            })
        })
        .collect()
}

fn is_repopilot_internal_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized == ".repopilot" || normalized.starts_with(".repopilot/")
}

fn git_output(cwd: &Path, args: &[&str], command_label: &str) -> Result<String, GitDiffError> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;

    if !output.status.success() {
        return Err(GitDiffError::GitCommandFailed {
            command: command_label.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_diff_git_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let old_path = normalize_diff_path(parts.next()?)?;
    let new_path = normalize_diff_path(parts.next()?)?;

    Some((old_path, new_path))
}

fn normalize_diff_path(path: &str) -> Option<String> {
    let path = path.trim();

    if path == "/dev/null" {
        return None;
    }

    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);

    Some(path.trim_matches('"').replace('\\', "/"))
}

fn parse_hunk_added_range(line: &str) -> Option<ChangedRange> {
    let range = line.split_once(" +")?.1.split_once(" @@")?.0;
    let mut parts = range.split(',');
    let start = parts.next()?.parse::<usize>().ok()?;
    let count = parts
        .next()
        .and_then(|count| count.parse::<usize>().ok())
        .unwrap_or(1);

    if count == 0 {
        return None;
    }

    Some(ChangedRange {
        start,
        end: start + count - 1,
    })
}
