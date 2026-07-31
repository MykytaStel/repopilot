### Added

- **Hono request taint analysis.** The JavaScript/TypeScript frontend now recognizes Hono context request APIs including `c.req.query()`, `c.req.queries()`, `c.req.param()`, `c.req.header()`, `c.req.json()`, `c.req.parseBody()`, `c.req.text()`, `c.req.arrayBuffer()`, `c.req.raw`, and `c.req.url` as HTTP request sources. Parameterized SQL remains quiet, Hono response helpers such as `c.json()` are excluded from request-source classification, and differential safe/unsafe review-zoo fixtures pin the request-to-raw-SQL boundary.
