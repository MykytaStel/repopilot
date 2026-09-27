use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BatchedContent {
    Loaded(Option<String>),
    Fallback,
}

pub(crate) fn git_show_many(
    repo_root: &Path,
    reference: &str,
    paths: &[String],
) -> Option<Vec<BatchedContent>> {
    super::validate_git_ref(reference).ok()?;
    let mut results = std::iter::repeat_with(|| BatchedContent::Fallback)
        .take(paths.len())
        .collect::<Vec<_>>();
    let batch_paths = paths
        .iter()
        .enumerate()
        .filter(|(_, path)| !path.contains('\n'))
        .collect::<Vec<_>>();
    if batch_paths.is_empty() {
        return Some(results);
    }

    let contents = read_batch(repo_root, reference, &batch_paths)?;
    for ((index, _), content) in batch_paths.into_iter().zip(contents) {
        results[index] = BatchedContent::Loaded(content);
    }
    Some(results)
}

fn read_batch(
    repo_root: &Path,
    reference: &str,
    batch_paths: &[(usize, &String)],
) -> Option<Vec<Option<String>>> {
    let input = batch_paths
        .iter()
        .map(|(_, path)| format!("{reference}:{path}\n"))
        .collect::<String>()
        .into_bytes();
    let paths = batch_paths
        .iter()
        .map(|(_, path)| (*path).clone())
        .collect::<Vec<_>>();
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let stdout = child.stdout.take()?;
    let parsed = parse_batch_output(BufReader::new(stdout), reference, &paths);
    if parsed.is_none() {
        let _ = child.kill();
    }
    let status = child.wait().ok()?;
    let write_result = writer.join().ok()?;
    if !status.success() || write_result.is_err() {
        return None;
    }
    parsed
}

fn parse_batch_output(
    mut reader: impl BufRead,
    reference: &str,
    paths: &[String],
) -> Option<Vec<Option<String>>> {
    let mut results = Vec::with_capacity(paths.len());
    for path in paths {
        results.push(read_batch_entry(&mut reader, reference, path)?);
    }

    let mut trailing = [0; 1];
    (reader.read(&mut trailing).ok()? == 0).then_some(results)
}

fn read_batch_entry(
    reader: &mut impl BufRead,
    reference: &str,
    path: &str,
) -> Option<Option<String>> {
    let mut header = Vec::new();
    if reader.read_until(b'\n', &mut header).ok()? == 0 || header.pop() != Some(b'\n') {
        return None;
    }
    let query = format!("{reference}:{path}");
    if let Some(missing) = header.strip_suffix(b" missing")
        && missing == query.as_bytes()
    {
        return Some(None);
    }

    let mut fields = std::str::from_utf8(&header).ok()?.split_ascii_whitespace();
    fields.next()?;
    let object_type = fields.next()?;
    let size = fields.next()?.parse::<usize>().ok()?;
    if fields.next().is_some() {
        return None;
    }
    let content = read_batch_content(reader, size)?;
    if object_type == "blob" {
        Some(Some(String::from_utf8_lossy(&content).into_owned()))
    } else {
        Some(None)
    }
}

fn read_batch_content(reader: &mut impl BufRead, size: usize) -> Option<Vec<u8>> {
    let mut content = Vec::with_capacity(size);
    let bytes_read = reader
        .take(u64::try_from(size).ok()?)
        .read_to_end(&mut content)
        .ok()?;
    if bytes_read != size {
        return None;
    }
    let mut separator = [0; 1];
    reader.read_exact(&mut separator).ok()?;
    (separator == [b'\n']).then_some(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::git_show;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn batch_reads_match_git_show_and_marks_missing_paths() {
        let temp = TempDir::new().expect("temporary repository");
        let root = temp.path();
        git(root, &["init", "--quiet"]);
        git(root, &["config", "user.email", "repopilot@example.test"]);
        git(root, &["config", "user.name", "RepoPilot tests"]);
        fs::create_dir_all(root.join("src")).expect("create source directory");
        fs::write(root.join("src/with spaces.rs"), "pub fn spaced() {}\n")
            .expect("write spaced path");
        fs::write(root.join("src/line\nbreak.rs"), "pub fn unusual() {}\n")
            .expect("write newline path");
        git(root, &["add", "--all"]);
        git(root, &["commit", "--quiet", "-m", "base"]);

        let normal = "src/with spaces.rs".to_string();
        let unusual = "src/line\nbreak.rs".to_string();
        let missing = "src/missing.rs".to_string();
        let paths = vec![normal.clone(), unusual.clone(), missing.clone()];
        let loaded = git_show_many(root, "HEAD", &paths).expect("batch command succeeds");

        assert_eq!(
            loaded,
            vec![
                BatchedContent::Loaded(git_show(root, "HEAD", &normal)),
                BatchedContent::Fallback,
                BatchedContent::Loaded(None),
            ]
        );
    }

    #[test]
    fn invalid_reference_returns_no_batch_result() {
        let temp = TempDir::new().expect("temporary repository");
        let result = git_show_many(temp.path(), "--help", &["file.rs".to_string()]);
        assert!(result.is_none());
    }
}
