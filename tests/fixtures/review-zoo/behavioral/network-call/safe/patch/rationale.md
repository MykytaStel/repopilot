# Rationale — behavioral/network-call (safe)

A new file adds a pure, synchronous helper (`double`) with no network call,
subprocess exec, filesystem write, or new dependency import. The behavioral
detector only flags *added* I/O-shaped operations within the changed lines, so
a pure computation must not raise a signal. This is the safe twin of
`unsafe/`, which adds an outbound `fetch()` call to an equivalent ordinary
service file instead.
