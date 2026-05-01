use std::fs;
use std::io;
use std::path::Path;

use super::types::ScanSummary;

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
        }

        if metadata.is_dir() {
            summary.directories_count += 1;
            let entry_path = entry.path();
            scan_directory(&entry_path, summary)?;
        }
    }

    Ok(())
}
