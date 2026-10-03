#!/usr/bin/env bash
# Mirror integrations/claude-code/repopilot to MykytaStel/repopilot-plugin.
#
# The Claude plugin directory applies stricter command rules to a plugin that
# sits in a subfolder of its repository, so the directory lists the plugin from
# a mirror where it is the repository root. `git subtree split` is
# deterministic: the same history gives the same commits, so each sync is a
# fast-forward of the mirror's main branch. Never commit to the mirror directly.
#
# Usage: scripts/sync-plugin-repo.sh [ref]   (default: origin/main)
set -euo pipefail

ref="${1:-origin/main}"
prefix="integrations/claude-code/repopilot"
mirror="https://github.com/MykytaStel/repopilot-plugin.git"
branch="plugin-mirror-sync"

git fetch --quiet origin
git branch -D "$branch" >/dev/null 2>&1 || true
git subtree split --prefix="$prefix" "$ref" -b "$branch" >/dev/null
git push "$mirror" "$branch:main"
echo "Mirrored $prefix at $(git rev-parse --short "$ref") to $mirror ($(git rev-parse --short "$branch"))."
git branch -D "$branch" >/dev/null
