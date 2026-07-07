export function findUser(req: Request) {
  const id = req.query.id;
  return db.query("SELECT * FROM users WHERE id = " + id);
}
