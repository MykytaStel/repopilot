use repopilot::graph::{build_coupling_graph, compute_metrics, detect_cycles};
use repopilot::scan::scanner::collect_scan_facts;
use std::fs;
use tempfile::tempdir;

// ── Rust graph ────────────────────────────────────────────────────────────────

#[test]
fn rust_graph_edge_from_main_to_foo() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "use crate::foo;\nfn main() {}\n").unwrap();
    fs::write(root.join("src/foo.rs"), "pub fn bar() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let main_path = root.join("src/main.rs");
    let foo_path = root.join("src/foo.rs");

    assert!(graph.nodes.contains(&main_path), "main.rs must be a node");
    assert!(graph.nodes.contains(&foo_path), "foo.rs must be a node");

    let main_edges = graph
        .edges
        .get(&main_path)
        .expect("main.rs must have an edge set");
    assert!(
        main_edges.contains(&foo_path),
        "main.rs must have an edge to foo.rs"
    );
}

#[test]
fn rust_graph_no_spurious_edges() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/foo.rs"), "pub fn bar() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let foo_edges = graph
        .edges
        .get(&root.join("src/foo.rs"))
        .map(|edges| edges.len())
        .unwrap_or(0);
    assert_eq!(foo_edges, 0, "foo.rs must have no outgoing edges");
}

#[test]
fn rust_graph_edge_from_attributed_mod_declaration() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "#[cfg(feature = \"child\")]\npub mod child;\n",
    )
    .unwrap();
    fs::write(root.join("src/child.rs"), "pub fn child() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let lib_path = root.join("src/lib.rs");
    let child_path = root.join("src/child.rs");
    let lib_edges = graph
        .edges
        .get(&lib_path)
        .expect("lib.rs must have an edge set");

    assert!(
        lib_edges.contains(&child_path),
        "attributed mod declarations should resolve to module file edges"
    );
}

#[test]
fn rust_graph_edge_from_crate_symbol_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/main.rs"),
        "use crate::config::Settings;\nfn main() {}\n",
    )
    .unwrap();
    fs::write(root.join("src/config.rs"), "pub struct Settings;\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let main_path = root.join("src/main.rs");
    let config_path = root.join("src/config.rs");
    let main_edges = graph
        .edges
        .get(&main_path)
        .expect("main.rs must have an edge set");

    assert!(
        main_edges.contains(&config_path),
        "crate::module::Symbol should resolve to module file"
    );
}

#[test]
fn rust_graph_edge_from_self_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src/feature")).unwrap();
    fs::write(
        root.join("src/feature/mod.rs"),
        "use self::model::Thing;\npub fn feature() {}\n",
    )
    .unwrap();
    fs::write(root.join("src/feature/model.rs"), "pub struct Thing;\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let feature_path = root.join("src/feature/mod.rs");
    let model_path = root.join("src/feature/model.rs");
    let feature_edges = graph
        .edges
        .get(&feature_path)
        .expect("feature/mod.rs must have an edge set");

    assert!(
        feature_edges.contains(&model_path),
        "self::module should resolve within the current module directory"
    );
}

#[test]
fn rust_graph_edge_from_super_path() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("src/feature")).unwrap();
    fs::write(
        root.join("src/feature/controller.rs"),
        "use super::model::Thing;\npub fn controller() {}\n",
    )
    .unwrap();
    fs::write(root.join("src/feature/model.rs"), "pub struct Thing;\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let controller_path = root.join("src/feature/controller.rs");
    let model_path = root.join("src/feature/model.rs");
    let controller_edges = graph
        .edges
        .get(&controller_path)
        .expect("controller.rs must have an edge set");

    assert!(
        controller_edges.contains(&model_path),
        "super::module should resolve relative to the parent module"
    );
}

// ── TypeScript graph ──────────────────────────────────────────────────────────

#[test]
fn ts_graph_edge_from_app_to_format() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    let utils = src.join("utils");
    fs::create_dir_all(&utils).unwrap();

    fs::write(
        src.join("App.tsx"),
        "import { format } from \"./utils/format\";\n",
    )
    .unwrap();
    fs::write(
        utils.join("format.ts"),
        "export function format(s: string) { return s; }\n",
    )
    .unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let app_path = src.join("App.tsx");
    let format_path = utils.join("format.ts");

    let app_edges = graph
        .edges
        .get(&app_path)
        .expect("App.tsx must have an edge set");
    assert!(
        app_edges.contains(&format_path),
        "App.tsx must have an edge to utils/format.ts"
    );
}

