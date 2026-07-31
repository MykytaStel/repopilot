export function findUser(req: any) {
  const body = req.body;
  const { id } = body;
  return db.query("SELECT * FROM users WHERE id = $1", [id]);
}
