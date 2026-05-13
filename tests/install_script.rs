#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn install_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh")
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });

    let mut permissions = fs::metadata(path)
        .unwrap_or_else(|error| panic!("failed to stat {}: {error}", path.display()))
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap_or_else(|error| {
        panic!("failed to chmod {}: {error}", path.display());
    });
}

fn fake_curl_script() -> &'static str {
    r#"#!/bin/sh
set -eu

out=""
url=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    -o)
      out="$2"
      shift 2
      ;;
    -*)
      shift
      ;;
    *)
      url="$1"
      shift
      ;;
  esac
done

case "$url" in
  *api.github.com*)
    printf '{"tag_name":"v0.9.0"}\n'
    ;;
  *.sha256)
    if [ "${REPOPILOT_FAKE_CURL_MODE:-}" = "missing-checksum" ]; then
      exit 22
    fi
    printf '%s  repopilot-test.tar.gz\n' "${REPOPILOT_FAKE_EXPECTED_SHA:-0000000000000000000000000000000000000000000000000000000000000000}" > "$out"
    ;;
  *.tar.gz)
    if [ "${REPOPILOT_FAKE_CURL_MODE:-}" = "success" ]; then
      cat "$REPOPILOT_FAKE_ARCHIVE" > "$out"
    else
      printf 'not a tarball\n' > "$out"
    fi
    ;;
  *)
    exit 2
    ;;
esac
"#
}

fn fake_sha256sum_script() -> &'static str {
    r#"#!/bin/sh
set -eu

printf '%s  %s\n' "${REPOPILOT_FAKE_ACTUAL_SHA:-1111111111111111111111111111111111111111111111111111111111111111}" "$1"
"#
}

fn run_install_with_path(path: &str, mode: &str) -> Output {
    let temp = tempfile::tempdir().expect("failed to create temp dir");

    Command::new(find_executable("bash"))
        .arg(install_script())
        .env("PATH", path)
        .env("INSTALL_DIR", temp.path().join("bin"))
        .env("REPOPILOT_FAKE_CURL_MODE", mode)
        .output()
        .expect("failed to run install.sh")
}

fn path_with_fake_tools(fake_tools_dir: &Path) -> String {
    let old_path = env::var("PATH").unwrap_or_default();
    format!("{}:{old_path}", fake_tools_dir.display())
}

fn find_executable(name: &str) -> PathBuf {
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| panic!("failed to find required test tool: {name}"))
}

fn isolated_path_without_sha_tools(fake_tools_dir: &Path) -> String {
    for tool in ["awk", "basename", "grep", "mktemp", "rm", "sed", "uname"] {
        symlink(find_executable(tool), fake_tools_dir.join(tool))
            .unwrap_or_else(|error| panic!("failed to link {tool}: {error}"));
    }

    fake_tools_dir.display().to_string()
}

fn create_release_archive(root: &Path) -> PathBuf {
    let payload = root.join("payload");
    fs::create_dir(&payload).expect("failed to create payload dir");
    write_executable(
        &payload.join("repopilot"),
        "#!/bin/sh\nprintf 'repopilot 0.9.0\\n'\n",
    );

    let archive = root.join("repopilot.tar.gz");
    let output = Command::new(find_executable("tar"))
        .args([
            "-czf",
            archive.to_str().expect("non-utf8 archive path"),
            "-C",
            payload.to_str().expect("non-utf8 payload path"),
            "repopilot",
        ])
        .output()
        .expect("failed to create release archive");

    assert!(
        output.status.success(),
        "failed to create release archive\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    archive
}

#[test]
fn install_script_passes_bash_syntax_check() {
    let output = Command::new(find_executable("bash"))
        .arg("-n")
        .arg(install_script())
        .output()
        .expect("failed to run bash -n install.sh");

    assert!(
        output.status.success(),
        "install.sh should pass bash -n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn install_script_aborts_when_checksum_download_fails() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("failed to create fake bin");
    write_executable(&fake_bin.join("curl"), fake_curl_script());

    let output = run_install_with_path(&path_with_fake_tools(&fake_bin), "missing-checksum");

    assert!(
        !output.status.success(),
        "install.sh should fail when checksum download fails"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to download required checksum file"),
        "stderr should name the missing checksum\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Installation aborted for safety"),
        "stderr should explain the safety abort\nstderr:\n{stderr}"
    );
}

#[test]
fn install_script_installs_when_checksum_verification_succeeds() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let fake_bin = temp.path().join("bin");
    let install_dir = temp.path().join("install");
    let archive = create_release_archive(temp.path());
    let fake_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fs::create_dir(&fake_bin).expect("failed to create fake bin");
    write_executable(&fake_bin.join("curl"), fake_curl_script());
    write_executable(&fake_bin.join("sha256sum"), fake_sha256sum_script());

    let output = Command::new(find_executable("bash"))
        .arg(install_script())
        .env("PATH", path_with_fake_tools(&fake_bin))
        .env("INSTALL_DIR", &install_dir)
        .env("REPOPILOT_FAKE_CURL_MODE", "success")
        .env("REPOPILOT_FAKE_ARCHIVE", archive)
        .env("REPOPILOT_FAKE_EXPECTED_SHA", fake_sha)
        .env("REPOPILOT_FAKE_ACTUAL_SHA", fake_sha)
        .output()
        .expect("failed to run install.sh");

    assert!(
        output.status.success(),
        "install.sh should install when checksum verification succeeds\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        install_dir.join("repopilot").is_file(),
        "installer should place repopilot in INSTALL_DIR"
    );
}

#[test]
fn install_script_aborts_when_sha256_tool_is_missing() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("failed to create fake bin");
    write_executable(&fake_bin.join("curl"), fake_curl_script());

    let output = run_install_with_path(
        &isolated_path_without_sha_tools(&fake_bin),
        "invalid-checksum",
    );

    assert!(
        !output.status.success(),
        "install.sh should fail when no SHA256 tool is available"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No SHA256 verification tool found"),
        "stderr should explain the missing SHA256 tool\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Installation aborted for safety"),
        "stderr should explain the safety abort\nstderr:\n{stderr}"
    );
}

#[test]
fn install_script_aborts_when_checksum_verification_fails() {
    let temp = tempfile::tempdir().expect("failed to create temp dir");
    let fake_bin = temp.path().join("bin");
    fs::create_dir(&fake_bin).expect("failed to create fake bin");
    write_executable(&fake_bin.join("curl"), fake_curl_script());
    write_executable(&fake_bin.join("sha256sum"), fake_sha256sum_script());

    let output = run_install_with_path(&path_with_fake_tools(&fake_bin), "invalid-checksum");

    assert!(
        !output.status.success(),
        "install.sh should fail when checksum verification fails"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("SHA256 verification failed"),
        "stderr should explain the checksum mismatch\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Installation aborted for safety"),
        "stderr should explain the safety abort\nstderr:\n{stderr}"
    );
}
