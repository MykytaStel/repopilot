export function runCommand(req: any) {
  const commands = req.body.commands;
  const [cmd] = commands;
  return exec(cmd);
}
