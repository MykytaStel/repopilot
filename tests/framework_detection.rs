use repopilot::frameworks::detector::detect_frameworks;
use repopilot::frameworks::types::DetectedFramework;
use std::fs;
use tempfile::TempDir;

// ── Python ────────────────────────────────────────────────────────────────────

#[test]
fn detects_django_from_requirements() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("requirements.txt"),
        "django==4.2.0\nrequests==2.31.0\n",
    )
    .unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::Django { version: Some(v) } if v == "4.2.0")),
        "expected Django 4.2.0, got: {frameworks:?}"
    );
}

#[test]
fn detects_flask_pinned_version() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("requirements.txt"), "Flask==2.3.1\n").unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::Flask { version: Some(v) } if v == "2.3.1")),
        "expected Flask 2.3.1, got: {frameworks:?}"
    );
}

#[test]
fn detects_fastapi_no_version() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("requirements.txt"), "fastapi\nuvicorn\n").unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::FastApi { version: None })),
        "expected FastAPI with no pinned version, got: {frameworks:?}"
    );
}

#[test]
fn no_python_frameworks_when_no_requirements_txt() {
    let dir = TempDir::new().unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        !frameworks.iter().any(|f| matches!(
            f,
            DetectedFramework::Django { .. }
                | DetectedFramework::Flask { .. }
                | DetectedFramework::FastApi { .. }
        )),
        "no Python frameworks expected without requirements.txt: {frameworks:?}"
    );
}

// ── Go ────────────────────────────────────────────────────────────────────────

#[test]
fn detects_gin_from_go_mod() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/app\n\ngo 1.21\n\nrequire github.com/gin-gonic/gin v1.9.1\n",
    )
    .unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::Gin { version: Some(v) } if v == "1.9.1")),
        "expected Gin 1.9.1, got: {frameworks:?}"
    );
}

#[test]
fn detects_echo_from_go_mod() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/app\n\ngo 1.21\n\nrequire github.com/labstack/echo/v4 v4.11.3\n",
    )
    .unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::Echo { .. })),
        "expected Echo, got: {frameworks:?}"
    );
}

#[test]
fn detects_fiber_from_go_mod() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/app\n\ngo 1.21\n\nrequire github.com/gofiber/fiber/v2 v2.51.0\n",
    )
    .unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        frameworks
            .iter()
            .any(|f| matches!(f, DetectedFramework::Fiber { .. })),
        "expected Fiber, got: {frameworks:?}"
    );
}

#[test]
fn no_go_frameworks_when_no_go_mod() {
    let dir = TempDir::new().unwrap();
    let frameworks = detect_frameworks(dir.path());
    assert!(
        !frameworks.iter().any(|f| matches!(
            f,
            DetectedFramework::Gin { .. }
                | DetectedFramework::Echo { .. }
                | DetectedFramework::Fiber { .. }
        )),
        "no Go frameworks expected without go.mod: {frameworks:?}"
    );
}
