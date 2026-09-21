# Rationale — boundary/access-control-with-test (unsafe)

The changed `src/auth/session.ts` file is an access-control boundary. Its
related `tests/auth/session.test.ts` file changes in the same diff, and the
stable `auth`/`session` path relationship is the bounded evidence for
`test-changed`. Static naming still does not prove that the test executes the
authorization behavior, so both contract deltas remain limited confidence.
