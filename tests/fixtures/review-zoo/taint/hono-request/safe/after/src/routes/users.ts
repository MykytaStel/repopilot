import { Hono } from "hono";

const app = new Hono();

app.get("/users/:id", async (c) => {
  const id = c.req.param("id");
  return c.json({ id });
});

export default app;
