export function findUser() {
  const [userId] = ["system"];
  return db.query("SELECT * FROM users WHERE id = $1", [userId]);
}
