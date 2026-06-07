import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  const extensionDevelopmentPath = path.resolve(__dirname, "../../");
  const extensionTestsPath = path.resolve(__dirname, "./suite/index");
  delete process.env.ELECTRON_RUN_AS_NODE;

  await runTests({
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs: ["--disable-workspace-trust"],
  });
}

main().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
