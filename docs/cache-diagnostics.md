# Cache Diagnostics

Changed-file scans use a local cache under `.repopilot/cache`.

```bash
repopilot inspect cache .
repopilot inspect cache . --format json
repopilot cache clear .
```

`inspect cache` is read-only. It reports cache directory path, cache existence,
schema version, cache entry counts, stale/unmatched cache entries, and approximate
cache size.
