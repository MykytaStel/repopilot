export function runFirstCommand(req: any) {
  const commands = req.body.commands;
  const command = commands[0];
  return exec(command);
}
