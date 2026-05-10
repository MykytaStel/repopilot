use crate::frameworks::types::DetectedFramework;
use std::path::Path;

pub(super) fn detect_go_frameworks(root: &Path) -> Vec<DetectedFramework> {
    let content = match std::fs::read_to_string(root.join("go.mod")) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut frameworks = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        let entry = trimmed.trim_start_matches("require ").trim();
        let version = entry
            .split_whitespace()
            .nth(1)
            .map(|v| v.trim_start_matches('v').to_string())
            .filter(|v| !v.is_empty());

        if trimmed.contains("gin-gonic/gin") {
            frameworks.push(DetectedFramework::Gin { version });
        } else if trimmed.contains("labstack/echo") {
            frameworks.push(DetectedFramework::Echo { version });
        } else if trimmed.contains("gofiber/fiber") {
            frameworks.push(DetectedFramework::Fiber { version });
        }
    }

    frameworks
}
