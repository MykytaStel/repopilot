# Rationale — boundary/access-control (unsafe)

A new file is added at `src/auth/session.ts` — a path under the access-control
boundary. Boundary detection is purely path-based (never inspects code
semantics), so any new file under `src/auth/**` must raise a
`boundary.access-control` signal in the definitely-sensitive tier, regardless
of how trivial its contents are. This is the unsafe twin of `safe/`, which
adds an equivalent-shaped file under the ordinary `src/utils/` path instead.
