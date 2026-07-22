use crate::frameworks::types::DetectedFramework;
use std::path::Path;

pub(crate) fn detect_python_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let content = match std::fs::read_to_string(root.join("requirements.txt")) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut frameworks = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name_end = line
            .find(|c: char| ['=', '>', '<', '!', '~', '[', ' ', '#', '@'].contains(&c))
            .unwrap_or(line.len());
        let pkg = line[..name_end].trim().to_lowercase();
        let version = line
            .find("==")
            .map(|pos| {
                line[pos + 2..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .filter(|v| !v.is_empty());

        match pkg.as_str() {
            "django" => frameworks.push(DetectedFramework::Django { version }),
            "flask" => frameworks.push(DetectedFramework::Flask { version }),
            "fastapi" => frameworks.push(DetectedFramework::FastApi { version }),
            _ => {}
        }
    }

    frameworks
}
