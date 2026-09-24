# Verification provenance v1

RepoPilot treats verification as external execution evidence. A check result
can support a review obligation only for the selected check, the captured
workspace revision, and the configured policy. This document defines the
additional diagnostic identity that can be used by the local differential
protocol.

## Canonical record

`VerificationOutcome.diagnostics` is optional and additive. When present it
contains:

- `adapter`: the versioned parser contract, currently `pytest-node-v1`;
- `complete`: whether the adapter saw enough bounded, redacted output to make
  an identity claim;
- `entries`: stable diagnostic keys and their kind;
- `limitation`: a bounded reason when `complete` is false.

The current adapter is selected only for an explicitly configured `python.tests`
check whose command is recognizably `python`/`python3 -m pytest` (or equivalent
pytest argument). It emits keys such as:

```text
python.tests:tests/test_api.py::test_create[param]:failed
python.tests:tests/conftest.py:collection-error
```

Absolute paths are retained only when they can be safely normalized inside the
verification working directory. Paths outside that directory, parent escapes,
unknown formats, and empty failed output are unavailable rather than guessed.

## Completeness rules

An exact comparison requires all of the following:

1. exactly one selected `python.tests` outcome;
2. role `test` and a revision-compatible before/after workspace;
3. both streams complete and bounded;
4. `diagnostics.adapter == "pytest-node-v1"` and `complete == true`;
5. every diagnostic key passes the `python.tests:` namespace check.

A passed check has a complete empty diagnostic set. A failed check without a
supported node remains unavailable. Timeouts, cancellations, skipped checks,
unavailable programs, and unsupported adapters carry a limitation and cannot
be used as exact-node evidence.

## Projection and compatibility

The same outcome is serialized in review JSON and MCP, embedded in review
SARIF run properties, and summarized in console, Markdown, and HTML. The
cache stores the already redacted outcome, so provenance is reused only with
the same trusted cache key. Reports written before this field remain readable;
the differential helper keeps a legacy output parser for those reports.

This record identifies observed verification output. It does not prove that a
test is complete, that the underlying system is correct, or that a human will
find the report useful.
