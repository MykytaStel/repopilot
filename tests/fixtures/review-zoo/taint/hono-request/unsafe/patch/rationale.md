The Hono route replaces a trusted identifier with `c.req.query("id")` while keeping the raw SQL sink unchanged. The changed request source must produce a deterministic `taint.sql` signal.
