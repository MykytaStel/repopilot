export function find(req: any) {
  const body = req.body;
  return db.query("SELECT * FROM users WHERE id = " + body.id);
}
