use crate::scan::types::{Marker, MarkerKind};
use std::path::Path;

pub fn detect_markers(path: &Path, content: &str) -> Vec<Marker> {
    let mut markers = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        if line.contains("TODO") {
            markers.push(Marker {
                kind: MarkerKind::Todo,
                line_number,
                path: path.to_path_buf(),
                text: line.to_string(),
            });
        }
        if line.contains("FIXME") {
            markers.push(Marker {
                kind: MarkerKind::Fixme,
                line_number,
                path: path.to_path_buf(),
                text: line.to_string(),
            });
        }
        if line.contains("HACK") {
            markers.push(Marker {
                kind: MarkerKind::Hack,
                line_number,
                path: path.to_path_buf(),
                text: line.to_string(),
            });
        }
    }

    markers
}
