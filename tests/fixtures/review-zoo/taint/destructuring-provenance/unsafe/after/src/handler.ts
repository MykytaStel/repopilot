export function runCommand(req: any) {
  const { command: cmd } = req.body;
  return exec(cmd);
}
