import type { FastifyRequest } from "fastify";

export async function audit(request: FastifyRequest) {
  const origin = request.raw.url;
  return db.query("SELECT * FROM audit_logs WHERE origin = $1", [origin]);
}
