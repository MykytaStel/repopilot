# Feature Specification: RepoPilot 0.21 Risk Memory and Merge Readiness

**Feature Branch**: `001-021-risk-memory-merge-readiness`

**Created**: 2026-07-26

**Status**: Approved

**Input**: Make RepoPilot 0.21.0 a breakthrough local-first release that shows
whether a change made repository risk better or worse, what it affects, who
should review it, and what must be verified before merge. Harden overlays and
deepen the existing language frontends as part of the same trust contract.

## Product Promise

RepoPilot 0.21 does not stop at finding problems. It explains whether the
current change improved or regressed repository risk, identifies the affected
surface and responsible owners, and emits one deterministic merge-readiness
decision across CLI, reports, GitHub Action, MCP, and AI context.

## User Scenarios & Testing

### User Story 1 - See Risk Regression Across Runs (Priority: P1)

A maintainer or coding agent records compatible local analyses and immediately
sees which finding occurrences are new, persisting, resolved, or changed in
severity.

**Why this priority**: This turns isolated reports into evidence of progress and
prevents an apparently smaller changed scan from being mistaken for a safer
repository.

**Independent Test**: Record two full scans of the same workspace contract,
introduce and resolve findings between them, and verify the classified delta.

**Acceptance Scenarios**:

1. **Given** two full scans with the same target, profile, configuration, and
   overlay contract, **When** the second scan is recorded, **Then** RepoPilot
   reports new, persisting, resolved, and severity-shift occurrences.
2. **Given** a previous full scan and a current changed-only scan, **When**
   RepoPilot looks for a comparison, **Then** it does not classify out-of-scope
   findings as resolved and explains that no compatible prior run exists.
3. **Given** two findings with the same baseline ID but different evidence,
   **When** history computes the delta, **Then** both occurrence identities are
   retained independently.
4. **Given** history is not enabled, **When** a scan runs, **Then** no history
   directory or ledger is created.

---

### User Story 2 - Know Whether a Change Is Ready to Merge (Priority: P1)

A maintainer reviewing a change gets a single `ready`, `review`, or `blocked`
verdict with exact reasons, affected surfaces, owners, and verification steps.

**Why this priority**: It converts analysis into an actionable merge decision
instead of leaving users to assemble findings, blast radius, and ownership by
hand.

**Independent Test**: Review a fixture repository with CODEOWNERS, a dependency
chain, and changed-code findings, then assert the deterministic readiness
record and projections.

**Acceptance Scenarios**:

1. **Given** a change with no blocking findings and bounded impact, **When**
   review completes, **Then** the verdict is `ready` and still lists relevant
   verification steps.
2. **Given** a change with review-worthy signals or unowned impacted files,
   **When** review completes, **Then** the verdict is `review` with explicit
   reasons.
3. **Given** a change with a blocking decision under the active profile,
   **When** review completes, **Then** the verdict is `blocked`.
4. **Given** matching CODEOWNERS rules, **When** changed and transitively
   impacted files are evaluated, **Then** the readiness record lists
   deterministic, deduplicated owner suggestions.
5. **Given** no CODEOWNERS file, **When** review completes, **Then** RepoPilot
   reports directory or package ownership boundaries without inventing people.

---

### User Story 3 - Trust Repository Calibration and Cache Freshness (Priority: P1)

A repository can commit `.repopilot/overlay.toml`, and every analysis surface
observes overlay changes immediately.

**Why this priority**: A stale cached decision after a policy edit invalidates
the product's trust promise.

**Independent Test**: Run a cached MCP scan, edit only the overlay, rerun it,
and verify that the result and workspace revision change.

**Acceptance Scenarios**:

1. **Given** a tracked overlay file, **When** its severity or suppression rule
   changes, **Then** the MCP cache key changes and the next scan recomputes.
2. **Given** generated cache and history state changes, **When** a workspace
   fingerprint is computed, **Then** those changes do not invalidate analysis.
