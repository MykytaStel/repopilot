import * as assert from "node:assert";
import * as vscode from "vscode";

suite("RepoPilot extension", () => {
  test("activates and registers the command surface", async () => {
    const extension = vscode.extensions.all.find(
      (candidate) => candidate.packageJSON.name === "repopilot-vscode",
    );
    assert.ok(extension, "RepoPilot extension should be installed in the test host");
    await extension.activate();

    const commands = new Set(await vscode.commands.getCommands(true));
    for (const command of [
      "repopilot.scanWorkspace",
      "repopilot.reviewChanges",
      "repopilot.explainCurrentFile",
      "repopilot.refreshDiagnostics",
      "repopilot.openReport",
    ]) {
      assert.ok(commands.has(command), `missing command ${command}`);
    }
  });
});
