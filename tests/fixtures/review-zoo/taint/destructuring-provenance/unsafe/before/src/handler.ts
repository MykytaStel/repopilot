export function runCommand(req: any) {
  const command = "echo safe";
  return exec(command);
}