3. **Given** the repository's ignore rules, **When** a user creates an overlay,
   **Then** Git can track it while generated history remains ignored.

---

### User Story 4 - Get Deeper Results in Existing Languages (Priority: P2)

Users of the eight rule-aware frontends receive framework-aware review and
security coverage backed by safe/unsafe differential fixtures.

**Why this priority**: Depth and precision in languages already claimed as
rule-aware provide more trust than adding a shallow ninth language.

**Independent Test**: For each added framework contract, run its paired safe and
unsafe fixture and verify that only the unsafe flow emits the expected signal.

**Acceptance Scenarios**:

1. **Given** Java, Kotlin, or C# web input reaching a high-specificity sink,
   **When** review runs, **Then** the matching Spring, Ktor, or ASP.NET flow is
   detected.
2. **Given** TypeScript/JavaScript, Python, or Go framework input reaching a
   supported sink, **When** review runs, **Then** the unsafe fixture emits a
   signal and the safe counterpart does not.
3. **Given** Rust Axum, Actix, or Rocket boundary changes, **When** review runs,
   **Then** boundary evidence is reported without claiming generic Rust taint.
4. **Given** a new capability claim, **When** the generated support matrix is
   checked, **Then** the claim is derived from real frontend wiring.

---

### User Story 5 - Consume One Contract Everywhere (Priority: P2)

CLI, JSON, Markdown, GitHub Action, MCP, and AI context consumers receive the
same risk-delta and merge-readiness meaning.

**Why this priority**: Divergent surface-specific logic makes agent and human
decisions inconsistent.

**Independent Test**: Analyze one fixture through each applicable projection
and compare the canonical record fields and counts.

**Acceptance Scenarios**:

1. **Given** one canonical readiness record, **When** it is rendered through
   human and machine outputs, **Then** verdict, reasons, counts, owners, and
   verification steps agree.
2. **Given** an older JSON consumer, **When** it reads the additive 0.21
   fields, **Then** existing fields and exit-code behavior remain compatible.
3. **Given** history is unavailable or disabled, **When** a report is emitted,
   **Then** readiness still works and history availability is explicit.

### Edge Cases

- A scan targets a single file, a subdirectory, or a workspace different from
  the current directory.
- A changed scan has no merge base or uses a different base/head pair.
- The active profile, relevant configuration, overlay, or report schema changes
  between runs.
- The ledger contains an interrupted final line, an older supported schema, or
  an unsupported future schema.
- Two processes attempt to record history concurrently.
- Retention is configured to zero, one, or a value that exceeds the byte cap.
- CODEOWNERS contains escaped spaces, comments, overlapping patterns, negation-
  like text unsupported by GitHub semantics, or owners repeated across rules.
- Changed files have no owner, are deleted, or are outside the dependency graph.
- Sensitive snippets appear in readiness reasons or verification steps.
- A language framework pattern is syntactically similar to a sink but receives
  a constant, sanitized, or otherwise untainted value.

## Requirements

### Functional Requirements

- **FR-001**: History MUST be opt-in and MUST NOT create files when disabled.
- **FR-002**: Every run receipt MUST include a schema version, workspace
  identity, analysis target, scope, revisions, profile, configuration
  fingerprint, overlay fingerprint, and finding occurrence records.
- **FR-003**: RepoPilot MUST compare only receipts with compatible comparison
  identities and MUST explain an unavailable comparison.
- **FR-004**: Delta matching MUST use occurrence identity, not the baseline
  finding ID alone.
- **FR-005**: History storage MUST use the workspace root, remain bounded by run
  count and bytes, and replace pruned data atomically.
- **FR-006**: A damaged ledger MUST produce a structured diagnostic without
  failing the underlying scan.
- **FR-007**: Wall-clock metadata MUST NOT affect analysis determinism,
  occurrence identity, report equality, or cache keys.
- **FR-008**: RepoPilot MUST derive one canonical merge-readiness record with a
  `ready`, `review`, or `blocked` verdict and ordered reason codes.