#[test]
fn ts_graph_edge_from_dynamic_import() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join("App.ts"), "const route = import(\"./route\");\n").unwrap();
    fs::write(src.join("route.ts"), "export const route = true;\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let app_path = src.join("App.ts");
    let route_path = src.join("route.ts");
    let app_edges = graph
        .edges
        .get(&app_path)
        .expect("App.ts must have an edge set");

    assert!(
        app_edges.contains(&route_path),
        "dynamic import() should resolve to a local module edge"
    );
}

#[test]
fn ts_graph_edges_from_tsconfig_aliases_and_root_absolute_imports() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(src.join("app")).unwrap();
    fs::create_dir_all(root.join("internal")).unwrap();

    fs::write(
        root.join("tsconfig.json"),
        r##"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "@app/*": ["src/app/*"],
                    "@/*": ["src/*"],
                    "~/*": ["src/*"],
                    "#internal/*": ["internal/*"]
                }
            }
        }"##,
    )
    .unwrap();
    fs::write(
        src.join("App.ts"),
        r##"
            import { app } from "@app/util";
            import { shared } from "@/shared";
            import theme from "~/theme";
            import { client } from "#internal/client";
            import { rooted } from "/src/rooted";
            export const value = [app, shared, theme, client, rooted];
        "##,
    )
    .unwrap();
    fs::write(src.join("app/util.ts"), "export const app = 1;\n").unwrap();
    fs::write(src.join("shared.ts"), "export const shared = 1;\n").unwrap();
    fs::write(src.join("theme.ts"), "export default 1;\n").unwrap();
    fs::write(
        root.join("internal/client.ts"),
        "export const client = 1;\n",
    )
    .unwrap();
    fs::write(src.join("rooted.ts"), "export const rooted = 1;\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let app_path = src.join("App.ts");
    let app_edges = graph
        .edges
        .get(&app_path)
        .expect("App.ts must have an edge set");

    for expected in [
        src.join("app/util.ts"),
        src.join("shared.ts"),
        src.join("theme.ts"),
        root.join("internal/client.ts"),
        src.join("rooted.ts"),
    ] {
        assert!(
            app_edges.contains(&expected),
            "expected alias/root import edge to {}; got {app_edges:?}",
            expected.display()
        );
    }
}

// ── Python graph ──────────────────────────────────────────────────────────────

#[test]
fn python_graph_edges_from_absolute_repo_imports() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("app")).unwrap();
    fs::write(
        root.join("app/main.py"),
        "import app.models\nfrom app import services\n",
    )
    .unwrap();
    fs::write(root.join("app/models.py"), "class User: pass\n").unwrap();
    fs::write(root.join("app/services.py"), "class Service: pass\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let main_path = root.join("app/main.py");
    let main_edges = graph
        .edges
        .get(&main_path)
        .expect("main.py must have an edge set");

    assert!(main_edges.contains(&root.join("app/models.py")));
    assert!(main_edges.contains(&root.join("app/services.py")));
}

// ── Go graph ──────────────────────────────────────────────────────────────────

#[test]
fn go_graph_edge_from_module_import() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("internal/config")).unwrap();
    fs::write(root.join("go.mod"), "module example.com/repopilot-test\n").unwrap();
    fs::write(
        root.join("main.go"),
        "package main\n\nimport \"example.com/repopilot-test/internal/config\"\n",
    )
    .unwrap();
    fs::write(root.join("internal/config/config.go"), "package config\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    let main_path = root.join("main.go");
    let config_path = root.join("internal/config/config.go");
    let main_edges = graph
        .edges
        .get(&main_path)
        .expect("main.go must have an edge set");

    assert!(
        main_edges.contains(&config_path),
        "go module imports should resolve to package files"
    );
}

// ── Metrics ───────────────────────────────────────────────────────────────────

#[test]
fn metrics_fan_in_and_fan_out() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join("shared.rs"), "pub fn shared() {}\n").unwrap();
    fs::write(src.join("a.rs"), "use crate::shared;\npub fn a() {}\n").unwrap();
    fs::write(
        src.join("b.rs"),
        "use crate::shared;\nuse crate::a;\npub fn b() {}\n",
    )
    .unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let metrics = compute_metrics(&graph);

    let shared_path = src.join("shared.rs");
    let a_path = src.join("a.rs");
    let b_path = src.join("b.rs");

    let shared_m = metrics.iter().find(|m| m.path == shared_path).unwrap();
    assert_eq!(shared_m.fan_in, 2, "shared.rs imported by a and b");
    assert_eq!(shared_m.fan_out, 0);

    let a_m = metrics.iter().find(|m| m.path == a_path).unwrap();
    assert_eq!(a_m.fan_in, 1, "a.rs imported by b");
    assert_eq!(a_m.fan_out, 1, "a.rs imports shared");

    let b_m = metrics.iter().find(|m| m.path == b_path).unwrap();
    assert_eq!(b_m.fan_in, 0);
    assert_eq!(b_m.fan_out, 2, "b.rs imports shared and a");
}

