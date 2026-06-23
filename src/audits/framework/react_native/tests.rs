use super::*;
use crate::audits::traits::ProjectAudit;
use crate::findings::types::Severity;
use crate::frameworks::ReactNativeArchitectureProfile;
use crate::scan::config::ScanConfig;
use crate::scan::facts::{FileFacts, ScanFacts};
use std::io::Write;
use tempfile::tempdir;

fn facts_for(root: &std::path::Path) -> ScanFacts {
    ScanFacts {
        root_path: root.to_path_buf(),
        react_native: Some(ReactNativeArchitectureProfile {
            detected: true,
            ..ReactNativeArchitectureProfile::default()
        }),
        ..ScanFacts::default()
    }
}

fn jsx_file(dir: &tempfile::TempDir, name: &str, content: &str) -> FileFacts {
    let path = dir.path().join(name);
    write!(std::fs::File::create(&path).unwrap(), "{content}").unwrap();
    FileFacts {
        path,
        language: Some("TypeScript React".to_string()),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: vec![],
        content: None,
        has_inline_tests: false,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    }
}

// ── Old architecture ──────────────────────────────────────────────────────

include!("tests/group_1.rs");
include!("tests/group_2.rs");
include!("tests/group_3.rs");
