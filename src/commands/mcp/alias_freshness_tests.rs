use super::workspace_freshness_tests::{git, state, tool_text};
use super::{handle_tools_call, scan};
use serde_json::{Value, json};
use std::fs;

#[test]
fn scan_alias_remap_is_fresh_and_semantically_stable_in_one_mcp_session() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    git(root, &["init", "-q"]);
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/consumer.ts"),
        "import { api } from '@api';\nexport const result = api;\n",
    )
    .unwrap();
    for target in ["one", "two"] {
        fs::write(
            root.join(format!("src/{target}.ts")),
            "export const api = 1;\n",
        )
        .unwrap();
    }
    let params = json!({"name": scan::TOOL_NAME, "arguments": {"path": ".", "profile": "strict"}});
    let mut session = state(root);
    let remap = |target| {
        fs::write(
            root.join("tsconfig.json"),
            format!(r#"{{"compilerOptions":{{"paths":{{"@api":["src/{target}"]}}}}}}"#),
        )
        .unwrap()
    };
    remap("one");
    let before: Value = serde_json::from_str(&tool_text(handle_tools_call(
        json!(1),
        &params,
        &mut session,
    )))
    .unwrap();
    remap("two");
    let after: Value = serde_json::from_str(&tool_text(handle_tools_call(
        json!(2),
        &params,
        &mut session,
    )))
    .unwrap();
    assert_ne!(
        before["coupling_graph"]["edges"],
        after["coupling_graph"]["edges"]
    );
    let repeated: Value = serde_json::from_str(&tool_text(handle_tools_call(
        json!(3),
        &params,
        &mut session,
    )))
    .unwrap();
    let normalize = |mut value: Value| {
        let object = value.as_object_mut().unwrap();
        for key in ["context_graph_cache", "scan_timings", "scan_duration_us"] {
            object.remove(key);
        }
        value
    };
    assert_eq!(normalize(after), normalize(repeated));
}
