use super::*;

#[test]
fn default_export_removal_requires_surviving_direct_import() {
    for (extension, label) in [("ts", "TypeScript"), ("js", "JavaScript")] {
        for (before, after, caller, expected) in [
            (
                "export default function load() {}\n",
                "export function load() {}\n",
                "import load from './api';\n",
                1,
            ),
            (
                "export default 42;\n",
                "export const value = 42;\n",
                "import load from './api';\n",
                1,
            ),
            (
                "const load = 42; export { load as default };\n",
                "const load = 42;\n",
                "import { default as load } from './api';\n",
                1,
            ),
            (
                "export default 42;\n",
                "export default 43;\n",
                "import load from './api';\n",
                0,
            ),
            (
                "export default 42;\n",
                "export const load = 42;\n",
                "import { load } from './api';\n",
                0,
            ),
            (
                "export default 42;\n",
                "export { default } from './other';\n",
                "import load from './api';\n",
                0,
            ),
            (
                "export default 42;\n",
                "export default ;\n",
                "import load from './api';\n",
                0,
            ),
            (
                "const load = 42; export { load as \"default\" };\n",
                "const load = 42;\n",
                "import { \"default\" as load } from './api';\n",
                1,
            ),
            (
                "export default 42;\n",
                "module.exports = 42;\n",
                "import load from './api';\n",
                0,
            ),
            (
                "export default 42;\n",
                "exports.default = 42;\n",
                "import load from './api';\n",
                0,
            ),
            (
                "import load from './other'; export default (load);\n",
                "export const value = 42;\n",
                "import load from './api';\n",
                0,
            ),
            ("export default 42;\n", "export const value = 42;\n", "", 0),
            (
                "import load from './other'; export default load;\n",
                "export const value = 42;\n",
                "import load from './api';\n",
                0,
            ),
            (
                "const load = 42; export default load;\n",
                "export const value = 42;\n",
                "import load from './api';\n",
                1,
            ),
            (
                "export default class Load {}\n",
                "class Load {}\n",
                "import load from './api';\n",
                1,
            ),
        ] {
            let temp = TempDir::new().unwrap();
            let root = temp.path();
            init_repo(root);
            let exporter = format!("src/api.{extension}");
            let importer = format!("src/caller.{extension}");
            write(root, &exporter, after);
            write(root, &importer, caller);
            let api = changed(&exporter, ChangeStatus::Modified);
            let pre = ReviewSource::new(before.to_string(), Some(label.to_string()));
            let post = ReviewSource::new(after.to_string(), Some(label.to_string()));
            let signals = detect_removed_export_imports(
                root,
                DiffTarget::WorkingTree,
                &[ChangedReviewSources {
                    file: &api,
                    pre: Some(&pre),
                    post: Some(&post),
                }],
                Some(&coupling_graph(&[(&importer, &exporter)])),
            );
            assert_eq!(signals.len(), expected, "{extension}: {before} -> {after}");
            if let Some(signal) = signals.first() {
                assert_eq!(signal.exported_name, "default");
                assert_eq!(signal.local_name, "load");
                assert_eq!(signal.exporter_path, PathBuf::from(&exporter));
                assert_eq!(signal.importer_path, PathBuf::from(&importer));
                assert_eq!((signal.line_start, signal.line_end), (1, 1));
            }
        }
    }
}

#[test]
fn default_export_ts_export_assignment_is_uncertain() {
    let facts =
        extract_javascript_symbol_facts(&source("const load = 42; export = load;\n")).unwrap();
    assert_eq!(facts.re_exports, vec!["default"]);
}
