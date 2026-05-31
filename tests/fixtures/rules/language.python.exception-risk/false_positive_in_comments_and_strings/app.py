# AST precision guard: the risky tokens below appear only inside comments and
# string literals, so AST detection must NOT flag them.

# Avoid a bare "except:" and production "assert" statements.
USAGE = "raise NotImplementedError once the provider is wired"


def describe():
    note = "use assert only in tests; never a bare except: in production"
    return note
