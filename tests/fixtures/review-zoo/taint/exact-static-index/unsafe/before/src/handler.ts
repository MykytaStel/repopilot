export function runSecondCommand() {
  const items = ["echo safe", "echo ready"];
  items[0] = "echo safe";
  return exec("echo ready");
}
