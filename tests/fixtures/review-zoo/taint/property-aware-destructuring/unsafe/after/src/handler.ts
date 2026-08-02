export function runCommand(req: any) {
  const body = req.body;
  const { command } = body;
  return exec(command);
}
