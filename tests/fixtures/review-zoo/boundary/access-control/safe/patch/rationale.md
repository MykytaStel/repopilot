# Rationale — boundary/access-control (safe)

A new file is added at `src/utils/format.ts`, an ordinary utility path. Boundary
detection is purely path-based, so a new file outside a security-boundary
directory (`src/auth/**`, request-trust, deploy-surface, supply-chain,
secret-config) must not raise a boundary signal — and since the added code has
no network/exec/fs/db call and no tainted sink, none of the other three delta
types (behavioral, algorithmic, taint-lite) should fire either. This is the
safe twin of `unsafe/`, which adds an equivalent-shaped file under `src/auth/`
instead.
