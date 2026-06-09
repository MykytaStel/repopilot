use crate::graph::v2::{
    GraphEdge, GraphEdgeConfidence, GraphEdgeKind, GraphEdgeProvenance, GraphNodeId,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Adds `TestOf` edges from co-located test files to the file they exercise,
/// using conservative naming conventions. An edge is only emitted when the
/// inferred subject is itself a scanned file, so unknown subjects are skipped.
pub(super) fn test_of_edges(known_files: &BTreeMap<PathBuf, GraphNodeId>) -> Vec<GraphEdge> {
    known_files
        .iter()
        .filter_map(|(path, test_id)| {
            let subject_path = test_subject_path(path)?;
            let subject_id = known_files.get(&subject_path)?;
            Some(GraphEdge {
                from: test_id.clone(),
                to: subject_id.clone(),
                kind: GraphEdgeKind::TestOf,
                provenance: GraphEdgeProvenance::TestHeuristic,
                confidence: GraphEdgeConfidence::High,
            })
        })
        .collect()
}

/// The sibling file a test file most likely exercises, or `None` when the name
/// does not match a known test convention.
fn test_subject_path(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    let subject = test_subject_file_name(file_name)?;
    Some(path.with_file_name(subject))
}

fn test_subject_file_name(file_name: &str) -> Option<String> {
    // `foo.test.ts` / `foo.spec.tsx` -> `foo.ts` / `foo.tsx` (JS/TS family).
    for marker in [".test.", ".spec."] {
        if let Some(index) = file_name.find(marker) {
            let stem = &file_name[..index];
            let extension = &file_name[index + marker.len()..];
            if !stem.is_empty()
                && matches!(
                    extension,
                    "ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs"
                )
            {
                return Some(format!("{stem}.{extension}"));
            }
        }
    }

    // `name_test.go` -> `name.go`, `name_test.py` -> `name.py`.
    for suffix in ["_test.go", "_test.py"] {
        if let Some(stem) = file_name.strip_suffix(suffix)
            && !stem.is_empty()
        {
            let extension = &suffix["_test.".len()..];
            return Some(format!("{stem}.{extension}"));
        }
    }

    // `test_name.py` -> `name.py` (pytest convention).
    if let Some(rest) = file_name.strip_prefix("test_")
        && rest.ends_with(".py")
        && rest.len() > ".py".len()
    {
        return Some(rest.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_common_test_naming_conventions() {
        assert_eq!(
            test_subject_file_name("math.test.ts").as_deref(),
            Some("math.ts")
        );
        assert_eq!(
            test_subject_file_name("api.spec.tsx").as_deref(),
            Some("api.tsx")
        );
        assert_eq!(
            test_subject_file_name("server_test.go").as_deref(),
            Some("server.go")
        );
        assert_eq!(
            test_subject_file_name("client_test.py").as_deref(),
            Some("client.py")
        );
        assert_eq!(
            test_subject_file_name("test_client.py").as_deref(),
            Some("client.py")
        );
    }

    #[test]
    fn ignores_non_test_and_malformed_names() {
        assert_eq!(test_subject_file_name("math.ts"), None);
        assert_eq!(test_subject_file_name(".test.ts"), None);
        assert_eq!(test_subject_file_name("_test.go"), None);
        assert_eq!(test_subject_file_name("test_.py"), None);
        assert_eq!(test_subject_file_name("math.test.rs"), None);
    }

    #[test]
    fn emits_edge_only_when_subject_is_a_known_file() {
        let mut known = BTreeMap::new();
        known.insert(
            PathBuf::from("/repo/src/math.ts"),
            GraphNodeId::new("file:src/math.ts"),
        );
        known.insert(
            PathBuf::from("/repo/src/math.test.ts"),
            GraphNodeId::new("file:src/math.test.ts"),
        );
        // A test file whose subject was not scanned produces no edge.
        known.insert(
            PathBuf::from("/repo/src/orphan.test.ts"),
            GraphNodeId::new("file:src/orphan.test.ts"),
        );

        let edges = test_of_edges(&known);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from.as_str(), "file:src/math.test.ts");
        assert_eq!(edges[0].to.as_str(), "file:src/math.ts");
        assert_eq!(edges[0].kind, GraphEdgeKind::TestOf);
    }
}
