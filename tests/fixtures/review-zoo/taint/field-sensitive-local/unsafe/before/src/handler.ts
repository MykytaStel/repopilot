export function find(req: any) {
  const target = { id: "system" };
  return db.query("SELECT * FROM users WHERE id = $1", [target.id]);
}
