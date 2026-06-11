use super::slash_path;
use crate::graph::resolver::normalize_path;
use crate::graph::v2::{GraphNode, GraphNodeId, GraphNodeKind};
use crate::scan::workspace::detect_workspace_packages;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `Package` graph nodes for a workspace plus the file → package membership map.
#[derive(Default)]
pub(super) struct PackageGraph {
    pub nodes: Vec<(GraphNodeId, GraphNode)>,
    pub membership: BTreeMap<GraphNodeId, GraphNodeId>,
}

/// A detected workspace package, normalized for prefix matching.
struct PackageNode {
    id: GraphNodeId,
    node: GraphNode,
    root: PathBuf,
}

/// Build `Package` nodes for the workspace rooted at `repo_root` (already
/// normalized) and map each known file node to the most specific package whose
/// root is a path prefix of the file. Returns empty for a non-workspace root.
pub(super) fn package_graph(
    repo_root: &Path,
    known_files: &BTreeMap<PathBuf, GraphNodeId>,
) -> PackageGraph {
    // An empty root would make manifest reads resolve against the process CWD.
    if repo_root.as_os_str().is_empty() {
        return PackageGraph::default();
    }

    let packages = detect_package_nodes(repo_root);
    let membership = membership_by_longest_prefix(&packages, known_files);

    PackageGraph {
        nodes: packages
            .into_iter()
            .map(|package| (package.id, package.node))
            .collect(),
        membership,
    }
}

fn detect_package_nodes(repo_root: &Path) -> Vec<PackageNode> {
    let mut packages: Vec<PackageNode> = detect_workspace_packages(repo_root)
        .into_iter()
        .filter_map(|package| {
            let root = normalize_path(&package.root);
            // The repo root itself is not a sub-package; sub-package roots have a
            // non-empty relative path.
            let relative = root.strip_prefix(repo_root).ok()?;
            let label = slash_path(relative);
            if label.is_empty() {
                return None;
            }
            let id = GraphNodeId::new(format!("package:{label}"));
            Some(PackageNode {
                node: GraphNode {
                    id: id.clone(),
                    kind: GraphNodeKind::Package,
                    label: package.name,
                    path: Some(root.clone()),
                },
                id,
                root,
            })
        })
        .collect();

    // Deterministic, duplicate-free node order.
    packages.sort_by(|left, right| left.id.cmp(&right.id));
    packages.dedup_by(|left, right| left.id == right.id);
    packages
}

fn membership_by_longest_prefix(
    packages: &[PackageNode],
    known_files: &BTreeMap<PathBuf, GraphNodeId>,
) -> BTreeMap<GraphNodeId, GraphNodeId> {
    let mut membership = BTreeMap::new();
    if packages.is_empty() {
        return membership;
    }

    for (file_path, file_id) in known_files {
        let owner = packages
            .iter()
            .filter(|package| file_path.starts_with(&package.root))
            .max_by_key(|package| package.root.components().count());
        if let Some(package) = owner {
            membership.insert(file_id.clone(), package.id.clone());
        }
    }
    membership
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(id: &str, root: &str) -> PackageNode {
        let id = GraphNodeId::new(id);
        PackageNode {
            node: GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::Package,
                label: String::new(),
                path: Some(PathBuf::from(root)),
            },
            id,
            root: PathBuf::from(root),
        }
    }

    fn files(entries: &[(&str, &str)]) -> BTreeMap<PathBuf, GraphNodeId> {
        entries
            .iter()
            .map(|(path, id)| (PathBuf::from(path), GraphNodeId::new(*id)))
            .collect()
    }

    #[test]
    fn membership_picks_the_most_specific_package() {
        let packages = vec![
            pkg("package:packages/app", "/repo/packages/app"),
            pkg("package:packages/app/plugin", "/repo/packages/app/plugin"),
        ];
        let known = files(&[
            ("/repo/packages/app/src/main.ts", "file:a"),
            ("/repo/packages/app/plugin/src/p.ts", "file:b"),
        ]);

        let membership = membership_by_longest_prefix(&packages, &known);

        assert_eq!(
            membership
                .get(&GraphNodeId::new("file:a"))
                .unwrap()
                .as_str(),
            "package:packages/app"
        );
        // The nested package wins over its ancestor by longest prefix.
        assert_eq!(
            membership
                .get(&GraphNodeId::new("file:b"))
                .unwrap()
                .as_str(),
            "package:packages/app/plugin"
        );
    }

    #[test]
    fn files_outside_every_package_have_no_membership() {
        let packages = vec![pkg("package:packages/app", "/repo/packages/app")];
        let known = files(&[("/repo/tools/script.ts", "file:x")]);

        assert!(membership_by_longest_prefix(&packages, &known).is_empty());
    }

    #[test]
    fn no_packages_means_no_membership() {
        let known = files(&[("/repo/src/main.ts", "file:x")]);
        assert!(membership_by_longest_prefix(&[], &known).is_empty());
    }
}
