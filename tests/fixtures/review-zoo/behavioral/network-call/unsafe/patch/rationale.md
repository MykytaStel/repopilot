# Rationale — behavioral/network-call (unsafe)

A new file adds an outbound `fetch()` call in an ordinary (non-boundary)
service file. The behavioral detector flags added network calls within the
changed lines; since the file isn't under a boundary path, this lands in the
maybe-sensitive tier, not definitely. This is the unsafe twin of `safe/`,
which adds a pure helper with no I/O instead.
