#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawn } = require("node:child_process");

const SUPPORTED_TARGETS = [
  { platform: "darwin", arch: "arm64", packageName: "@repopilot/darwin-arm64", binary: "repopilot" },
  { platform: "darwin", arch: "x64", packageName: "@repopilot/darwin-x64", binary: "repopilot" },
  {
    platform: "linux",
    arch: "arm64",
    libc: "glibc",
    packageName: "@repopilot/linux-arm64-gnu",
    binary: "repopilot",
  },
  {
    platform: "linux",
    arch: "x64",
    libc: "glibc",
    packageName: "@repopilot/linux-x64-gnu",
    binary: "repopilot",
  },
  { platform: "win32", arch: "x64", packageName: "@repopilot/win32-x64-msvc", binary: "repopilot.exe" },
];

class UnsupportedPlatformError extends Error {
  constructor(platform, arch, libc) {
    const runtime = libc ? `${platform}/${arch}/${libc}` : `${platform}/${arch}`;
    super(`Unsupported npm platform: ${runtime}`);
    this.name = "UnsupportedPlatformError";
    this.code = "ERR_REPOPILOT_UNSUPPORTED_PLATFORM";
    this.platform = platform;
    this.arch = arch;
    this.libc = libc;
  }
}

class MissingOptionalPackageError extends Error {
  constructor(packageName, cause) {
    super(`Missing optional native package: ${packageName}`);
    this.name = "MissingOptionalPackageError";
    this.code = "ERR_REPOPILOT_MISSING_OPTIONAL_PACKAGE";
    this.packageName = packageName;
    this.cause = cause;
  }
}

function detectLibc(platform = process.platform) {
  if (platform !== "linux") {
    return undefined;
  }

  const report = typeof process.report?.getReport === "function" ? process.report.getReport() : undefined;
  if (report?.header?.glibcVersionRuntime) {
    return "glibc";
  }
  return "musl";
}

function platformPackage(platform = process.platform, arch = process.arch, libc = detectLibc(platform)) {
  const match = SUPPORTED_TARGETS.find((target) => {
    if (target.platform !== platform || target.arch !== arch) {
      return false;
    }
    return target.platform !== "linux" || target.libc === libc;
  });

  if (!match) {
    throw new UnsupportedPlatformError(platform, arch, platform === "linux" ? libc : undefined);
  }
  return match;
}

function packageRoot(packageName, resolver = require.resolve) {
  try {
    return path.dirname(resolver(`${packageName}/package.json`));
  } catch (error) {
    if (error && error.code === "MODULE_NOT_FOUND") {
      throw new MissingOptionalPackageError(packageName, error);
    }
    throw error;
  }
}

function resolveBinaryPath(options = {}) {
  const env = options.env ?? process.env;
  if (env.REPOPILOT_BINARY_PATH) {
    return env.REPOPILOT_BINARY_PATH;
  }

  const target = platformPackage(options.platform, options.arch, options.libc);
  return path.join(packageRoot(target.packageName, options.resolver), "bin", target.binary);
}

function supportedTargetsText() {
  return SUPPORTED_TARGETS.map((target) => {
    const runtime = target.libc
      ? `${target.platform}/${target.arch}/${target.libc}`
      : `${target.platform}/${target.arch}`;
    return `${runtime} (${target.packageName})`;
  }).join(", ");
}

function formatResolveError(error) {
  if (error.code === "ERR_REPOPILOT_UNSUPPORTED_PLATFORM") {
    return [
      "RepoPilot does not provide an npm native package for this platform.",
      `Detected: ${error.libc ? `${error.platform}/${error.arch}/${error.libc}` : `${error.platform}/${error.arch}`}`,
      `Supported npm targets: ${supportedTargetsText()}`,
      "Install with cargo, Homebrew, GitHub Releases, or set REPOPILOT_BINARY_PATH to an existing repopilot binary.",
    ].join("\n");
  }

  if (error.code === "ERR_REPOPILOT_MISSING_OPTIONAL_PACKAGE") {
    return [
      "RepoPilot optional native package was not installed.",
      `Expected npm package: ${error.packageName}`,
      "Reinstall without omitting optional dependencies, for example: npm install -g repopilot",
      "If your environment blocks optional dependencies, install with cargo or set REPOPILOT_BINARY_PATH to an existing repopilot binary.",
    ].join("\n");
  }

  return `Failed to resolve RepoPilot binary: ${error.message}`;
}

function main() {
  let binary;
  try {
    binary = resolveBinaryPath();
  } catch (error) {
    console.error(formatResolveError(error));
    process.exit(1);
  }

  if (!fs.existsSync(binary)) {
    console.error(
      [
        "RepoPilot binary was not found.",
        `Expected: ${binary}`,
        "Reinstall repopilot, install with cargo, or set REPOPILOT_BINARY_PATH to an existing repopilot binary.",
      ].join("\n")
    );
    process.exit(1);
  }

  const child = spawn(binary, process.argv.slice(2), {
    stdio: "inherit",
    windowsHide: false,
  });

  child.on("error", (error) => {
    console.error(`Failed to launch RepoPilot: ${error.message}`);
    process.exit(1);
  });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 1);
  });
}

if (require.main === module) {
  main();
}

module.exports = {
  SUPPORTED_TARGETS,
  MissingOptionalPackageError,
  UnsupportedPlatformError,
  detectLibc,
  formatResolveError,
  platformPackage,
  resolveBinaryPath,
  supportedTargetsText,
};
