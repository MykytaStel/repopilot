use crate::scan::types::{Marker, MarkerKind};
use std::path::Path;

pub fn detect_markers(path: &Path, content: &str) -> Vec<Marker> {
    let mut markers = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let Some(comment_text) = comment_text(line) else {
            continue;
        };

        let line_number = index + 1;
        if comment_text.contains("TODO") {
            markers.push(Marker {
                kind: MarkerKind::Todo,
                line_number,
                path: path.to_path_buf(),
                text: line.to_string(),
            });
        }
        if comment_text.contains("FIXME") {
            markers.push(Marker {
                kind: MarkerKind::Fixme,
                line_number,
                path: path.to_path_buf(),
                text: line.to_string(),
            });
        }
        if comment_text.contains("HACK") {
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

fn comment_text(line: &str) -> Option<&str> {
    const COMMENT_MARKERS: &[&str] = &["//", "#", "--", "/*", "*", "<!--"];

    COMMENT_MARKERS
        .iter()
        .filter_map(|marker| line.find(marker).map(|index| (index, marker.len())))
        .min_by_key(|(index, _)| *index)
        .map(|(index, marker_len)| &line[index + marker_len..])
}
