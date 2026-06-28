use super::*;
use crate::audits::context::model::{
    FileRole, FrameworkKind, LanguageKind, ProgrammingParadigm, RoleEvidenceSource, RuntimeKind,
};
use crate::scan::facts::FileFacts;
use std::path::PathBuf;

include!("tests/group_1.rs");
include!("tests/group_2.rs");
include!("tests/group_3.rs");

fn facts(path: &str, language: Option<&str>, content: &str, has_inline_tests: bool) -> FileFacts {
    FileFacts {
        path: PathBuf::from(path),
        language: language.map(str::to_string),
        non_empty_lines: content.lines().count(),
        branch_count: 0,
        imports: Vec::new(),
        content: Some(content.to_string()),
        has_inline_tests,
        in_executable_package: false,
        deferred_imports: Vec::new(),
    }
}
