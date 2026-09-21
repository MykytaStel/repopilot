# Rationale — boundary/access-control-with-test (safe)

The utility and its test both change under ordinary `src/utils/` and
`tests/utils/` paths. The edit has no access-control or request-trust boundary,
so the related test must not create a security or test-coverage contract delta.
This is the safe twin of the unsafe variant, which changes `src/auth/session.ts`
and its related test.
