export function runCommand(req: any) {
  const cmd = "echo safe";
  return exec(cmd);
}
