use std::fs;
use std::path::Path;

const FORBIDDEN_SCAN_CALLS: &[&str] = &[
    "run_product_scan(",
    "scan_path(",
    "scan_path_with_config(",
    "scan_changed_with_config(",
    "scan_with_config(",
];

#[test]
fn renderers_do_not_trigger_repository_scans() {
    for dir in ["src/output", "src/report", "src/review/render", "src/ai"] {
        assert_no_scan_calls_in_dir(Path::new(dir));
    }
}

#[test]
fn engine_pipeline_docs_exist() {
    let docs = fs::read_to_string("docs/engineering/engine-pipeline.md")
        .expect("engine pipeline docs should exist");

    assert!(docs.contains("Renderers must never trigger a repository scan"));
    assert!(docs.contains("discovery"));
    assert!(docs.contains("risk scoring"));
}

fn assert_no_scan_calls_in_dir(dir: &Path) {
    if !dir.exists() {
        return;
    }

    for path in rust_files(dir) {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

        for forbidden in FORBIDDEN_SCAN_CALLS {
            assert!(
                !content.contains(forbidden),
                "renderer/report-like module {} must not call scan helper `{}`",
                path.display(),
                forbidden
            );
        }
    }
}

fn rust_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    collect_rust_files(root, &mut out);
    out
}

fn collect_rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
