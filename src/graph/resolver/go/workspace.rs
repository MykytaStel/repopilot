//! Local Go module mapping from `go.mod`, `go.work`, and in-repository replaces.

use super::super::normalize_path;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LocalModule {
    pub(super) import_path: String,
    pub(super) directory: PathBuf,
}

pub(crate) struct GoIndex {
    root: PathBuf,
    source_modules: Vec<(PathBuf, Vec<LocalModule>)>,
}

impl GoIndex {
    pub(crate) fn new(root: &Path) -> Self {
        let root = normalize_path(root);
        let mut source_roots = vec![root.clone()];
        source_roots.extend(
            workspace_use_directories(&root)
                .into_iter()
                .filter(|directory| {
                    directory.starts_with(&root)
                        && read_module_file(&directory.join("go.mod")).is_some()
                }),
        );
        source_roots.sort();
        source_roots.dedup();
        let source_modules = source_roots
            .into_iter()
            .map(|source_root| {
                let source = source_root.join("__repopilot_index__.go");
                let modules = load_local_modules(&source, &root);
                (source_root, modules)
            })
            .collect();
        Self {
            root,
            source_modules,
        }
    }

    pub(super) fn modules_for_source(&self, source: &Path) -> &[LocalModule] {
        let source = if source.is_absolute() {
            normalize_path(source)
        } else {
            normalize_path(&self.root.join(source))
        };
        self.source_modules
            .iter()
            .filter(|(module_root, _)| source.starts_with(module_root))
            .max_by_key(|(module_root, _)| module_root.components().count())
            .or_else(|| self.source_modules.first())
            .map(|(_, modules)| modules.as_slice())
            .unwrap_or_default()
    }
}

pub(super) fn local_modules(source: &Path, root: &Path) -> Vec<LocalModule> {
    load_local_modules(source, root)
}

fn load_local_modules(source: &Path, root: &Path) -> Vec<LocalModule> {
    let root = normalize_path(root);
    let source_module = source_module_directory(source, &root);
    let replacements = module_replacements(&root, &source_module);
    let replaced_paths = replacement_paths(&replacements);
    let mut modules = root_module(&root, &replaced_paths);
    modules.extend(workspace_modules(&root, &replaced_paths));
    modules.extend(replacement_modules(&root, replacements));
    modules.sort_by(|left, right| {
        left.import_path
            .cmp(&right.import_path)
            .then_with(|| left.directory.cmp(&right.directory))
    });
    modules.dedup();
    modules
}

fn replacement_paths(replacements: &[(String, Option<PathBuf>)]) -> HashSet<&str> {
    replacements.iter().map(|(path, _)| path.as_str()).collect()
}

fn root_module(root: &Path, replaced_paths: &HashSet<&str>) -> Vec<LocalModule> {
    read_module_file(&root.join("go.mod"))
        .filter(|module| !replaced_paths.contains(module.as_str()))
        .map(|import_path| {
            vec![LocalModule {
                import_path,
                directory: root.to_path_buf(),
            }]
        })
        .unwrap_or_default()
}

fn workspace_modules(root: &Path, replaced_paths: &HashSet<&str>) -> Vec<LocalModule> {
    workspace_use_directories(root)
        .into_iter()
        .filter(|directory| directory.starts_with(root))
        .filter_map(|directory| {
            let import_path = read_module_file(&directory.join("go.mod"))?;
            (!replaced_paths.contains(import_path.as_str())).then_some(LocalModule {
                import_path,
                directory,
            })
        })
        .collect()
}

fn replacement_modules(
    root: &Path,
    replacements: Vec<(String, Option<PathBuf>)>,
) -> Vec<LocalModule> {
    replacements
        .into_iter()
        .filter_map(|(import_path, directory)| {
            let directory = directory?;
            (directory.starts_with(root) && read_module_file(&directory.join("go.mod")).is_some())
                .then_some(LocalModule {
                    import_path,
                    directory,
                })
        })
        .collect()
}

fn source_module_directory(source: &Path, root: &Path) -> PathBuf {
    let source = if source.is_absolute() {
        normalize_path(source)
    } else {
        normalize_path(&root.join(source))
    };
    let mut directory = source.parent();
    while let Some(candidate) = directory {
        if !candidate.starts_with(root) {
            break;
        }
        if candidate.join("go.mod").is_file() {
            return candidate.to_path_buf();
        }
        if candidate == root {
            break;
        }
        directory = candidate.parent();
    }
    root.to_path_buf()
}

fn workspace_use_directories(root: &Path) -> Vec<PathBuf> {
    let Ok(content) = std::fs::read_to_string(root.join("go.work")) else {
        return Vec::new();
    };
    let mut in_use_block = false;
    let mut directories = Vec::new();
    for line in content.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line == "use (" {
            in_use_block = true;
            continue;
        }
        if in_use_block && line == ")" {
            in_use_block = false;
            continue;
        }
        let path = if in_use_block {
            Some(line)
        } else {
            line.strip_prefix("use ")
                .map(|value| value.trim().trim_matches('"'))
        };
        let Some(path) = path else {
            continue;
        };
        directories.push(normalize_path(&root.join(path.trim_matches('"'))));
    }
    directories.sort();
    directories.dedup();
    directories
}

fn module_replacements(root: &Path, source_module: &Path) -> Vec<(String, Option<PathBuf>)> {
    let mut replacements = Vec::new();
    for (file, base) in [
        (root.join("go.work"), root),
        (source_module.join("go.mod"), source_module),
    ] {
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        let mut in_replace_block = false;
        for line in content.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if line == "replace (" {
                in_replace_block = true;
                continue;
            }
            if in_replace_block && line == ")" {
                in_replace_block = false;
                continue;
            }
            let directive = if in_replace_block {
                Some(line)
            } else {
                line.strip_prefix("replace")
                    .map(str::trim_start)
                    .filter(|value| !value.is_empty())
            };
            let Some((old, new)) = directive.and_then(|value| value.split_once("=>")) else {
                continue;
            };
            let import_path = old.split_whitespace().next().unwrap_or("");
            let local_path = new
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"');
            if import_path.is_empty() {
                continue;
            }
            let directory = (local_path.starts_with("./") || local_path.starts_with("../"))
                .then(|| normalize_path(&base.join(local_path)));
            replacements.push((import_path.to_string(), directory));
        }
    }
    replacements.sort();
    replacements.dedup();
    replacements
}

fn read_module_file(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().and_then(|content| {
        content.lines().find_map(|line| {
            let line = line.split("//").next().unwrap_or("").trim();
            line.strip_prefix("module ")
                .map(|module| module.trim().trim_matches('"').to_string())
        })
    })
}
