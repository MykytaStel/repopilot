export function runFirstCommand() {
  const commands = ["echo safe"];
  const command = commands[0];
  return exec(command);
}
