> **Instructions for AI assistant:** RepoPilot scan of `ai-context-sample`.
> 1. Fix the 2 high-severity security finding(s) with concrete code patches.
> 2. Keep changes minimal and focused — no rewrites unless necessary.


## How To Work

- Fix the highest-risk findings first (P0, then P1); leave low-signal cleanup for a separate change.
- Make the smallest safe edits that resolve a finding; do not rewrite broad architecture to move a metric.
- Work within the existing code style, module boundaries, and public APIs; preserve serialized formats and documented behavior.
- Inspect the cited files and nearby tests before editing, and explain any finding you judge to be a false positive.
- Add or update tests whenever behavior changes or a regression is plausible.
- Do not upload source code, call external services, or refresh a baseline to hide newly introduced risk.
# RepoPilot AI Context — ai-context-sample

**Risk Level:** 🟡 MODERATE
**Tech Stack:** Django 4.2.11
**Size:** 0 files · 0 non-empty lines · 2 directories · ~0 tokens
**Health:** 2 findings — 0 critical, 2 high, 0 medium

## Security (2 high)

**1. [HIGH] DEBUG = True in Django settings** — `tests/fixtures/projects/ai-context-sample/config/settings.py:1`
```
DEBUG = True
```
> **Confidence:** HIGH
> **Context:** Django's `DEBUG = True` mode exposes detailed error pages with stack traces, local variables, and settings values to anyone who triggers an exception
> **Fix:** Set DEBUG = False for deployed environments and load debug mode only from local development configuration.
> **Docs:** https://docs.djangoproject.com/en/stable/ref/settings/#debug

**2. [HIGH] ALLOWED_HOSTS is empty in Django settings** — `tests/fixtures/projects/ai-context-sample/config/settings.py:2`
```
ALLOWED_HOSTS = []
```
> **Confidence:** HIGH
> **Context:** `ALLOWED_HOSTS = []` permits any host header when `DEBUG = False`, making the application vulnerable to HTTP Host header attacks
> **Fix:** Set ALLOWED_HOSTS to the explicit domain names and IP addresses the service should accept.
> **Docs:** https://docs.djangoproject.com/en/stable/ref/settings/#allowed-hosts


## Remediation Plan

Prioritized from RepoPilot findings — start at P0 and stop when the remaining risk is acceptable for this release.

### P0 - Immediate risk

#### 1. [HIGH] DEBUG = True in Django settings
- Rule: `framework.django.debug-true`
- Area: `tests/fixtures`
- Priority: P1 (max risk 75/100)
- Examples: `tests/fixtures/projects/ai-context-sample/config/settings.py:1`
- Why: Django's `DEBUG = True` mode exposes detailed error pages with stack traces, local variables, and settings values to anyone who triggers an exception. This setting must be `False` in any environment reachable from the internet.
- Fix: Set DEBUG = False for deployed environments and load debug mode only from local development configuration.

#### 2. [HIGH] ALLOWED_HOSTS is empty in Django settings
- Rule: `framework.django.missing-allowed-hosts`
- Area: `tests/fixtures`
- Priority: P1 (max risk 75/100)
- Examples: `tests/fixtures/projects/ai-context-sample/config/settings.py:2`
- Why: `ALLOWED_HOSTS = []` permits any host header when `DEBUG = False`, making the application vulnerable to HTTP Host header attacks. Set `ALLOWED_HOSTS` to the explicit domain names and IPs this service accepts.
- Fix: Set ALLOWED_HOSTS to the explicit domain names and IP addresses the service should accept.

## Verify

- Run `repopilot scan . --min-severity high` after P0/P1 fixes.
- Run `repopilot review . --base origin/main --fail-on new-high` before merging.
- Refresh a baseline only when the remaining findings are accepted technical debt.
---
*~{{TOKENS}} tokens (budget: 4096) · scanned in 0ms — paste into Claude Code, Cursor, or ChatGPT to start fixing*
