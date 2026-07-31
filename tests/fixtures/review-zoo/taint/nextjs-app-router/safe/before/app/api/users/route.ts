import type { NextRequest } from "next/server";

export async function GET(_request: NextRequest) {
  const id = "system";
  return db.query("SELECT * FROM users WHERE id = $1", [id]);
}
