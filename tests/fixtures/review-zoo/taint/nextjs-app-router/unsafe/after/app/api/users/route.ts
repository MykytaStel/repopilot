import type { NextRequest } from "next/server";

export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("id");
  return db.query("SELECT * FROM users WHERE id = " + id);
}
