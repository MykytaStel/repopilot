export function runSecondCommand(req: any) {
  const items = req.body.items;
  items[0] = "echo safe";
  return exec(items[1]);
}
