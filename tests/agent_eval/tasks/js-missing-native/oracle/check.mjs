// Hidden check, with a recording stand-in for sharp installed: the code
// still drives sharp as documented, and the repository's own tests run,
// skip nothing, and still exercise thumbnail().
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

rmSync("node_modules/sharp", { recursive: true, force: true });
mkdirSync("node_modules", { recursive: true });
cpSync("oracle/fake-sharp", "node_modules/sharp", { recursive: true });

const hidden = node(["--test", "oracle/thumbnails.oracle.test.js"]);
if (hidden.status !== 0) fail("thumbnail() no longer drives sharp as documented", hidden.stdout);

const own = node(["--test", "--test-reporter=tap", "test/**/*.test.js"]);
if (own.status !== 0) fail("the repository's tests fail with sharp available", own.stdout);
if (count(own.stdout, "pass") < 2) fail("fewer than two tests still pass", own.stdout);
if (count(own.stdout, "skip") + count(own.stdout, "todo") > 0) fail("tests are skipped", own.stdout);
console.log("ok");
