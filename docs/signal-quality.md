# Signal Quality

RepoPilot 0.13.0 adds a report-level `signal_quality` summary derived from the
visible findings and the finding contract.

JSON reports include:

- total findings;
- counts by confidence;
- counts by rule lifecycle;
- counts by signal source;
- evidence coverage percentage;
- recommendation coverage percentage;
- docs coverage for high/critical findings;
- contract violation count.

Console and Markdown reports show a compact summary:

```text
Signal quality:
- High confidence: 12
- Medium confidence: 8
- Low confidence: 2
- Evidence coverage: 100%
- Recommendation coverage: 100%
- Contract warnings: 0
```

The summary is meant to help users decide how much trust to place in the current
report. A report with many experimental or low-confidence findings should be
treated as review guidance, while stable high-confidence findings should receive
faster attention.
