# GitHub Branch Ruleset Configuration

This document describes the GitHub repository ruleset that protects the `main` branch.

## Why rulesets instead of branch protection rules

GitHub Rulesets (available on all plans) supersede the legacy branch protection UI. Advantages:

- Layered rules (org-level + repo-level)
- Bypass lists for specific actors
- Rule insights (per-rule audit log)

## Setting up the `protect-main` ruleset

Go to **Settings → Rules → Rulesets → New ruleset** and configure the following.

### Target

| Setting | Value |
|---|---|
| Name | `protect-main` |
| Enforcement status | Active |
| Target branches | `refs/heads/main` |

### Rules

| Rule | Setting |
|---|---|
| Restrict deletions | Enabled |
| Require linear history | Enabled |
| Require a pull request before merging | Enabled |
| — Required approvals | `1` |
| — Dismiss stale approvals when new commits are pushed | Enabled |
| — Require review from Code Owners | Disabled (no `CODEOWNERS` file yet) |
| Require status checks to pass | Enabled |
| — Required status check | `Rust checks` (job name in `ci.yaml`) |
| — Require branches to be up to date before merging | Enabled |
| Block force pushes | Enabled |

### Bypass list

Add **Repository admin** to the bypass list. This allows merging hotfixes in an emergency without being blocked by the ruleset. Keep the bypass list minimal.

## CODEOWNERS (future)

When the project has more contributors, add `.github/CODEOWNERS` to require domain-owner review:

```
# Audit rules require review from the core team
src/audits/   @MykytaStel
```

## Verifying the ruleset is active

Open a draft PR targeting `main` and confirm:

- The `Rust checks` status check is listed as required.
- Merging is blocked until the check passes and at least one approval is given.
