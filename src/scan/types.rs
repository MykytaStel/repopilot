#[derive(Debug, Default)]
pub struct ScanSummary {
    pub files_count: usize,
    pub directories_count: usize,
    pub total_lines: usize,
}
