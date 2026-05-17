#!/usr/bin/env node

const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const ROOT = path.join(__dirname, "..");
const ROOT_PACKAGE = require(path.join(ROOT, "package.json"));

const TARGETS = [
  {
    target: "aarch64-apple-darwin",
    packageName: "@repopilot/darwin-arm64",
    description: "Native RepoPilot binary for macOS ARM64.",
    os: ["darwin"],
    cpu: ["arm64"],
    binary: "repopilot",
    archiveExt: "tar.gz",
  },
  {
    target: "x86_64-apple-darwin",
    packageName: "@repopilot/darwin-x64",
    description: "Native RepoPilot binary for macOS x64.",
    os: ["darwin"],
    cpu: ["x64"],
    binary: "repopilot",
    archiveExt: "tar.gz",
  },
  {
    target: "aarch64-unknown-linux-gnu",
    packageName: "@repopilot/linux-arm64-gnu",
    description: "Native RepoPilot binary for Linux ARM64 glibc.",
    os: ["linux"],
    cpu: ["arm64"],
    libc: ["glibc"],
    binary: "repopilot",
    archiveExt: "tar.gz",
  },
  {
    target: "x86_64-unknown-linux-gnu",
    packageName: "@repopilot/linux-x64-gnu",
    description: "Native RepoPilot binary for Linux x64 glibc.",
    os: ["linux"],
    cpu: ["x64"],
    libc: ["glibc"],
    binary: "repopilot",
    archiveExt: "tar.gz",
  },
  {
    target: "x86_64-pc-windows-msvc",
    packageName: "@repopilot/win32-x64-msvc",
    description: "Native RepoPilot binary for Windows x64 MSVC.",
    os: ["win32"],
    cpu: ["x64"],
    binary: "repopilot.exe",
    archiveExt: "zip",
  },
];

function parseArgs(argv) {
  const options = {
    dist: path.join(ROOT, "dist"),
    out: path.join(ROOT, "dist", "npm-platform-packages"),
    version: ROOT_PACKAGE.version,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--dist") {
      options.dist = path.resolve(argv[++i]);
    } else if (arg === "--out") {
      options.out = path.resolve(argv[++i]);
    } else if (arg === "--version") {
      options.version = argv[++i];
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  return options;
}

function archiveName(version, target) {
  return `repopilot-v${version}-${target.target}.${target.archiveExt}`;
}

function expectedChecksum(checksumText) {
  const first = checksumText.trim().split(/\s+/)[0];
  if (!/^[a-fA-F0-9]{64}$/.test(first)) {
    throw new Error("Invalid checksum file format");
  }
  return first.toLowerCase();
}

function sha256(file) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(file));
  return hash.digest("hex");
}

function verifyChecksum(archivePath, checksumText) {
  const expected = expectedChecksum(checksumText);
  const actual = sha256(archivePath);
  if (actual !== expected) {
    throw new Error(`Checksum mismatch for ${path.basename(archivePath)}: expected ${expected}, got ${actual}`);
  }
  return expected;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(" ")}`);
  }
}

function extractArchive(archivePath, destination, target) {
  fs.rmSync(destination, { recursive: true, force: true });
  fs.mkdirSync(destination, { recursive: true });

  if (target.archiveExt === "zip") {
    run("unzip", ["-q", archivePath, "-d", destination]);
  } else {
    run("tar", ["-xzf", archivePath, "-C", destination]);
  }
}

function findExtractedBinary(extractDir, binary) {
  const direct = path.join(extractDir, binary);
  if (fs.existsSync(direct)) {
    return direct;
  }

  const entries = fs.readdirSync(extractDir, { withFileTypes: true });
  for (const entry of entries) {
    const entryPath = path.join(extractDir, entry.name);
    if (entry.isDirectory()) {
      const nested = findExtractedBinary(entryPath, binary);
      if (nested) {
        return nested;
      }
    } else if (entry.name === binary) {
      return entryPath;
    }
  }
  return undefined;
}

function packageDirectory(outDir, packageName) {
  return path.join(outDir, ...packageName.split("/"));
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function packageJson(version, target) {
  const manifest = {
    name: target.packageName,
    version,
    description: target.description,
    license: ROOT_PACKAGE.license,
    repository: ROOT_PACKAGE.repository,
    homepage: ROOT_PACKAGE.homepage,
    bugs: ROOT_PACKAGE.bugs,
    os: target.os,
    cpu: target.cpu,
    engines: ROOT_PACKAGE.engines,
    files: ["bin", "README.md", "repopilot-native.json"],
  };

  if (target.libc) {
    manifest.libc = target.libc;
  }
  return manifest;
}

function readme(version, target) {
  return `# ${target.packageName}

This package contains the native RepoPilot binary for ${target.target}.

It is published as an optional dependency of \`repopilot@${version}\`. Users should install \`repopilot\`, not this package directly.

This package has no install scripts and performs no network downloads during installation.
`;
}

function buildPlatformPackage(distDir, outDir, version, target) {
  const archive = archiveName(version, target);
  const archivePath = path.join(distDir, archive);
  const checksumPath = `${archivePath}.sha256`;

  if (!fs.existsSync(archivePath)) {
    throw new Error(`Missing release archive: ${archivePath}`);
  }
  if (!fs.existsSync(checksumPath)) {
    throw new Error(`Missing release checksum: ${checksumPath}`);
  }

  const checksum = verifyChecksum(archivePath, fs.readFileSync(checksumPath, "utf8"));
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-npm-"));

  try {
    const extractDir = path.join(tmp, "extract");
    extractArchive(archivePath, extractDir, target);

    const binaryPath = findExtractedBinary(extractDir, target.binary);
    if (!binaryPath) {
      throw new Error(`Archive ${archive} does not contain ${target.binary}`);
    }

    const packageDir = packageDirectory(outDir, target.packageName);
    const binDir = path.join(packageDir, "bin");
    fs.rmSync(packageDir, { recursive: true, force: true });
    fs.mkdirSync(binDir, { recursive: true });

    const packagedBinary = path.join(binDir, target.binary);
    fs.copyFileSync(binaryPath, packagedBinary);
    fs.chmodSync(packagedBinary, 0o755);
    writeJson(path.join(packageDir, "package.json"), packageJson(version, target));
    fs.writeFileSync(path.join(packageDir, "README.md"), readme(version, target));
    writeJson(path.join(packageDir, "repopilot-native.json"), {
      packageName: target.packageName,
      version,
      target: target.target,
      binary: `bin/${target.binary}`,
      sourceArchive: archive,
      sourceArchiveSha256: checksum,
    });

    return {
      packageName: target.packageName,
      target: target.target,
      directory: packageDir,
      archive,
      archiveSha256: checksum,
    };
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

function buildPlatformPackages(options = {}) {
  const distDir = path.resolve(options.dist ?? path.join(ROOT, "dist"));
  const outDir = path.resolve(options.out ?? path.join(ROOT, "dist", "npm-platform-packages"));
  const version = options.version ?? ROOT_PACKAGE.version;

  fs.rmSync(outDir, { recursive: true, force: true });
  fs.mkdirSync(outDir, { recursive: true });

  return TARGETS.map((target) => buildPlatformPackage(distDir, outDir, version, target));
}

function main() {
  const packages = buildPlatformPackages(parseArgs(process.argv.slice(2)));
  process.stdout.write(`${JSON.stringify(packages, null, 2)}\n`);
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}

module.exports = {
  TARGETS,
  archiveName,
  buildPlatformPackages,
  expectedChecksum,
  findExtractedBinary,
  packageJson,
  verifyChecksum,
};
