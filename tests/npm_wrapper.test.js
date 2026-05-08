const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");

const launcher = path.join(__dirname, "..", "bin", "repopilot.js");
const installer = require("../scripts/install.js");

test("maps supported platforms to release targets", () => {
  assert.equal(installer.targetTriple("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(installer.targetTriple("darwin", "x64"), "x86_64-apple-darwin");
  assert.equal(installer.targetTriple("linux", "x64"), "x86_64-unknown-linux-gnu");
  assert.equal(installer.targetTriple("linux", "arm64"), "aarch64-unknown-linux-gnu");
  assert.equal(installer.targetTriple("win32", "x64"), "x86_64-pc-windows-msvc");
});

test("builds release asset names", () => {
  assert.equal(
    installer.assetName("0.6.0", "aarch64-apple-darwin"),
    "repopilot-v0.6.0-aarch64-apple-darwin.tar.gz"
  );
  assert.equal(
    installer.assetName("0.6.0", "x86_64-pc-windows-msvc"),
    "repopilot-v0.6.0-x86_64-pc-windows-msvc.zip"
  );
});

test("verifies sha256 checksum text", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-test-"));
  const file = path.join(tmp, "archive.tar.gz");
  fs.writeFileSync(file, "archive");
  const hash = crypto.createHash("sha256").update("archive").digest("hex");

  assert.doesNotThrow(() => installer.verifyChecksum(file, `${hash}  archive.tar.gz\n`));
  assert.throws(() => installer.verifyChecksum(file, `${"0".repeat(64)}  archive.tar.gz\n`), /Checksum mismatch/);

  fs.rmSync(tmp, { recursive: true, force: true });
});

test("launcher reports a missing binary clearly", () => {
  const result = spawnSync(process.execPath, [launcher], {
    env: {
      ...process.env,
      REPOPILOT_BINARY_PATH: path.join(os.tmpdir(), "missing-repopilot-binary"),
    },
    encoding: "utf8",
  });

  assert.equal(result.status, 1);
  assert.match(result.stderr, /RepoPilot binary was not found/);
  assert.match(result.stderr, /REPOPILOT_BINARY_PATH/);
});

test("launcher uses REPOPILOT_BINARY_PATH override", () => {
  const result = spawnSync(process.execPath, [launcher, "--version"], {
    env: {
      ...process.env,
      REPOPILOT_BINARY_PATH: process.execPath,
    },
    encoding: "utf8",
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /^v?\d+\./);
});
