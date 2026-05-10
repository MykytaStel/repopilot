use crate::frameworks::react_native::profile::ReactNativeProjectKind;
use std::path::Path;

pub(super) fn project_kind(
    has_expo_dependency: bool,
    has_ios: bool,
    has_android: bool,
) -> ReactNativeProjectKind {
    match (has_expo_dependency, has_ios || has_android) {
        (true, true) => ReactNativeProjectKind::ExpoPrebuild,
        (true, false) => ReactNativeProjectKind::ExpoManaged,
        (false, true) => ReactNativeProjectKind::Bare,
        (false, false) => ReactNativeProjectKind::Unknown,
    }
}

pub(super) fn detect_package_manager(root: &Path) -> Option<String> {
    [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("npm-shrinkwrap.json", "npm"),
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
    ]
    .iter()
    .find_map(|(file, manager)| root.join(file).is_file().then(|| (*manager).to_string()))
}
