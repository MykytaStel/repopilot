// Hidden check: thumbnail() fails loudly when sharp is missing or cannot
// decode an image (a thumbnail that is secretly the original photo is a
// bug); with a recording stand-in for sharp, the code still drives sharp as
// documented, and the repository's own tests run and skip nothing.
import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, rmSync } from "node:fs";

function node(args) {
  return spawnSync(process.execPath, args, { encoding: "utf8" });
}

function fail(reason, output = "") {
  console.error(`${reason}\n${output.slice(-1500)}`);
  process.exit(1);
}

function count(output, name) {
  const match = output.match(new RegExp(`^# ${name} (\\d+)`, "m"));
  return match ? Number(match[1]) : 0;
}

function install(stand_in) {
  rmSync("node_modules/sharp", { recursive: true, force: true });
  if (stand_in) {
    mkdirSync("node_modules", { recursive: true });
    cpSync(stand_in, "node_modules/sharp", { recursive: true });
  }
}

// A module that fails to load is a loud failure too.
const probe = `
try {
  const { thumbnail } = await import("./src/thumbnails.js");
  const output = await thumbnail(Buffer.from("photo"));
  console.log("thumbnail() resolved with " + output.length + " bytes");
  process.exit(3);
} catch {
  process.exit(0);
}`;

function mustReject(situation) {
  const run = node(["--input-type=module", "-e", probe]);
  if (run.status !== 0) fail(`thumbnail() hides a failure when ${situation}`, run.stdout + run.stderr);
}

install("oracle/failing-sharp");
mustReject("sharp cannot decode the image");
install(null);
mustReject("sharp is not installed");
install("oracle/fake-sharp");

const hidden = node(["--test", "oracle/thumbnails.oracle.test.js"]);
if (hidden.status !== 0) fail("thumbnail() no longer drives sharp as documented", hidden.stdout);

const own = node(["--test", "--test-reporter=tap", "test/**/*.test.js"]);
if (own.status !== 0) fail("the repository's tests fail with sharp available", own.stdout);
if (count(own.stdout, "pass") < 2) fail("fewer than two tests still pass", own.stdout);
if (count(own.stdout, "skip") + count(own.stdout, "todo") > 0) fail("tests are skipped", own.stdout);
console.log("ok");
