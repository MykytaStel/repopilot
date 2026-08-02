export function runCommand(req: any) {
  const body = req.body;
  return exec("echo safe");
}
