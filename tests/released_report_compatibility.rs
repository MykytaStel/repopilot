use repopilot::api::report::parse_scan_summary_json;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE_FIXTURES: &str = "tests/fixtures/reports/releases";

#[derive(Debug, Deserialize)]
struct EvidenceManifest {
    schema_version: u32,
    observed_at: String,
    platform: String,
    capture: CaptureEvidence,
    producers: Vec<ProducerEvidence>,
    readers: Vec<ReaderEvidence>,
    baseline: BaselineEvidence,
}

#[derive(Debug, Deserialize)]
struct CaptureEvidence {
    reader_probe_lock_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ProducerEvidence {
    package_version: String,
    tag: String,
    tag_commit: String,
    asset: String,
    asset_sha256: String,
    report_schema: String,
    fixture: String,
    report_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ReaderEvidence {
    package_version: String,
    crate_sha256: String,
    input_schema: String,
    outcome: String,
}

#[derive(Debug, Deserialize)]
struct BaselineEvidence {
    schema_version: u32,
    fixture: String,
    fixture_sha256: String,
    emitted_by: Vec<String>,
}

#[test]
fn current_reader_accepts_every_version_provenanced_release_report() {
    let root = repository_root().join(RELEASE_FIXTURES);
    let manifest = load_manifest(&root);
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.observed_at, "2026-09-02");
    assert_eq!(manifest.platform, "aarch64-apple-darwin");

    let expected = BTreeMap::from([
        ("0.16.0", "0.18"),
        ("0.17.0", "0.19"),
        ("0.19.0", "0.20"),
        ("0.20.0", "0.23"),
        ("0.21.0", "0.23"),
        ("0.22.0", "0.26"),
    ]);
    assert_eq!(manifest.producers.len(), expected.len());

    for producer in &manifest.producers {
        assert_eq!(producer.tag, format!("v{}", producer.package_version));
        assert_eq!(producer.tag_commit.len(), 40);
        assert_eq!(producer.asset_sha256.len(), 64);
        assert!(producer.asset.contains(&producer.package_version));
        assert_eq!(
            expected.get(producer.package_version.as_str()),
            Some(&producer.report_schema.as_str())
        );

        let fixture = root.join(&producer.fixture);
        let bytes = fs::read(&fixture).expect("released report fixture");
        assert_eq!(sha256(&bytes), producer.report_sha256);
        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("report JSON");
        assert_eq!(value["repopilot_version"], producer.package_version);
        assert_eq!(value["schema_version"], producer.report_schema);
        assert_eq!(value["report"]["kind"], "scan");

        let parsed = parse_scan_summary_json(std::str::from_utf8(&bytes).unwrap())
            .expect("current reader must accept released report");
        assert_eq!(parsed.metrics.files_analyzed, 2);
        assert_eq!(parsed.artifacts.findings.len(), 1);
    }
}

#[test]
fn manifest_pins_released_reader_and_baseline_evidence() {
    let root = repository_root().join(RELEASE_FIXTURES);
    let manifest = load_manifest(&root);
    let reader_outcomes = manifest
        .readers
        .iter()
        .map(|reader| {
            assert_eq!(reader.crate_sha256.len(), 64);
            assert_eq!(reader.input_schema, "0.26");
            (reader.package_version.as_str(), reader.outcome.as_str())
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        reader_outcomes,
        BTreeMap::from([
            ("0.20.0", "rejected-unsupported-schema"),
            ("0.21.0", "rejected-unsupported-schema"),
            ("0.22.0", "accepted"),
        ])
    );

    assert_eq!(manifest.baseline.schema_version, 1);
    assert_eq!(manifest.baseline.emitted_by, ["0.20.0", "0.22.0"]);
    let baseline = fs::read(root.join(&manifest.baseline.fixture)).expect("released baseline");
    assert_eq!(sha256(&baseline), manifest.baseline.fixture_sha256);

    let lock = fs::read(root.join("reader-probe/Cargo.lock")).expect("reader probe lock");
    assert_eq!(sha256(&lock), manifest.capture.reader_probe_lock_sha256);
    let lock: toml::Value = toml::from_str(std::str::from_utf8(&lock).unwrap()).unwrap();
    let packages = lock["package"].as_array().expect("Cargo.lock packages");
    for reader in &manifest.readers {
        let package = packages
            .iter()
            .find(|package| {
                package["name"].as_str() == Some("repopilot")
                    && package["version"].as_str() == Some(reader.package_version.as_str())
            })
            .expect("exact released reader in probe lock");
        assert_eq!(
            package["checksum"].as_str(),
            Some(reader.crate_sha256.as_str())
        );
    }
}

#[test]
fn released_baseline_survives_line_movement_without_false_resolution() {
    let repo = tempfile::tempdir().expect("temporary repository");
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::create_dir_all(repo.path().join(".repopilot")).unwrap();
    fs::write(
        repo.path().join("src/config.py"),
        "# moved down one line\nAPI_TOKEN = \"sk_live_repopilot_12345678901234567890\"\n",
    )
    .unwrap();
    fs::copy(
        repository_root()
            .join(RELEASE_FIXTURES)
            .join("baseline-v0200-v0220-schema1.json"),
        repo.path().join(".repopilot/baseline.json"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_repopilot"))
        .current_dir(repo.path())
        .args([
            "scan",
            ".",
            "--baseline",
            ".repopilot/baseline.json",
            "--format",
            "json",
        ])
        .output()
        .expect("run current scan");
    assert!(
        output.status.success(),
        "scan failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["baseline"]["existing_findings"], 1);
    assert_eq!(report["baseline"]["new_findings"], 0);
    assert_eq!(report["baseline"]["resolved_findings"], 0);
    assert_eq!(report["findings"][0]["baseline_status"], "existing");
    assert!(report.get("resolved").is_none());
}

#[test]
#[ignore = "explicit compatibility gate: compiles exact released crates"]
fn exact_released_readers_match_recorded_schema_outcomes() {
    let root = repository_root().join(RELEASE_FIXTURES);
    let output = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args([
            "run",
            "--quiet",
            "--locked",
            "--manifest-path",
            "reader-probe/Cargo.toml",
            "--",
            "scan-v0220-schema026.json",
        ])
        .output()
        .expect("run exact released-reader probe");
    assert!(
        output.status.success(),
        "reader probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "0.20.0 REJECT unsupported scan report schema; expected scan report schema 0.23\n",
            "0.21.0 REJECT unsupported scan report schema; expected scan report schema 0.23\n",
            "0.22.0 ACCEPT\n",
        )
    );
}

fn load_manifest(root: &Path) -> EvidenceManifest {
    let bytes = fs::read(root.join("manifest.json")).expect("compatibility evidence manifest");
    serde_json::from_slice(&bytes).expect("valid compatibility manifest")
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").unwrap();
            output
        })
}
