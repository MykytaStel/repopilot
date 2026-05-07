use repopilot::graph::extract_imports;

// ── Rust ─────────────────────────────────────────────────────────────────────

#[test]
fn rust_use_crate_path() {
    let imports = extract_imports("use crate::foo::bar;\n", Some("Rust"));
    assert!(
        imports.contains(&"crate::foo::bar".to_string()),
        "expected crate::foo::bar in {imports:?}"
    );
}

#[test]
fn rust_mod_child() {
    let imports = extract_imports("mod child;\n", Some("Rust"));
    assert!(
        imports.contains(&"mod::child".to_string()),
        "expected mod::child in {imports:?}"
    );
}

#[test]
fn rust_pub_mod_child() {
    let imports = extract_imports("pub mod child;\n", Some("Rust"));
    assert!(
        imports.contains(&"mod::child".to_string()),
        "expected mod::child from pub mod in {imports:?}"
    );
}

#[test]
fn rust_block_comment_not_extracted() {
    let code = "/* use crate::foo; */\n";
    let imports = extract_imports(code, Some("Rust"));
    assert!(
        imports.is_empty(),
        "block comment must not produce imports: {imports:?}"
    );
}

#[test]
fn rust_line_comment_not_extracted() {
    let code = "// use crate::bar;\n";
    let imports = extract_imports(code, Some("Rust"));
    assert!(
        imports.is_empty(),
        "line comment must not produce imports: {imports:?}"
    );
}

#[test]
fn rust_inline_group_import() {
    let code = "use crate::{foo, bar};\n";
    let imports = extract_imports(code, Some("Rust"));
    assert!(imports.contains(&"crate::foo".to_string()), "{imports:?}");
    assert!(imports.contains(&"crate::bar".to_string()), "{imports:?}");
}

#[test]
fn rust_multiline_group_import() {
    let code = "use crate::{\n    foo,\n    bar,\n};\n";
    let imports = extract_imports(code, Some("Rust"));
    assert!(imports.contains(&"crate::foo".to_string()), "{imports:?}");
    assert!(imports.contains(&"crate::bar".to_string()), "{imports:?}");
}

#[test]
fn rust_imports_are_deduplicated() {
    let code = "use crate::foo;\nuse crate::foo;\n";
    let imports = extract_imports(code, Some("Rust"));
    assert_eq!(imports.iter().filter(|i| *i == "crate::foo").count(), 1);
}

// ── TypeScript / JavaScript ───────────────────────────────────────────────────

#[test]
fn ts_import_from_relative() {
    let code = r#"import { foo } from "./utils";"#;
    let imports = extract_imports(code, Some("TypeScript"));
    assert!(imports.contains(&"./utils".to_string()), "{imports:?}");
}

#[test]
fn ts_export_from_relative() {
    let code = r#"export { bar } from "../lib";"#;
    let imports = extract_imports(code, Some("TypeScript"));
    assert!(imports.contains(&"../lib".to_string()), "{imports:?}");
}

#[test]
fn ts_require_relative() {
    let code = r#"const x = require("../lib");"#;
    let imports = extract_imports(code, Some("TypeScript"));
    assert!(imports.contains(&"../lib".to_string()), "{imports:?}");
}

#[test]
fn ts_non_relative_import_skipped() {
    // Bare specifiers and @-scoped aliases must not be emitted
    let code = r#"import React from "react";"#;
    let imports = extract_imports(code, Some("TypeScript React"));
    assert!(
        imports.is_empty(),
        "bare specifier must be skipped: {imports:?}"
    );
}

#[test]
fn js_import_from_relative() {
    let code = r#"import { fn } from "./helpers";"#;
    let imports = extract_imports(code, Some("JavaScript"));
    assert!(imports.contains(&"./helpers".to_string()), "{imports:?}");
}

// ── Python ────────────────────────────────────────────────────────────────────

#[test]
fn python_relative_single_dot() {
    let code = "from .models import User\n";
    let imports = extract_imports(code, Some("Python"));
    assert!(imports.contains(&".models".to_string()), "{imports:?}");
}

#[test]
fn python_relative_double_dot() {
    let code = "from ..base import Thing\n";
    let imports = extract_imports(code, Some("Python"));
    assert!(imports.contains(&"..base".to_string()), "{imports:?}");
}

#[test]
fn python_comment_not_extracted() {
    let code = "# from .models import User\n";
    let imports = extract_imports(code, Some("Python"));
    assert!(imports.is_empty(), "{imports:?}");
}

// ── Go ────────────────────────────────────────────────────────────────────────

#[test]
fn go_single_import() {
    let code = "import \"fmt\"\n";
    let imports = extract_imports(code, Some("Go"));
    assert!(imports.contains(&"fmt".to_string()), "{imports:?}");
}

#[test]
fn go_import_block() {
    let code = "import (\n\t\"fmt\"\n\t\"os\"\n)\n";
    let imports = extract_imports(code, Some("Go"));
    assert!(imports.contains(&"fmt".to_string()), "{imports:?}");
    assert!(imports.contains(&"os".to_string()), "{imports:?}");
}

#[test]
fn go_import_block_with_alias() {
    let code = "import (\n\tfmt2 \"fmt\"\n)\n";
    let imports = extract_imports(code, Some("Go"));
    assert!(imports.contains(&"fmt".to_string()), "{imports:?}");
}

// ── Unknown language ──────────────────────────────────────────────────────────

#[test]
fn unknown_language_returns_empty() {
    let imports = extract_imports("use crate::foo;\n", Some("TOML"));
    assert!(imports.is_empty());
}

#[test]
fn no_language_returns_empty() {
    let imports = extract_imports("use crate::foo;\n", None);
    assert!(imports.is_empty());
}
