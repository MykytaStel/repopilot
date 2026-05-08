#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawn } = require("node:child_process");

function binaryName() {
  return process.platform === "win32" ? "repopilot.exe" : "repopilot";
}

function bundledBinaryPath() {
  return path.join(__dirname, "..", "vendor", binaryName());
}

function resolveBinaryPath() {
  if (process.env.REPOPILOT_BINARY_PATH) {
    return process.env.REPOPILOT_BINARY_PATH;
  }
  return bundledBinaryPath();
}

function main() {
  const binary = resolveBinaryPath();
  if (!fs.existsSync(binary)) {
    console.error(
      [
        "RepoPilot binary was not found.",
        `Expected: ${binary}`,
        "Run npm install again, install with cargo, or set REPOPILOT_BINARY_PATH to an existing repopilot binary.",
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
  binaryName,
  bundledBinaryPath,
  resolveBinaryPath,
};
