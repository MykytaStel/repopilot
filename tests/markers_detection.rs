use repopilot::scan::markers::detect_markers;
use repopilot::scan::types::MarkerKind;
use std::path::Path;

#[test]
fn detects_markers_with_line_numbers_and_evidence() {
    let content = "\
fn main() {}

// TODO: add better scanner
// FIXME: handle binary files
// HACK: temporary workaround
";

    let markers = detect_markers(Path::new("src/main.rs"), content);

    assert_eq!(markers.len(), 3);

    assert_eq!(markers[0].kind, MarkerKind::Todo);
    assert_eq!(markers[0].line_number, 3);
    assert_eq!(markers[0].path, Path::new("src/main.rs"));
    assert!(markers[0].text.contains("TODO"));

    assert_eq!(markers[1].kind, MarkerKind::Fixme);
    assert_eq!(markers[1].line_number, 4);

    assert_eq!(markers[2].kind, MarkerKind::Hack);
    assert_eq!(markers[2].line_number, 5);
}
