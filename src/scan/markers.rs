use crate::scan::types::{CodeMarker, MarkerKind};
use std::path::Path;

pub fn detect_markers(path: &Path, content: &str) -> Vec<CodeMarker> {
    let mut markers = Vec::new();

    for (index, line) in content.lines().enumerate() {
        if line.contains("TODO") {
            markers.push(build_marker(path, index, line, MarkerKind::Todo));
        }

        if line.contains("FIXME") {
            markers.push(build_marker(path, index, line, MarkerKind::Fixme));
        }

        if line.contains("HACK") {
            markers.push(build_marker(path, index, line, MarkerKind::Hack));
        }
    }

    markers
}

fn build_marker(path: &Path, index: usize, line: &str, kind: MarkerKind) -> CodeMarker {
    CodeMarker {
        kind,
        path: path.to_path_buf(),
        line_number: index + 1,
        text: line.to_string(),
    }
}
