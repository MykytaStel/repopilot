# RepoPilot Rule Scorecard

<!-- @generated from tests/zoo/expectations/*.toml and docs/rules-reference.md — do not edit by hand. -->
<!-- Regenerate with `python3 scripts/zoo.py scorecard --write`. -->

Per-rule signal quality derived from the real-repo validation zoo (`tests/zoo/expectations/*.toml`) and each rule's lifecycle (`docs/rules-reference.md`). Validity estimate is `(actionable + valid-but-accepted) / labeled` default-profile zoo findings — a proxy from human-reviewed dispositions, not measured production precision. The validity column includes a 95% Wilson interval for the labeled sample; actionability and false-positive rate remain separate descriptive proportions. False-positive debt is the count of zoo findings a reviewer explicitly dispositioned `false-positive`; labels never suppress the finding, so debt reflects outstanding calibration work, not detector correctness at large.

## Default-profile evidence

Every default-visible zoo finding is labeled, so these rows are exhaustive for the pinned repositories. `no zoo evidence` means the rule never fired in the default profile on any of them — the rule is unmeasured, which is not the same as clean.

| Rule | Lifecycle | Zoo Evidence | Evidence Status | Validity (95% Wilson) | Actionability | False-Positive Rate | False-Positive Debt |
|---|---|---|---|---:|---:|---:|---:|
| `architecture.barrel-file-risk` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.circular-dependency` | stable | 11 labeled across 4 repo(s) | descriptive | 1.00 (0.74–1.00) | 0.45 | 0.00 | 0 |
| `architecture.dead-module` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.deep-directory-nesting` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.deep-relative-imports` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.excessive-fan-out` | stable | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.high-instability-hub` | stable | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.large-file` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.layer-violation` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.package-boundary-violation` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.test-leak` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.too-many-modules` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `architecture.unresolved-local-import` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `behavioral.removed-export-still-imported` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-marker.fixme` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-marker.hack` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-marker.todo` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-quality.complex-file` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-quality.complex-function` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-quality.deep-control-flow` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `code-quality.long-function` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.django.debug-true` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.django.missing-allowed-hosts` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.django.raw-sql-query` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.js.console-log` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.js.var-declaration` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.architecture-mismatch` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.async-storage-from-core` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.codegen-missing` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.deprecated-api` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.direct-state-mutation` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.flatlist-missing-key` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.hermes-disabled` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.hermes-mismatch` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.inline-style` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.old-architecture` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react-native.old-react-navigation` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react.class-component` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.react.prop-types` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.rn-async-storage-legacy` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.rn-gesture-handler-old` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.rn-navigation-compat` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.rn-new-arch-incompatible-dep` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `framework.rn-reanimated-compat` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `language.go.panic-exit-risk` | preview | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `language.javascript.runtime-exit-risk` | preview | 1 labeled across 1 repo(s) | insufficient evidence | 1.00 (0.21–1.00) | 1.00 | 0.00 | 0 |
| `language.managed.fatal-exception-risk` | preview | 1 labeled across 1 repo(s) | insufficient evidence | 1.00 (0.21–1.00) | 1.00 | 0.00 | 0 |
| `language.python.exception-risk` | preview | 1 labeled across 1 repo(s) | insufficient evidence | 1.00 (0.21–1.00) | 0.00 | 0.00 | 0 |
| `language.rust.panic-risk` | preview | 4 labeled across 1 repo(s) | insufficient evidence | 1.00 (0.51–1.00) | 0.50 | 0.00 | 0 |
| `security.env-file-committed` | stable | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `security.private-key-candidate` | stable | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `security.secret-candidate` | preview | 7 labeled across 1 repo(s) | insufficient evidence | 1.00 (0.65–1.00) | 0.00 | 0.00 | 0 |
| `testing.missing-test-folder` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |
| `testing.source-without-test` | experimental | no zoo evidence | unmeasured | n/a | n/a | n/a | 0 |

## Evidence coverage

- Default-profile evidence: 6 of 54 rules (11.1%), 25 labeled findings across 7 repo(s).
- Default-profile rules without evidence: 48 (unmeasured, not clean).
- Strict-profile sampled evidence: 6 rules, 14 sampled findings across 5 repo(s).
- Evidence status is `insufficient evidence` below 10 labeled findings; `descriptive` is a sample-size label, not a production precision claim.
- These coverage counts describe committed labels and do not establish recall.


## Strict-profile sampled evidence

Rules that fire only in the strict profile are too numerous to label exhaustively. These rows come from deterministic per-rule samples (`python3 scripts/zoo.py sample --rule <id>`), so the validity estimate describes the sampled findings, not the rule's full strict-profile population. A rule missing from this table has no sampled evidence at all.

| Rule | Lifecycle | Sampled | Evidence Status | Validity (95% Wilson) | Actionability | False-Positive Rate | False-Positive Debt |
|---|---|---|---|---:|---:|---:|---:|
| `architecture.dead-module` | experimental | 3 sampled across 2 repo(s) | insufficient evidence | 0.00 (0.00–0.56) | 0.00 | 1.00 | 3 |
| `architecture.excessive-fan-out` | stable | 1 sampled across 1 repo(s) | insufficient evidence | 1.00 (0.21–1.00) | 0.00 | 0.00 | 0 |
| `architecture.large-file` | experimental | 3 sampled across 3 repo(s) | insufficient evidence | 1.00 (0.44–1.00) | 0.00 | 0.00 | 0 |
| `code-quality.complex-function` | preview | 3 sampled across 3 repo(s) | insufficient evidence | 1.00 (0.44–1.00) | 1.00 | 0.00 | 0 |
| `code-quality.long-function` | preview | 3 sampled across 3 repo(s) | insufficient evidence | 1.00 (0.44–1.00) | 0.33 | 0.00 | 0 |
| `security.env-file-committed` | stable | 1 sampled across 1 repo(s) | insufficient evidence | 1.00 (0.21–1.00) | 0.00 | 0.00 | 0 |
