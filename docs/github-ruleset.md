# GitHub Branch Ruleset Configuration

This document describes the GitHub repository ruleset that protects the `main` branch.

## Repository metadata

Set these repository fields in **Settings -> General**:

| Setting | Value |
|---|---|
| Description | `Local-first CLI for repository audits, architecture risk detection, SARIF, CI gates, and AI-ready remediation context.` |
| Website | `https://github.com/MykytaStel/repopilot#readme` |
| Topics | `rust`, `cli`, `static-analysis`, `code-review`, `security`, `security-tools`, `architecture`, `sarif`, `github-actions`, `ai-tools`, `react-native`, `expo`, `npm` |

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
| — Required approvals | `1` |
| — Dismiss stale approvals when new commits are pushed | Enabled |
| — Require review from Code Owners | Disabled (no `CODEOWNERS` file yet) |
| Require status checks to pass | Enabled |
| — Required status check | `Rust MSRV 1.87` |
| — Required status check | `Rust checks` |
| — Required status check | `Security and maintenance checks` |
| — Required status check | `CodeQL` |
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
