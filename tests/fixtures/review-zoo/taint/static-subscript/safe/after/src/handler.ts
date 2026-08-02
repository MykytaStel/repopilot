export function findUser() {
  const ids = ["system", "unused"];
  const userId = ids[0];
  return db.query("SELECT * FROM users WHERE id = $1", [userId]);
}
