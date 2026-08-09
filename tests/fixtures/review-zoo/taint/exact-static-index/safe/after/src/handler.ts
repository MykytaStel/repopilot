export function findUser(req: any) {
  const items = req.body.items;
  items[0] = "system";
  const userId = items[0];
  return db.query("SELECT * FROM users WHERE id = $1", [userId]);
}
