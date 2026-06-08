# GitHub Branch Ruleset Configuration

This document describes the GitHub repository ruleset that protects the `main` branch.

## Repository metadata

Set these repository fields in **Settings -> General**:

| Setting | Value |
|---|---|
| Description | `Local-first CLI for reviewing Git changes, security boundaries, and blast radius before merge.` |
| Website | `https://github.com/MykytaStel/repopilot#readme` |
| Topics | `rust`, `cli`, `code-review`, `security`, `static-analysis`, `sarif`, `github-actions`, `ai-tools`, `npm` |

Recommended feature toggles:

| Setting | Value |
|---|---|
| Issues | Enabled |
| Projects | Disabled unless actively used |
| Wiki | Disabled unless actively used |
| Automatically delete head branches | Enabled |

## Repository security settings

Set these in **Settings -> Code security and analysis**:

| Setting | Value |
|---|---|
| Dependency graph | Enabled |
| Dependabot alerts | Enabled |
| Dependabot security updates | Enabled |
| Secret scanning | Enabled |
| Push protection | Enabled |
| Non-provider secret scanning patterns | Enabled if available |
| Secret validity checks | Enabled if available |

The repository also contains:

- `.github/dependabot.yml` for Cargo, npm, and GitHub Actions updates.
- `.github/workflows/codeql.yml` for CodeQL Rust and JavaScript/TypeScript scanning.

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
| — Required approvals | `0` for the current single-maintainer repository |
| — Require review conversation resolution | Enabled |
| — Require review from Code Owners | Disabled (no `CODEOWNERS` file yet) |
| Require status checks to pass | Enabled |
| — Required status check | `Rust checks` |
| — Required status check | `Security and maintenance checks` |
| — Require branches to be up to date before merging | Enabled |
| Block force pushes | Enabled |

### Bypass list

Keep the bypass list empty. Emergency changes should still use a short pull
request and the required checks.

## CODEOWNERS (future)

When the project has more contributors, add `.github/CODEOWNERS` to require domain-owner review:

```
# Audit rules require review from the core team
src/audits/   @MykytaStel
```

## Verifying the ruleset is active

Open a draft PR targeting `main` and confirm:

- all three required checks are listed;
- merging is blocked until checks pass and review conversations are resolved;
- only squash merge is available;
- merged head branches are deleted automatically.
