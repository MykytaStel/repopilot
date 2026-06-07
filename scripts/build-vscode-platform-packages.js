#!/usr/bin/env node

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");
const {
  TARGETS,
  archiveName,
  findExtractedBinary,
  verifyChecksum,
} = require("./build-npm-platform-packages");

const ROOT = path.join(__dirname, "..");
const SOURCE = path.join(ROOT, "editors", "vscode");
const VSCE_TARGETS = {
  "aarch64-apple-darwin": "darwin-arm64",
  "x86_64-apple-darwin": "darwin-x64",
  "aarch64-unknown-linux-gnu": "linux-arm64",
  "x86_64-unknown-linux-gnu": "linux-x64",
  "x86_64-pc-windows-msvc": "win32-x64",
};

function parseArgs(argv) {
  const options = {
    dist: path.join(ROOT, "dist"),
    out: path.join(ROOT, "dist", "vscode"),
    targets: [],
    version: require(path.join(SOURCE, "package.json")).version,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--dist") options.dist = path.resolve(argv[++index]);
    else if (arg === "--out") options.out = path.resolve(argv[++index]);
    else if (arg === "--target") options.targets.push(argv[++index]);
    else if (arg === "--version") options.version = argv[++index];
    else throw new Error(`Unknown argument: ${arg}`);
  }
  return options;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.status !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(" ")}`);
  }
}

function sha256(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function buildTarget(options, target) {
  const archive = archiveName(options.version, target);
  const archivePath = path.join(options.dist, archive);
  const checksumPath = `${archivePath}.sha256`;
  const checksum = verifyChecksum(
    archivePath,
    fs.readFileSync(checksumPath, "utf8"),
  );
  const stage = fs.mkdtempSync(path.join(os.tmpdir(), "repopilot-vscode-"));
  try {
    fs.cpSync(SOURCE, stage, {
      recursive: true,
      filter(source) {
        const relative = path.relative(SOURCE, source);
        const topLevel = relative.split(path.sep)[0];
        return !["node_modules", "dist", ".vscode-test", "bin"].includes(topLevel);
      },
    });
    fs.copyFileSync(path.join(ROOT, "LICENSE"), path.join(stage, "LICENSE"));
    const extract = path.join(stage, ".binary");
    fs.mkdirSync(extract);
    if (target.archiveExt === "zip") {
      run("unzip", ["-q", archivePath, "-d", extract]);
    } else {
      run("tar", ["-xzf", archivePath, "-C", extract]);
    }
    const binary = findExtractedBinary(extract, target.binary);
    if (!binary) throw new Error(`${archive} does not contain ${target.binary}`);
    const binDir = path.join(stage, "bin");
    fs.mkdirSync(binDir, { recursive: true });
    fs.copyFileSync(binary, path.join(binDir, target.binary));
    fs.chmodSync(path.join(binDir, target.binary), 0o755);
    fs.rmSync(extract, { recursive: true, force: true });

    const manifestPath = path.join(stage, "package.json");
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    manifest.version = options.version;
    fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    fs.writeFileSync(
      path.join(stage, "repopilot-native.json"),
      `${JSON.stringify(
        {
          version: options.version,
          target: target.target,
          sourceArchive: archive,
          sourceArchiveSha256: checksum,
        },
        null,
        2,
      )}\n`,
    );

    run("npm", ["ci", "--ignore-scripts"], { cwd: stage });
    run("npm", ["run", "compile"], { cwd: stage });
    const out = path.join(
      options.out,
      `repopilot-vscode-${options.version}-${VSCE_TARGETS[target.target]}.vsix`,
    );
    run(
      "npx",
      [
        "vsce",
        "package",
        "--no-dependencies",
        "--target",
        VSCE_TARGETS[target.target],
        "--out",
        out,
      ],
      { cwd: stage },
    );
    return {
      version: options.version,
      target: target.target,
      binaryArchive: archive,
      binaryArchiveSha256: checksum,
      npmPackage: target.packageName,
      vsix: path.basename(out),
      vsixSha256: sha256(out),
    };
  } finally {
    fs.rmSync(stage, { recursive: true, force: true });
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  fs.rmSync(options.out, { recursive: true, force: true });
  fs.mkdirSync(options.out, { recursive: true });
  const selected = options.targets.length
    ? TARGETS.filter((target) => options.targets.includes(target.target))
    : TARGETS;
  if (selected.length !== (options.targets.length || TARGETS.length)) {
    throw new Error(`Unknown target in: ${options.targets.join(", ")}`);
  }
  const result = selected.map((target) => buildTarget(options, target));
  fs.writeFileSync(
    path.join(options.out, "manifest.json"),
    `${JSON.stringify(result, null, 2)}\n`,
  );
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
