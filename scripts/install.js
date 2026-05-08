#!/usr/bin/env node

const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const ROOT = path.join(__dirname, "..");
const VENDOR_DIR = path.join(ROOT, "vendor");
const PACKAGE_JSON = require(path.join(ROOT, "package.json"));

function targetTriple(platform = process.platform, arch = process.arch) {
  if (platform === "darwin" && arch === "x64") return "x86_64-apple-darwin";
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "linux" && arch === "x64") return "x86_64-unknown-linux-gnu";
  if (platform === "linux" && arch === "arm64") return "aarch64-unknown-linux-gnu";
  if (platform === "win32" && arch === "x64") return "x86_64-pc-windows-msvc";
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

function archiveExt(target) {
  return target.endsWith("windows-msvc") ? "zip" : "tar.gz";
}

function assetName(version, target) {
  return `repopilot-v${version}-${target}.${archiveExt(target)}`;
}

function releaseUrl(version, file) {
  return `https://github.com/MykytaStel/repopilot/releases/download/v${version}/${file}`;
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destination);
    https
      .get(url, (response) => {
        if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          file.close();
          fs.rmSync(destination, { force: true });
          download(response.headers.location, destination).then(resolve, reject);
          return;
        }
        if (response.statusCode !== 200) {
          file.close();
          fs.rmSync(destination, { force: true });
          reject(new Error(`Download failed (${response.statusCode}): ${url}`));
          return;
        }
        response.pipe(file);
        file.on("finish", () => file.close(resolve));
      })
      .on("error", (error) => {
        file.close();
        fs.rmSync(destination, { force: true });
        reject(error);
      });
  });
}

function sha256(file) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(file));
  return hash.digest("hex");
}

function expectedChecksum(checksumText) {
  const first = checksumText.trim().split(/\s+/)[0];
  if (!/^[a-fA-F0-9]{64}$/.test(first)) {
    throw new Error("Invalid checksum file format");
  }
  return first.toLowerCase();
}

function verifyChecksum(archivePath, checksumText) {
  const expected = expectedChecksum(checksumText);
  const actual = sha256(archivePath);
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${path.basename(archivePath)}: expected ${expected}, got ${actual}`);
  }
}

function extractArchive(archivePath, destination, target) {
  fs.rmSync(destination, { recursive: true, force: true });
  fs.mkdirSync(destination, { recursive: true });

  const result = target.endsWith("windows-msvc")
    ? spawnSync("powershell", ["-NoProfile", "-Command", `Expand-Archive -Force '${archivePath}' '${destination}'`], { stdio: "inherit" })
    : spawnSync("tar", ["-xzf", archivePath, "-C", destination], { stdio: "inherit" });

  if (result.status !== 0) {
    throw new Error(`Failed to extract ${archivePath}`);
  }
}

async function install() {
  if (process.env.REPOPILOT_SKIP_DOWNLOAD === "1" || process.env.REPOPILOT_SKIP_DOWNLOAD === "true") {
    console.log("Skipping RepoPilot binary download because REPOPILOT_SKIP_DOWNLOAD is set.");
    return;
  }

  const target = targetTriple();
  const version = PACKAGE_JSON.version;
  const archive = assetName(version, target);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-"));
  const archivePath = path.join(tmp, archive);
  const checksumPath = `${archivePath}.sha256`;
  const extractDir = path.join(tmp, "extract");
  const binaryName = target.endsWith("windows-msvc") ? "repopilot.exe" : "repopilot";

  try {
    await download(releaseUrl(version, archive), archivePath);
    await download(releaseUrl(version, `${archive}.sha256`), checksumPath);
    verifyChecksum(archivePath, fs.readFileSync(checksumPath, "utf8"));
    extractArchive(archivePath, extractDir, target);

    fs.mkdirSync(VENDOR_DIR, { recursive: true });
    fs.copyFileSync(path.join(extractDir, binaryName), path.join(VENDOR_DIR, binaryName));
    fs.chmodSync(path.join(VENDOR_DIR, binaryName), 0o755);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

if (require.main === module) {
  install().catch((error) => {
    console.error(`Failed to install RepoPilot binary: ${error.message}`);
    console.error("Set REPOPILOT_SKIP_DOWNLOAD=1 to skip download or REPOPILOT_BINARY_PATH to use an existing binary.");
    process.exit(1);
  });
}

module.exports = {
  archiveExt,
  assetName,
  expectedChecksum,
  targetTriple,
  verifyChecksum,
};
