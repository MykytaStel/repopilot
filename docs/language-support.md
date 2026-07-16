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
| Rust | ✓ | ✓ | ✓ | — | — | ✓ | rule-aware |
| TypeScript | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| JavaScript | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| Python | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| Go | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| Java | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | rule-aware |
| C# | ✓ | — | ✓ | — | ✓ | ✓ | rule-aware |
| Kotlin | ✓ | ✓ | ✓ | — | ✓ | ✓ | rule-aware |

Notes:

- **Rust runtime risk** is covered by the dedicated `rust.panic-risk` rule
family rather than the shared runtime-risk audit, so its column reads “—”
here; how it counts toward the capability model is tracked in the
support-honesty ledger.
- **C# review signals** exclude AST boundary classification: the pre-registry
dispatch never matched the label detection emits, and enabling it is a
deliberate behavior change, not a docs update.
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

## Adding a language

The whole point of the frontend contract is that support is added by
writing tables, not by editing engines. The checklist lives in
[Add a language](engineering/add-a-language.md).
