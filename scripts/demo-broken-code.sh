#!/usr/bin/env bash
set -euo pipefail

# Build the "tidy rename that quietly breaks two callers" scenario into the
# target directory. No review, no cleanup -- callers drive `repopilot review`
# and clean up. Shared by scripts/demo-broken-code-review.sh (quick local
# view) and docs/demos/05-broken-code.tape (VHS recording).
#
# Usage: scripts/demo-broken-code.sh <target-dir>

TARGET="${1:?usage: demo-broken-code.sh <target-dir>}"
mkdir -p "$TARGET"
cd "$TARGET"

git init -q
git config user.email "demo@example.invalid"
git config user.name "RepoPilot Demo"

mkdir -p src/format
printf 'export function calculateDiscount(price: number) {\n  return price * 0.9;\n}\n' \
  >src/pricing.ts
printf 'import { calculateDiscount } from "./pricing.ts";\n\nexport const finalPrice = (price: number) => calculateDiscount(price);\n' \
  >src/checkout.ts
# shellcheck disable=SC2016 # single-quoted on purpose: this is a literal TS
# template-string source line, not a shell expansion.
printf 'export function formatCurrency(amount: number) {\n  return `$${amount.toFixed(2)}`;\n}\n' \
  >src/format/currency.ts
printf 'import { formatCurrency } from "./format/currency.ts";\n\nexport const printReceipt = (amount: number) => formatCurrency(amount);\n' \
  >src/receipt.ts
git add .
git commit -qm "app skeleton"

# The "agent edit": a tidy-looking rename plus a file move that never touches
# checkout.ts or the original currency.ts import -- both callers still expect
# the old names.
printf 'export function applyDiscount(price: number) {\n  return price * 0.9;\n}\n' \
  >src/pricing.ts
mkdir -p src/utils
git mv src/format/currency.ts src/utils/currency.ts
rmdir src/format 2>/dev/null || true
printf 'import { formatCurrency } from "./format/currency.ts";\n\n// Print a customer-facing receipt line.\nexport const printReceipt = (amount: number) => formatCurrency(amount);\n' \
  >src/receipt.ts
