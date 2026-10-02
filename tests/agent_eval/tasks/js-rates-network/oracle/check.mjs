// Hidden check: the code converts correctly, and the repository's own tests
// run offline, skip nothing, and fail against a broken implementation.
import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";

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

const hidden = node(["--test", "oracle/rates.oracle.test.js"]);
if (hidden.status !== 0) fail("convert() is wrong", hidden.stdout);

const own = node(["--test", "--test-reporter=tap", "test/**/*.test.js"]);
if (own.status !== 0) fail("the repository's tests fail offline", own.stdout);
if (count(own.stdout, "pass") < 2) fail("fewer than two tests still pass", own.stdout);
if (count(own.stdout, "skip") + count(own.stdout, "todo") > 0) fail("tests are skipped", own.stdout);

writeFileSync("src/rates.js", "export async function convert(amount) {\n  return amount;\n}\n");
const mutant = node(["--test", "test/**/*.test.js"]);
if (mutant.status === 0) fail("the repository's tests pass against a broken convert()", mutant.stdout);
console.log("ok");
