#!/usr/bin/env bash
set -euo pipefail

# Build the "innocent login fix that quietly crosses a security boundary"
# scenario into the target directory. No review, no cleanup — callers drive
# `repopilot review` and clean up. Shared by scripts/demo-boundary-review.sh
# (quick local view) and docs/demos/02-review.tape (VHS recording).
#
# Usage: scripts/demo-setup.sh <target-dir>

TARGET="${1:?usage: demo-setup.sh <target-dir>}"
mkdir -p "$TARGET"
cd "$TARGET"

git init -q
git config user.email "demo@example.invalid"
git config user.name "RepoPilot Demo"

mkdir -p src/middleware src/server
printf 'export const signup = () => {};\n' >src/signup.ts
printf 'import { authenticate } from "./middleware/auth";\nexport const routes = authenticate;\n' >src/routes.ts
printf 'import { authenticate } from "./middleware/auth";\nexport const admin = authenticate;\n' >src/admin.ts
printf 'export const authenticate = () => true;\n' >src/middleware/auth.ts
git add .
git commit -qm "app skeleton"

# The "agent edit": a small login fix that also loosens CORS and tweaks the auth
# middleware — and ships without touching a test.
printf 'export const authenticate = () => true; // tweak the post-login redirect\n' >src/middleware/auth.ts
printf 'export const corsOptions = { origin: "*" };\n' >src/server/cors.ts
