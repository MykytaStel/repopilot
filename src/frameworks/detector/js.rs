use crate::frameworks::detector::extract_version;
use crate::frameworks::types::DetectedFramework;
use std::path::Path;

pub(super) fn detect_js_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let pkg_path = root.join("package.json");
    let content = match std::fs::read_to_string(&pkg_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut deps = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(obj) = value.get(key).and_then(|v| v.as_object()) {
            for (k, v) in obj {
                deps.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }

    let version_of = |pkg: &str| -> Option<String> {
        deps.get(pkg)
            .and_then(|v| v.as_str())
            .and_then(extract_version)
    };

    let mut frameworks = Vec::new();

    if deps.contains_key("react-native") {
        frameworks.push(DetectedFramework::ReactNative {
            version: version_of("react-native"),
        });
    }
    if deps.contains_key("expo") {
        frameworks.push(DetectedFramework::Expo {
            version: version_of("expo"),
        });
    }
    if deps.contains_key("next") {
        frameworks.push(DetectedFramework::NextJs {
            version: version_of("next"),
        });
    }
    if deps.contains_key("react") {
        frameworks.push(DetectedFramework::React {
            version: version_of("react"),
        });
    }
    if deps.contains_key("vue") {
        frameworks.push(DetectedFramework::Vue {
            version: version_of("vue"),
        });
    }
    if deps.contains_key("@angular/core") {
        frameworks.push(DetectedFramework::Angular {
            version: version_of("@angular/core"),
        });
    }
    if deps.contains_key("svelte") {
        frameworks.push(DetectedFramework::Svelte {
            version: version_of("svelte"),
        });
    }
    if deps.contains_key("@nestjs/core") {
        frameworks.push(DetectedFramework::NestJs {
            version: version_of("@nestjs/core"),
        });
    }
    if deps.contains_key("express") {
        frameworks.push(DetectedFramework::Express {
            version: version_of("express"),
        });
    }

    frameworks
}
