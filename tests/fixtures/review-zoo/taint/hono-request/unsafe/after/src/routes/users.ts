import { Hono } from "hono";

const app = new Hono();

app.get("/users", async (c) => {
  const id = c.req.query("id");
  return db.query("SELECT * FROM users WHERE id = " + id);
});

export default app;
