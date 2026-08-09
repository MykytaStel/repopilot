import { Hono } from "hono";

const app = new Hono();

app.get("/users", async (_c) => {
  const id = "system";
  return { id };
});

export default app;
