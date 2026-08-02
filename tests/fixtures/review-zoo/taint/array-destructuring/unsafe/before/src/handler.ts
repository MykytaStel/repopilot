export function runCommand() {
  const commands = ["echo safe"];
  const [cmd] = commands;
  return execSafe(cmd);
}
