export function find(req: any) {
  const target = { id: "system" };
  target.id = req.query.id;
  return db.query("SELECT * FROM users WHERE id = " + target.id);
}
