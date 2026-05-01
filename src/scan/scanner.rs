use std::fs;
use std::io;
use std::path::Path;

use super::types::ScanSummary;

const IGNORED_DIRECTORIES: &[&str] = &[".git", "target", "node_modules", "dist", "build", ".next"];
const SUPPORTED_TEXT_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "md", "toml", "json", "yaml", "yml", "html", "css",
    "scss",
];

pub fn scan_path(path: &Path) -> io::Result<ScanSummary> {
    let mut summary = ScanSummary::default();

    scan_directory(path, &mut summary)?;

    Ok(summary)
}

fn scan_directory(path: &Path, summary: &mut ScanSummary) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            summary.files_count += 1;

            if is_supported_text_file(&entry.path()) {
                let count_file_lines = count_file_lines(&entry.path())?;
                summary.total_lines += count_file_lines;
                summary.text_files_count += 1;
            }
        }

        if metadata.is_dir() {
            let entry_path = entry.path();

            if is_ignored_directory(&entry_path) {
                continue;
            }

            summary.directories_count += 1;
            scan_directory(&entry_path, summary)?;
        }
    }

    Ok(())
}

fn is_ignored_directory(path: &Path) -> bool {
    let Some(directory_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    IGNORED_DIRECTORIES.contains(&directory_name)
}

fn count_file_lines(path: &Path) -> io::Result<usize> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().count())
}

fn is_supported_text_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    SUPPORTED_TEXT_EXTENSIONS.contains(&extension)
}
