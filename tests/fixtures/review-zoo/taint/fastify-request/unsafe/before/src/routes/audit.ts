import type { FastifyRequest } from "fastify";

export async function audit(_request: FastifyRequest) {
  return db.query("SELECT * FROM audit_logs WHERE origin = $1", ["internal"]);
}
