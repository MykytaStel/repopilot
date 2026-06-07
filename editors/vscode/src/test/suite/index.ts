import * as fs from "node:fs";
import * as path from "node:path";
import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: "tdd", color: true });
  const root = __dirname;

  for (const entry of fs.readdirSync(root)) {
    if (entry.endsWith(".test.js")) {
      mocha.addFile(path.join(root, entry));
    }
  }

  return new Promise((resolve, reject) => {
    mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${failures} VS Code extension test(s) failed`));
      } else {
        resolve();
      }
    });
  });
}
