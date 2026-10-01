#!/usr/bin/env bash
set -euo pipefail

# Build the "weakened tests" scenario in a fresh Git repository: a pricing
# change that drops a range check, skips the test that would now fail, removes
# an assertion from another, and lets the CI test step fail without failing the
# job. Every check still passes; `repopilot review .` reports each weakening.
# Used by the README example and docs/demos/06-weakened-tests.tape.
#
# Usage: demo-weakened-tests.sh <empty-directory>

TARGET="${1:?usage: demo-weakened-tests.sh <empty-directory>}"
mkdir -p "$TARGET/src" "$TARGET/.github/workflows"
cd "$TARGET"

cat > src/pricing.ts <<'EOF'
export function applyDiscount(total: number, percent: number): number {
  if (percent < 0 || percent > 100) throw new RangeError("percent out of range");
  return Math.round(total * (100 - percent)) / 100;
}
EOF
cat > src/pricing.test.ts <<'EOF'
import { describe, expect, it } from "vitest";
import { applyDiscount } from "./pricing";

describe("applyDiscount", () => {
  it("takes a percentage off the total", () => {
    expect(applyDiscount(200, 10)).toBe(180);
    expect(applyDiscount(99.99, 0)).toBe(99.99);
  });

  it("rejects a discount above 100%", () => {
    expect(() => applyDiscount(50, 150)).toThrow(RangeError);
  });
});
EOF
cat > .github/workflows/ci.yml <<'EOF'
name: ci
on: [pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: npm test
EOF

git init -q
git add .
git -c user.email=demo@example.invalid -c user.name=Demo commit -qm "pricing with tests"

# The change under review.
cat > src/pricing.ts <<'EOF'
export function applyDiscount(total: number, percent: number): number {
  return Math.round(total * (100 - percent)) / 100;
}
EOF
sed -i.bak \
  -e 's/  it("rejects a discount above 100%"/  it.skip("rejects a discount above 100%"/' \
  -e '/applyDiscount(99.99, 0)/d' \
  src/pricing.test.ts
sed -i.bak -e 's/^      - run: npm test$/      - run: npm test\n        continue-on-error: true/' .github/workflows/ci.yml
rm -f src/pricing.test.ts.bak .github/workflows/ci.yml.bak
