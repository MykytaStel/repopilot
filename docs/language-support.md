# Language and Framework Support

<!-- @generated from the language frontend registry — do not edit by hand. -->
<!-- Regenerate with `REPOPILOT_BLESS=1 cargo test --test language_support_doc`. -->

RepoPilot combines generic repository heuristics with language-aware
analysis owned by *language frontends* (`src/languages/`). The capability
columns below are derived from what each frontend actually wires — a column
cannot be claimed without the code behind it. `Declared` is the support
level the bundled knowledge pack asserts; where it exceeds the wired
capabilities, the gap is tracked by the registry's support-honesty guard
test rather than hidden.

## Frontend capabilities

| Language | Grammar | Imports | Review signals | Taint flows | Runtime risk | Conventions | Declared |
|---|---|---|---|---|---|---|---|
| Rust | ✓ | ✓ | ✓ | — | ✓ (dedicated) | ✓ | rule-aware |
| TypeScript | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| JavaScript | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| Python | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| Go | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| Java | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| C# | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| C | — | ✓ | — | — | — | — | context-aware |
| C++ | — | ✓ | — | — | — | — | context-aware |
| C/C++ Header | — | ✓ | — | — | — | — | import-aware |
| Kotlin | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |

Notes:

- **Rust runtime risk** reads “✓ (dedicated)”: coverage comes from the
standalone `language.rust.panic-risk` audit — structural infallibility
detection, report-renderer path awareness — rather than the shared
per-node runtime-risk table other languages use. Too contextual for that
generic shape; it still counts toward the `RuntimeRisk` capability.
- JavaScript and TypeScript (with their React dialects) share one frontend
family: the same grammar shapes, import extractor, and signal tables.

## Bounded cross-file analysis

The frontend matrix shows which language facts are wired. These additional
cross-file behaviors have narrower evidence boundaries:

- **Rust:** review detects removed public free functions, changed public
function arity, removed or privatized named public struct fields, and removed
public enum variants when a surviving caller has an unambiguous file-backed
module-qualified use. Grouped `use` imports are expanded for removed-function
contracts. Struct checks cover direct literals; enum checks cover direct
constructors and match patterns through uniquely resolved modules or a named
type import. Shadowed names, opaque module declarations, and uncertain
replacement items are excluded. Findings include both declaration and caller
locations.
- **JavaScript, TypeScript, Python, and Go:** review can follow one direct
same-file helper call from a recognized taint source to a supported sink.
Simple identity returns are tracked when the helper has one unambiguous
return. Python keyword arguments bind by name, and Go grouped parameters keep
their argument positions. Unknown transformations preserve incoming taint.
Recursive calls, dynamic targets, unsupported control flow, and chains of
multiple summarized helpers are outside this analysis.
Recognized literal non-shell argv calls retain separate arguments unless shell
mode is enabled. Explicit shell command flags, including combined or separated
flags, remain a sink.
- **Python and Go test helpers:** when a changed local assertion helper loses
supported checks, review can attribute the loss to an unchanged importing
Python test or same-package Go test. Python attribution requires a uniquely
resolved module-scope import and an unshadowed direct call. Moved or renamed
tests are treated as changed. Signals name the called helper and its line.
- **C and C++:** direct quoted `#include` directives create static local graph
edges when one target resolves relative to the including file or repository
root. Conditional branches, system includes, macro operands, and compiler
search paths are outside this analysis. The edges describe static includes.
- **JavaScript and TypeScript:** local missing-import checks cover supported
extensionless relative imports and uniquely resolved configured aliases.
Package exports, dynamic imports, and ambiguous resolver candidates remain
uncertain.
- **Python:** missing-import checks include bounded repository-rooted absolute
module candidates and package initializers. Namespace directories count as
safe local package candidates; environment-provided and installed modules
cannot be confirmed from repository files alone.
- **Go:** local workspace and local `replace` module targets are considered
without fetching dependencies. Remote replacements, build-tag variants, and
generated packages are not treated as resolved repository sources.
- **C#:** SQL taint analysis follows simple local command receiver aliases and
merges supported straight-line assignments across simple `if` branches.
Unmodeled control flow, ambiguous aliases, loops, and nested scopes cause
conservative abstention.
- **Java and Kotlin:** same-package type references add local dependency edges
for impact analysis when a source type resolves uniquely. The edge does not
establish runtime reachability.

These checks are bounded static analysis, not compiler or runtime proofs. Use
the project's own build, type-check, and tests to verify a change.

## Detected languages without a frontend

Files in these languages still contribute to repository size, scan
scope, file roles (via the shared context classifier), and generic findings;
they have no language-specific extractors.

- **Context-aware (shared classifier):** Swift, PHP, Ruby, Dart, Scala, Shell, PowerShell, SQL, HTML, CSS, SCSS, Elixir, Erlang, Haskell, OCaml, F#, Terraform, Dockerfile, Nix.
- **Detect-only:** R, Julia, Lua, Perl, Zig, Solidity, Objective-C, YAML, TOML, JSON, Markdown.

## Rule philosophy

RepoPilot should not treat every paradigm as a smell. Functional,
object-oriented, procedural, and declarative code can all be valid; rules
use context and confidence instead of assuming one style is always better:

```text
evidence + context + severity + confidence + recommendation
```

## Limitations

RepoPilot is not a compiler or type checker. Some rules are text-based or
heuristic; findings are review signals, not absolute truth. Use
language-specific tools alongside it: `cargo clippy`, `tsc`/ESLint, Ruff and
Pyright, `go vet`, or your build's own checks.

## Adding a language

The whole point of the frontend contract is that support is added by
writing tables, not by editing engines. The checklist lives in
[Add a language](engineering/add-a-language.md).
