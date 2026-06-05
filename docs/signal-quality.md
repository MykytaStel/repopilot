# Signal Quality

RepoPilot reports a `signal_quality` summary derived from the visible findings
and the finding contract. The scan pipeline stores the summary on `ScanSummary`
after enrichment, risk scoring, and contract validation so JSON, console, and
Markdown renderers do not rebuild the same summary independently.

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

When filters, local feedback, baseline views, or visibility profiles change the
visible finding set, RepoPilot refreshes the cached summary for that report path.

## Self-scan gate

RepoPilot's release smoke checks run a default self-scan and then validate the
JSON report with:

```bash
python3 scripts/check-signal-quality.py --scan-json /tmp/repopilot-self-scan.json
```

This gate fails when the self-scan has finding contract violations or
default-visible low-quality noisy findings from broad testing, marker, or
maintainability heuristics. Those signals can still exist in strict mode, but
they should not dominate the default RepoPilot report.
