const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const test = require("node:test");

const launcher = path.join(__dirname, "..", "bin", "repopilot.js");
const launcherModule = require("../bin/repopilot.js");
const npmPackages = require("../scripts/build-npm-platform-packages.js");

function checksumFile(file) {
  const checksum = crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
  fs.writeFileSync(`${file}.sha256`, `${checksum}  ${path.basename(file)}\n`);
  return checksum;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: "utf8", ...options });
  assert.equal(result.status, 0, `${command} ${args.join(" ")}\n${result.stderr}`);
  return result;
}

function writeReleaseArchive(dist, version, target) {
  const sourceDir = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-archive-"));
  const binary = path.join(sourceDir, target.binary);
  fs.writeFileSync(binary, `binary for ${target.target}`);
  fs.chmodSync(binary, 0o755);

  const archive = path.join(dist, npmPackages.archiveName(version, target));
  if (target.archiveExt === "zip") {
    run("zip", ["-j", archive, binary], { stdio: "pipe" });
  } else {
    run("tar", ["-czf", archive, "-C", sourceDir, target.binary], { stdio: "pipe" });
  }

  fs.rmSync(sourceDir, { recursive: true, force: true });
  return checksumFile(archive);
}

test("maps supported platforms to optional native packages", () => {
  assert.equal(launcherModule.platformPackage("darwin", "arm64").packageName, "@repopilot/darwin-arm64");
  assert.equal(launcherModule.platformPackage("darwin", "x64").packageName, "@repopilot/darwin-x64");
  assert.equal(launcherModule.platformPackage("linux", "x64", "glibc").packageName, "@repopilot/linux-x64-gnu");
  assert.equal(launcherModule.platformPackage("linux", "arm64", "glibc").packageName, "@repopilot/linux-arm64-gnu");
  assert.equal(launcherModule.platformPackage("win32", "x64").packageName, "@repopilot/win32-x64-msvc");
  assert.throws(() => launcherModule.platformPackage("linux", "x64", "musl"), /Unsupported npm platform/);
});

test("builds release asset names", () => {
  const macosArm = npmPackages.TARGETS.find((target) => target.target === "aarch64-apple-darwin");
  const windowsX64 = npmPackages.TARGETS.find((target) => target.target === "x86_64-pc-windows-msvc");

  assert.equal(
    npmPackages.archiveName("0.6.0", macosArm),
    "repopilot-v0.6.0-aarch64-apple-darwin.tar.gz"
  );
  assert.equal(
    npmPackages.archiveName("0.6.0", windowsX64),
    "repopilot-v0.6.0-x86_64-pc-windows-msvc.zip"
  );
});

test("verifies sha256 checksum text", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-test-"));
  const file = path.join(tmp, "archive.tar.gz");
  fs.writeFileSync(file, "archive");
  const hash = crypto.createHash("sha256").update("archive").digest("hex");

  assert.doesNotThrow(() => npmPackages.verifyChecksum(file, `${hash}  archive.tar.gz\n`));
  assert.throws(() => npmPackages.verifyChecksum(file, `${"0".repeat(64)}  archive.tar.gz\n`), /Checksum mismatch/);

  fs.rmSync(tmp, { recursive: true, force: true });
});

test("resolves binary path from optional package", () => {
  const resolved = launcherModule.resolveBinaryPath({
    env: {},
    platform: "darwin",
    arch: "arm64",
    resolver: (request) => {
      assert.equal(request, "@repopilot/darwin-arm64/package.json");
      return "/tmp/project/node_modules/@repopilot/darwin-arm64/package.json";
    },
  });

  assert.equal(resolved, "/tmp/project/node_modules/@repopilot/darwin-arm64/bin/repopilot");
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

test("launcher reports missing optional package clearly", () => {
  const missingModule = new Error("Cannot find module");
  missingModule.code = "MODULE_NOT_FOUND";

  assert.throws(
    () =>
      launcherModule.resolveBinaryPath({
        env: {},
        platform: "darwin",
        arch: "arm64",
        resolver: () => {
          throw missingModule;
        },
      }),
    launcherModule.MissingOptionalPackageError
  );

  const message = launcherModule.formatResolveError(
    new launcherModule.MissingOptionalPackageError("@repopilot/darwin-arm64", missingModule)
  );
  assert.match(message, /optional native package was not installed/);
  assert.match(message, /@repopilot\/darwin-arm64/);
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

test("builds platform npm packages from release archives", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-npm-packages-"));
  const dist = path.join(tmp, "dist");
  const out = path.join(tmp, "out");
  const version = "0.0.0-test.0";
  fs.mkdirSync(dist, { recursive: true });

  const checksums = new Map();
  for (const target of npmPackages.TARGETS) {
    checksums.set(target.target, writeReleaseArchive(dist, version, target));
  }

  const packages = npmPackages.buildPlatformPackages({ dist, out, version });
  assert.equal(packages.length, npmPackages.TARGETS.length);

  const linuxX64 = packages.find((pkg) => pkg.packageName === "@repopilot/linux-x64-gnu");
  const manifest = JSON.parse(fs.readFileSync(path.join(linuxX64.directory, "package.json"), "utf8"));
  const metadata = JSON.parse(fs.readFileSync(path.join(linuxX64.directory, "repopilot-native.json"), "utf8"));

  assert.equal(manifest.name, "@repopilot/linux-x64-gnu");
  assert.deepEqual(manifest.os, ["linux"]);
  assert.deepEqual(manifest.cpu, ["x64"]);
  assert.deepEqual(manifest.libc, ["glibc"]);
  assert.equal(manifest.scripts, undefined);
  assert.equal(metadata.sourceArchiveSha256, checksums.get("x86_64-unknown-linux-gnu"));
  assert.equal(fs.existsSync(path.join(linuxX64.directory, "bin", "repopilot")), true);

  const pack = run("npm", ["pack", "--dry-run", "--json"], {
    cwd: linuxX64.directory,
    stdio: "pipe",
  });
  const files = JSON.parse(pack.stdout)[0].files.map((file) => file.path).sort();
  assert.deepEqual(files, ["README.md", "bin/repopilot", "package.json", "repopilot-native.json"]);

  fs.rmSync(tmp, { recursive: true, force: true });
});

test("root npm package excludes installer scripts", () => {
  const result = run("npm", ["pack", "--dry-run", "--json"], {
    cwd: path.join(__dirname, ".."),
    stdio: "pipe",
  });
  const files = JSON.parse(result.stdout)[0].files.map((file) => file.path);

  assert.equal(files.some((file) => file.startsWith("scripts/")), false);
  assert.equal(files.includes("scripts/install.js"), false);
});
