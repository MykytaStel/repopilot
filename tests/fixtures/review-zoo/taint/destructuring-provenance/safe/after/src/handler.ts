export function findUser(req: any) {
  const { id: userId } = req.body;
  return db.query("SELECT * FROM users WHERE id = $1", [userId]);
}
