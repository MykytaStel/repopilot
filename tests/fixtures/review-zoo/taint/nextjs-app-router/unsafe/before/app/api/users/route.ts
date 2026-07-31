import type { NextRequest } from "next/server";

export async function GET(_request: NextRequest) {
  return db.query("SELECT * FROM users WHERE active = true");
}
