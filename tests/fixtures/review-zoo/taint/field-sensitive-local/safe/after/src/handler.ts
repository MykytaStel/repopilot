export function find(req: any) {
  const body = req.body;
  body.id = "system";
  return db.query("SELECT * FROM users WHERE id = " + body.id);
}
