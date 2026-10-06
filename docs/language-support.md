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
| Kotlin | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |

Notes:

- **Rust runtime risk** reads “✓ (dedicated)”: coverage comes from the
standalone `language.rust.panic-risk` audit — structural infallibility
detection, report-renderer path awareness — rather than the shared
per-node runtime-risk table other languages use. Too contextual for that
generic shape; it still counts toward the `RuntimeRisk` capability.
- JavaScript and TypeScript (with their React dialects) share one frontend
family: the same grammar shapes, import extractor, and signal tables.

## Detected languages without a frontend

Files in these languages still contribute to repository size, scan
scope, file roles (via the shared context classifier), and generic findings;
they have no language-specific extractors.

- **Context-aware (shared classifier):** C, C++, Swift, PHP, Ruby, Dart, Scala, Shell, PowerShell, SQL, HTML, CSS, SCSS, Elixir, Erlang, Haskell, OCaml, F#, Terraform, Dockerfile, Nix.
- **Import-aware:** C/C++ Header.
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

## Direct Rust public-function removal

`review` and `scan --changed` can retain a removed top-level `pub fn` when
its current caller declares the file-backed child (`mod api;`), and imports
`api::load` / `self::api::load` (including `as` bindings), or directly calls
`api::load()` / `self::api::load()`. `crate::api::load` imports/calls require
that same declaration in `src/lib.rs` or `src/main.rs`. The ordinary resolver
must point to exactly one existing `api.rs` or `api/mod.rs` candidate.
Caller/exporter paths and exact occurrence spans survive CLI/MCP adapters and
cold/warm changed caches. Full scans lack the historical removal comparison.

This is fixture-tested coverage, not measured Rust repository precision/recall.
It does not follow arbitrary sibling callers or crate module hierarchies,
`super::` paths, grouped/glob imports, module aliases, or re-exports. Attributes
(including cfg/path), macros, inline modules and malformed syntax cause the
whole source to abstain. Foreign declarations and nested imports/modules also abstain; associated
methods and generic-function call sites are excluded. Function signatures,
fields, types and trait contracts are not compared. A preserved public name
or a coordinated current caller edit/removal does not establish a broken
contract. Retaining a private/pub(crate) definition or replacing it with a
same-name item is deliberately deferred: this slice detects definition removal,
not visibility downgrades or kind changes; module/file renames stay outside historical removal ownership.

## Adding a language

The whole point of the frontend contract is that support is added by
writing tables, not by editing engines. The checklist lives in
[Add a language](engineering/add-a-language.md).
