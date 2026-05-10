use std::path::Path;

pub(super) fn detect_expo_config(root: &Path) -> (bool, Option<bool>) {
    let app_json = root.join("app.json");
    if let Ok(content) = std::fs::read_to_string(&app_json) {
        let parsed = serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|value| {
                value
                    .get("expo")
                    .and_then(|expo| expo.get("newArchEnabled"))
                    .and_then(|v| v.as_bool())
            });
        return (true, parsed);
    }

    for name in ["app.config.js", "app.config.ts"] {
        if let Ok(content) = std::fs::read_to_string(root.join(name)) {
            return (true, parse_js_bool_property(&content, "newArchEnabled"));
        }
    }

    (false, None)
}

pub(super) fn parse_gradle_properties(content: &str) -> (Option<bool>, Option<bool>) {
    let mut new_arch = None;
    let mut hermes = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value
            .split_once('#')
            .map(|(v, _)| v)
            .unwrap_or(value)
            .trim();
        let parsed = match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        match key {
            "newArchEnabled" => new_arch = parsed,
            "hermesEnabled" | "enableHermes" if hermes.is_none() => {
                hermes = parsed;
            }
            _ => {}
        }
    }

    (new_arch, hermes)
}

pub(super) fn parse_podfile(content: &str) -> (Option<bool>, Option<bool>) {
    let mut new_arch = None;
    let mut hermes = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }

        if new_arch.is_none() && line.contains("RCT_NEW_ARCH_ENABLED") {
            if line.contains("'1'")
                || line.contains("\"1\"")
                || line.contains("= 1")
                || line.ends_with("=1")
            {
                new_arch = Some(true);
            } else if line.contains("'0'")
                || line.contains("\"0\"")
                || line.contains("= 0")
                || line.ends_with("=0")
            {
                new_arch = Some(false);
            }
        }

        if new_arch.is_none() && line.contains("new_arch_enabled") {
            if line.contains("=> true") {
                new_arch = Some(true);
            } else if line.contains("=> false") {
                new_arch = Some(false);
            }
        }

        if hermes.is_none() && line.contains(":hermes_enabled") {
            if line.contains("=> true") {
                hermes = Some(true);
            } else if line.contains("=> false") {
                hermes = Some(false);
            }
        }
    }

    (new_arch, hermes)
}

pub(super) fn parse_podfile_properties_json(content: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|value| match value.get("newArchEnabled") {
            Some(serde_json::Value::Bool(v)) => Some(*v),
            Some(serde_json::Value::String(v)) if v == "true" => Some(true),
            Some(serde_json::Value::String(v)) if v == "false" => Some(false),
            _ => None,
        })
}

pub(super) fn has_bool_mismatch(values: [Option<bool>; 3]) -> bool {
    let mut seen_true = false;
    let mut seen_false = false;
    for value in values.into_iter().flatten() {
        seen_true |= value;
        seen_false |= !value;
    }
    seen_true && seen_false
}

fn parse_js_bool_property(content: &str, property: &str) -> Option<bool> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        if !line.contains(property) {
            continue;
        }
        if line.contains("true") {
            return Some(true);
        }
        if line.contains("false") {
            return Some(false);
        }
    }
    None
}
