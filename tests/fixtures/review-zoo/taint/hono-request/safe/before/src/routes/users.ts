import { Hono } from "hono";

const app = new Hono();

function findUser(id: string | undefined) {
  return db.query("SELECT * FROM users WHERE id = $1", [id]);
}

app.get("/users", async (_c) => {
  const id = "system";
  return findUser(id);
});

export default app;
