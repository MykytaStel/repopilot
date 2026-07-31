export function findUser(req: any) {
  const body = req.body;
  const { id: userId } = body;
  return db.query("SELECT * FROM users WHERE id = $1", [userId]);
}
