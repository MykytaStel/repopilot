import { Hono } from "hono";

const app = new Hono();

app.get("/users", async (c) => {
  const id = c.req.query("id");
  return c.json({ id });
});

export default app;
