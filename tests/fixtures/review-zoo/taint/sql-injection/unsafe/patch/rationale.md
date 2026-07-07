# Rationale — taint/sql-injection (unsafe)

Request input (`req.query.id`) is concatenated directly into a raw SQL string
passed to `db.query`, reaching a dangerous sink with no sanitization in
between. Taint-lite traces untrusted-source-to-dangerous-sink flow within the
changed function and lands this in the definitely-sensitive tier. This is the
unsafe twin of `safe/`, where the same tainted input reaches a safe ORM call
instead.
