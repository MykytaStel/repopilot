# RepoPilot Product Design

Status: **target product direction** (guides post-0.28 stabilization, bug fixes,
and future feature selection; does not claim unshipped capabilities).

## Positioning

RepoPilot is a **deterministic, local-first change-review and agent-integrity
gate** for Git diffs. It runs offline in a single native binary with zero LLM
calls, zero token cost, and zero code uploads.

Its primary job is to answer one question before merge or at the end of a
coding-agent session:

> **Did this change quietly weaken checks, break local callers, or open a
> security boundary — and is the green status honest?**

## What RepoPilot is not

- **Not a cloud LLM code reviewer.** Tools like CodeRabbit, Qodo, or Greptile
  spend cloud tokens on prose comments. RepoPilot stays fast, local,
  reproducible, and evidence-backed.
- **Not a generic style or framework linter.** ESLint, Ruff, Clippy, and
  Semgrep already own general style and broad SAST catalogs. RepoPilot does not
  compete by adding dozens of unmeasured lint rules.

## Four core pillars

Every new feature, bug fix, or UX change should strengthen one of these four
pillars:

### 1. Check and test integrity ("Honest Green")

Catch changes that make a suite look green without preserving verification:

- Removed or weakened assertions and matchers (including across local test
  helpers).
- Newly skipped, disabled, or trivialized tests.
- Quiet error-swallowing fallbacks introduced in changed code (`catch {}`,
  `except: pass`, etc.).
- Explicit separation between `PASS` (verified) and `PASS (not verified)`.

### 2. Local API contracts and blast radius

Catch changes that break surviving local consumers even when isolated unit
tests still pass:

- Removed or incompatible local exports and public symbols (JS/TS exports, Rust
  public functions, arity, struct fields, and enum variants).
- Deterministic import and local include impact paths showing which files in
  the repository are directly or transitively affected.
- Explicit abstention when resolution is ambiguous or dynamic.

### 3. Security boundaries and bounded taint

Catch high-signal security regressions introduced in the diff:

- Removed or bypassed authorization and guard checks.
- Bounded data-flow paths from untrusted inputs to dangerous SQL, shell, or
  filesystem sinks (including one-hop local helpers where proven).
- Committed secrets and private-key candidates.

### 4. Agent-first review loop (Hooks + MCP)

Make RepoPilot effortless for both humans and coding agents (Claude Code,
Codex, Cursor, Gemini CLI):

- **Verdict-first first screen:** one clear decision, concrete top evidence
  with file/line proofs, and one next action. Deeper detail stays behind
  `--explain` / `--explain-signal` or MCP drill-down tools.
- **Zero-friction setup:** `repopilot init` wires stop hooks, MCP, and
  verification checks cleanly.
- **Safe verification:** configured local commands run only when explicitly
  requested (CLI) or approved per check (MCP).

## Surface discipline and noise reduction

1. **`repopilot review` is the hero workflow.** Full-repository `scan`,
   snapshots, and broad repository-health rules are secondary surfaces and must
   not dilute `review` clarity or performance.
2. **Evidence before default visibility.** A rule or signal should not become
   default-visible without motivating real-code cases, safe controls, and
   tracked precision/false-positive evidence in the scorecard or corpus.
3. **Prune or demote unmeasured noise.** Legacy/experimental lint rules with no
   measured evidence should move behind explicit opt-in profiles or be retired
   rather than adding maintenance drag and false alarms.
4. **Honest boundaries.** Every detector names what it cannot prove (dynamic
   dispatch, reflection, macros, external crates/packages) instead of guessing.

## Stabilization and evolution order

Rather than rushing to `1.0`, work proceeds in calm, disciplined slices:

1. **Fix chaos and known bugs first:** stabilize the `main` branch, fix real
   user-reported bugs and confusing UX, and keep release automation (including
   Homebrew tap publishing) green.
2. **Real-world dogfooding and signal calibration:** run RepoPilot on real
   repositories and live agent sessions (Claude Code, Codex, Cursor, Gemini
   CLI), turn real friction into regression tests, and expand measured rule
   coverage in [rule-scorecard.md](engineering/rule-scorecard.md).
3. **Internal maintainability:** gradually split oversized production modules
   (>500 lines) back toward the repository's 300-line convention so bug fixes
   stay fast and safe.
4. **Focused capability growth:** add new detectors only when they directly
   strengthen one of the four pillars above and are backed by real failure
   cases.
