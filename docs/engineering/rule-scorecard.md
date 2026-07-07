# RepoPilot Rule Scorecard

<!-- @generated from tests/zoo/expectations/*.toml and docs/rules-reference.md — do not edit by hand. -->
<!-- Regenerate with `python3 scripts/zoo.py scorecard --write`. -->

Per-rule signal quality derived from the real-repo validation zoo (`tests/zoo/expectations/*.toml`) and each rule's lifecycle (`docs/rules-reference.md`). Precision estimate is `(actionable + valid-but-accepted) / labeled` default-profile zoo findings — a proxy from human-reviewed dispositions, not measured precision. False-positive debt is the count of zoo findings a reviewer explicitly dispositioned `false-positive`; labels never suppress the finding, so debt reflects outstanding calibration work, not detector correctness at large.

| Rule | Lifecycle | Zoo Evidence | Precision Estimate | False-Positive Debt |
|---|---|---|---:|---:|
| `architecture.barrel-file-risk` | experimental | no zoo evidence | n/a | 0 |
| `architecture.circular-dependency` | stable | 11 labeled across 4 repo(s) | 1.00 | 0 |
| `architecture.dead-module` | experimental | no zoo evidence | n/a | 0 |
| `architecture.deep-directory-nesting` | experimental | no zoo evidence | n/a | 0 |
| `architecture.deep-relative-imports` | preview | no zoo evidence | n/a | 0 |
| `architecture.excessive-fan-out` | stable | no zoo evidence | n/a | 0 |
| `architecture.high-instability-hub` | stable | no zoo evidence | n/a | 0 |
| `architecture.large-file` | experimental | no zoo evidence | n/a | 0 |
| `architecture.layer-violation` | experimental | no zoo evidence | n/a | 0 |
| `architecture.package-boundary-violation` | experimental | no zoo evidence | n/a | 0 |
| `architecture.test-leak` | experimental | no zoo evidence | n/a | 0 |
| `architecture.too-many-modules` | experimental | no zoo evidence | n/a | 0 |
| `code-marker.fixme` | experimental | no zoo evidence | n/a | 0 |
| `code-marker.hack` | experimental | no zoo evidence | n/a | 0 |
| `code-marker.todo` | experimental | no zoo evidence | n/a | 0 |
| `code-quality.complex-file` | preview | no zoo evidence | n/a | 0 |
| `code-quality.complex-function` | preview | no zoo evidence | n/a | 0 |
| `code-quality.deep-control-flow` | preview | no zoo evidence | n/a | 0 |
| `code-quality.long-function` | preview | no zoo evidence | n/a | 0 |
| `framework.django.debug-true` | preview | no zoo evidence | n/a | 0 |
| `framework.django.missing-allowed-hosts` | preview | no zoo evidence | n/a | 0 |
| `framework.django.raw-sql-query` | preview | no zoo evidence | n/a | 0 |
| `framework.js.console-log` | experimental | no zoo evidence | n/a | 0 |
| `framework.js.var-declaration` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.architecture-mismatch` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.async-storage-from-core` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.codegen-missing` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.deprecated-api` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.direct-state-mutation` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.flatlist-missing-key` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.hermes-disabled` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.hermes-mismatch` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.inline-style` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.old-architecture` | preview | no zoo evidence | n/a | 0 |
| `framework.react-native.old-react-navigation` | preview | no zoo evidence | n/a | 0 |
| `framework.react.class-component` | preview | no zoo evidence | n/a | 0 |
| `framework.react.prop-types` | preview | no zoo evidence | n/a | 0 |
| `framework.rn-async-storage-legacy` | preview | no zoo evidence | n/a | 0 |
| `framework.rn-gesture-handler-old` | preview | no zoo evidence | n/a | 0 |
| `framework.rn-navigation-compat` | preview | no zoo evidence | n/a | 0 |
| `framework.rn-new-arch-incompatible-dep` | preview | no zoo evidence | n/a | 0 |
| `framework.rn-reanimated-compat` | preview | no zoo evidence | n/a | 0 |
| `language.go.panic-exit-risk` | preview | no zoo evidence | n/a | 0 |
| `language.javascript.runtime-exit-risk` | preview | 1 labeled across 1 repo(s) | 1.00 | 0 |
| `language.managed.fatal-exception-risk` | preview | 1 labeled across 1 repo(s) | 1.00 | 0 |
| `language.python.exception-risk` | preview | 1 labeled across 1 repo(s) | 1.00 | 0 |
| `language.rust.panic-risk` | preview | 4 labeled across 1 repo(s) | 1.00 | 0 |
| `security.env-file-committed` | stable | no zoo evidence | n/a | 0 |
| `security.private-key-candidate` | stable | no zoo evidence | n/a | 0 |
| `security.secret-candidate` | preview | 7 labeled across 1 repo(s) | 1.00 | 0 |
| `testing.missing-test-folder` | experimental | no zoo evidence | n/a | 0 |
| `testing.source-without-test` | experimental | no zoo evidence | n/a | 0 |
