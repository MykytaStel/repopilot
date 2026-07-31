import { Hono } from "hono";

const app = new Hono();

app.get("/users", async (_c) => {
  const id = "system";
  return db.query("SELECT * FROM users WHERE id = " + id);
});

export default app;