#[test]
fn instability_formula_is_correct() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join("stable.rs"), "pub fn s() {}\n").unwrap();
    fs::write(src.join("mid.rs"), "use crate::stable;\npub fn m() {}\n").unwrap();
    fs::write(src.join("unstable.rs"), "use crate::mid;\npub fn u() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let metrics = compute_metrics(&graph);

    let stable_m = metrics
        .iter()
        .find(|m| m.path == src.join("stable.rs"))
        .unwrap();
    let mid_m = metrics
        .iter()
        .find(|m| m.path == src.join("mid.rs"))
        .unwrap();
    let unstable_m = metrics
        .iter()
        .find(|m| m.path == src.join("unstable.rs"))
        .unwrap();

    assert!((stable_m.instability - 0.0).abs() < f32::EPSILON);
    assert!((mid_m.instability - 0.5).abs() < f32::EPSILON);
    assert!((unstable_m.instability - 1.0).abs() < f32::EPSILON);
}

// ── Cycle detection ───────────────────────────────────────────────────────────

#[test]
fn two_node_cycle_is_detected() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join("a.rs"), "use crate::b;\n").unwrap();
    fs::write(src.join("b.rs"), "use crate::a;\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let cycles = detect_cycles(&graph);

    assert!(!cycles.is_empty(), "must detect at least one cycle");
    let a_path = src.join("a.rs");
    let b_path = src.join("b.rs");
    assert!(
        cycles
            .iter()
            .any(|cycle| cycle.contains(&a_path) && cycle.contains(&b_path)),
        "cycle must include both a.rs and b.rs; got {cycles:?}"
    );
}

#[test]
fn no_cycle_in_acyclic_graph() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(src.join("a.rs"), "use crate::b;\n").unwrap();
    fs::write(src.join("b.rs"), "pub fn b() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let cycles = detect_cycles(&graph);

    assert!(
        cycles.is_empty(),
        "acyclic graph must have no cycles: {cycles:?}"
    );
}

// ── Isolated file ─────────────────────────────────────────────────────────────

#[test]
fn isolated_file_has_zero_metrics() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("standalone.rs"), "pub fn hello() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let metrics = compute_metrics(&graph);

    let metric = metrics
        .iter()
        .find(|metric| metric.path == src.join("standalone.rs"))
        .unwrap();
    assert_eq!(metric.fan_in, 0);
    assert_eq!(metric.fan_out, 0);
    assert!((metric.instability - 0.0).abs() < f32::EPSILON);
}

#[test]
fn isolated_file_is_a_graph_node() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("standalone.rs"), "pub fn hello() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);

    assert!(graph.nodes.contains(&src.join("standalone.rs")));
}

// ── Metrics output is stable ──────────────────────────────────────────────────

#[test]
fn metrics_are_returned_in_path_order() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.rs"), "pub fn a() {}\n").unwrap();
    fs::write(src.join("b.rs"), "pub fn b() {}\n").unwrap();
    fs::write(src.join("c.rs"), "pub fn c() {}\n").unwrap();

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let metrics = compute_metrics(&graph);

    let paths: Vec<_> = metrics.iter().map(|metric| &metric.path).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "metrics must be in stable path order");
}

#[test]
fn large_acyclic_import_graph_stays_stable() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    const FILE_COUNT: usize = 180;
    for index in 0..FILE_COUNT {
        let next_import = if index + 1 < FILE_COUNT {
            format!("use crate::module_{};\n", index + 1)
        } else {
            String::new()
        };
        fs::write(
            src.join(format!("module_{index}.rs")),
            format!("{next_import}pub fn module_{index}() {{}}\n"),
        )
        .unwrap();
    }

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let metrics = compute_metrics(&graph);
    let cycles = detect_cycles(&graph);

    assert_eq!(graph.nodes.len(), FILE_COUNT);
    assert_eq!(metrics.len(), FILE_COUNT);
    assert!(cycles.is_empty());
}

#[test]
fn cycle_detection_depth_exceeded_warning() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();

    const FILE_COUNT: usize = 515;
    for index in 0..FILE_COUNT {
        let next_import = if index + 1 < FILE_COUNT {
            format!("use crate::module_{};\n", index + 1)
        } else {
            "use crate::module_0;\n".to_string()
        };
        fs::write(
            src.join(format!("module_{index}.rs")),
            format!("{next_import}pub fn module_{index}() {{}}\n"),
        )
        .unwrap();
    }

    let facts = collect_scan_facts(root).unwrap();
    let graph = build_coupling_graph(&facts, root);
    let _cycles = detect_cycles(&graph);

    assert!(
        repopilot::graph::was_cycle_detection_depth_exceeded(),
        "should have exceeded depth cap"
    );
}
