use super::{JavaScriptContractFactProvider, detect_removed_export_imports};
use crate::analysis::symbols::{
    ExportedSymbolFact, ImportedSymbolFact, JavaScriptSymbolFacts, SymbolKind,
};
use crate::scan::types::CouplingGraph;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Facts {
    before: HashMap<PathBuf, JavaScriptSymbolFacts>,
    current: HashMap<PathBuf, JavaScriptSymbolFacts>,
}

impl JavaScriptContractFactProvider for Facts {
    fn pre_change_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        self.before.get(path).cloned()
    }

    fn current_facts(&mut self, path: &Path) -> Option<JavaScriptSymbolFacts> {
        self.current.get(path).cloned()
    }
}

#[test]
fn removed_value_export_with_surviving_resolved_import_is_reported() {
    let exporter = PathBuf::from("src/api.ts");
    let importer = PathBuf::from("src/caller.ts");
    let mut provider = Facts::default();
    provider.before.insert(
        exporter.clone(),
        JavaScriptSymbolFacts {
            exports: vec![exported("loadUser")],
            ..Default::default()
        },
    );
    provider.current.insert(
        exporter.clone(),
        JavaScriptSymbolFacts {
            exports: vec![exported("saveUser")],
            ..Default::default()
        },
    );
    provider.current.insert(
        importer.clone(),
        JavaScriptSymbolFacts {
            imports: vec![ImportedSymbolFact {
                imported_name: "loadUser".to_string(),
                local_name: "load".to_string(),
                kind: SymbolKind::Value,
                module_specifier: "./api.ts".to_string(),
                line_start: 2,
                line_end: 2,
                byte_start: 10,
                byte_end: 26,
            }],
            ..Default::default()
        },
    );
    let graph = CouplingGraph {
        edges: BTreeMap::from([(importer.clone(), BTreeSet::from([exporter.clone()]))]),
        nodes: BTreeSet::from([exporter.clone(), importer.clone()]),
        ..Default::default()
    };
    let root = Path::new("/repo");
    let current_files = HashSet::from([root.join(&exporter), root.join(&importer)]);

    let occurrences = detect_removed_export_imports(
        root,
        std::slice::from_ref(&exporter),
        &graph,
        &current_files,
        &mut provider,
    );

    assert_eq!(occurrences.len(), 1);
    let occurrence = &occurrences[0];
    assert_eq!(occurrence.exporter_path, exporter);
    assert_eq!(occurrence.importer_path, importer);
    assert_eq!(occurrence.exported_name, "loadUser");
    assert_eq!(occurrence.local_name, "load");
    assert_eq!((occurrence.line_start, occurrence.byte_start), (2, 10));
}

fn exported(name: &str) -> ExportedSymbolFact {
    ExportedSymbolFact {
        name: name.to_string(),
        kind: SymbolKind::Value,
        line_start: 1,
        line_end: 1,
    }
}