- **FR-009**: Readiness MUST include changed/impacted surfaces, matched owners,
  unowned surfaces, verification steps, and analysis limitations.
- **FR-010**: Ownership MUST support GitHub-compatible CODEOWNERS precedence and
  deterministic fallback package/directory boundaries without Git-history
  inference.
- **FR-011**: Human-readable readiness evidence MUST use centralized sensitive
  evidence redaction.
- **FR-012**: Risk delta and readiness MUST be projected from shared records
  across CLI, JSON, Markdown, Action, MCP, and AI context where those surfaces
  expose review results.
- **FR-013**: `.repopilot/overlay.toml` MUST be trackable and MUST participate in
  workspace and MCP scan-cache invalidation.
- **FR-014**: Generated `.repopilot/cache/` and `.repopilot/history/` changes
  MUST NOT invalidate analysis fingerprints.
- **FR-015**: Existing report contracts MUST evolve additively and accepted
  older schema versions MUST continue to parse.
- **FR-016**: Existing top-level CLI commands MUST remain unchanged.
- **FR-017**: New Java, Kotlin, C#, TypeScript/JavaScript, Python, Go, and Rust
  language behavior MUST have safe/unsafe regression fixtures appropriate to
  the claimed framework behavior.
- **FR-018**: New language signals MUST preserve default-profile precision and
  strict-mode recall according to the repository's false-positive policy.
- **FR-019**: Support documentation MUST be generated from the frontend registry
  and MUST NOT advertise unwired capabilities.
- **FR-020**: The v0.20 release roadmap, analysis platform state, language
  inventory, 0.21 roadmap, changelog, and release notes MUST agree with shipped
  and in-progress behavior.

### Key Entities

- **RunReceiptV1**: A bounded local record of one analysis contract and its
  occurrence-level findings.
- **ComparisonIdentity**: The fields that determine whether two runs can be
  compared without producing false resolutions.
- **RiskDelta**: Ordered new, persisting, resolved, and severity-shift
  occurrences plus comparison provenance.
- **MergeReadinessRecord**: Canonical verdict, reasons, impact, ownership,
  verification, limitations, and optional risk delta.
- **OwnershipIndex**: Parsed CODEOWNERS rules and deterministic fallback
  boundaries.
- **LanguageQualityCase**: A framework-specific safe/unsafe fixture pair and
  expected review behavior.

## Success Criteria

### Measurable Outcomes

- **SC-001**: Full-vs-changed and incompatible-profile regression tests produce
  zero false `resolved` classifications.
- **SC-002**: Two colliding baseline IDs with different occurrences remain two
  independent history entries and delta results.
- **SC-003**: Editing only `overlay.toml` invalidates the cached MCP scan in an
  integration test; editing generated history does not.
- **SC-004**: One readiness fixture produces identical verdict, reason codes,
  owner set, and verification count across every supported projection.
- **SC-005**: Every new language behavior has at least one unsafe true-positive
  and one near-identical safe false-positive guard.
- **SC-006**: All default-visible zoo findings remain reviewed and labeled; no
  measured zoo claim is made when the zoo is unavailable.
- **SC-007**: History retention never exceeds the configured positive run count
  or byte cap after a successful write.
- **SC-008**: The full RepoPilot release gate passes before 0.21 handoff.

## Assumptions

- Existing `FindingRecord`, `DecisionRecord`, occurrence keys, impact paths,
  workspace revisions, and centralized redaction remain the canonical
  foundations.
- GitHub-compatible CODEOWNERS files are discovered in `.github/`, repository
  root, then `docs/`, matching GitHub precedence.
- History timestamps are useful local metadata but are excluded from
  deterministic projections unless a user explicitly inspects the ledger.
- Runtime execution, automatic test execution, hosted sync, telemetry, implicit
  LLM calls, and a new language frontend are outside 0.21.
- No commits, pushes, tags, or publication occur without explicit user
  authorization.
