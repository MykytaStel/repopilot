# Rationale — taint/sql-injection (safe)

The same request input (`req.query.id`) that reaches a raw SQL sink in
`unsafe/` instead reaches `userRepository.findById(id)` here — no raw SQL
string, no dangerous sink at all. Taint-lite only fires when untrusted input
reaches a recognized dangerous sink (raw SQL, subprocess/exec, fs write,
outbound network call); a safe ORM call is not one, and the behavioral
detector's raw-SQL-added rule also does not apply since there is no
`db.query`-shaped call. This is a genuine near-miss twin of `unsafe/`: same
tainted input, different (safe) sink — a direct precision-boundary test, not
just an unrelated safe file.

(A parameterized `db.query("...?", [id])` variant was tried first and
rejected: the *behavioral* detector still flags any `db.query(...)` call as
`behavioral.raw-sql-added` regardless of parameterization, which would have
made that fixture non-zero-signal. The ORM-method form above is genuinely
clean across all four delta types.)
