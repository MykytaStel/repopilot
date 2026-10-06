use super::support::*;
use serde_json::Value;

#[test]
fn dirty_since_current_named_and_default_facts_keep_exact_cache_parity() {
    for (export, committed_caller, dirty_caller, expected) in [
        (
            "export default function load() {}\n",
            "import load from './api.ts';\n",
            "import { save } from './api.ts';\n",
            0,
        ),
        (
            "export default function load() {}\n",
            "import load from './api.ts';\n",
            "// dirty caller\nimport renamed from './api.ts';\nif (renamed) renamed();\n",
            1,
        ),
        (
            "export function load() {}\n",
            "import { load } from './api.ts';\n",
            "import { save } from './api.ts';\n",
            0,
        ),
        (
            "export function load() {}\n",
            "import { load } from './api.ts';\n",
            "// dirty caller\nimport { load as renamed } from './api.ts';\nif (renamed) renamed();\n",
            1,
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_repo(root);
        write(root, "src/api.ts", export);
        write(root, "src/caller.ts", committed_caller);
        commit_all(root, "before");
        let base = git_stdout(root, &["rev-parse", "HEAD"]);
        write(root, "src/api.ts", "export function save() {}\n");
        commit_all(root, "after");
        write(root, "src/caller.ts", dirty_caller);
        let retained = scan_json(root, &["--since", &base]);
        let expected_findings = contract_findings(&retained);
        assert_eq!(expected_findings.len(), expected);
        let warm = scan_json(root, &["--since", &base]);
        assert_eq!(expected_findings, contract_findings(&warm));
        let retained_entry = caller_cache_entry(root);
        remove_caller_symbol_facts(root);
        let rebuilt = scan_json(root, &["--since", &base]);
        assert_eq!(
            expected_findings,
            contract_findings(&rebuilt),
            "{dirty_caller}"
        );
        assert_eq!(rebuilt["context_graph_cache"]["status"], "hit");
        assert_eq!(retained_entry, caller_cache_entry(root));
        let rebuilt_warm = scan_json(root, &["--since", &base]);
        assert_eq!(expected_findings, contract_findings(&rebuilt_warm));
    }
}

fn contract_findings(report: &Value) -> Vec<Value> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule_id"] == RULE_ID)
        .cloned()
        .collect()
}

fn caller_cache_entry(root: &std::path::Path) -> Value {
    let cache: Value = serde_json::from_slice(
        &std::fs::read(root.join(".repopilot/cache/parsed_facts_v2.json")).unwrap(),
    )
    .unwrap();
    cache["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["imports"]
                .as_array()
                .unwrap()
                .iter()
                .any(|module| module == "./api.ts")
        })
        .unwrap()
        .clone()
}
