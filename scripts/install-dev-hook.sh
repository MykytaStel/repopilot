#!/usr/bin/env bash
set -euo pipefail

# Explicit opt-in; do not replace another hook or alter core.hooksPath.
root=$(git rev-parse --show-toplevel)
cd "$root"

if git config --get core.hooksPath >/dev/null; then
    echo "Custom core.hooksPath is configured; add cargo fmt --all -- --check to your existing hook." >&2
    exit 1
fi

hook=$(git rev-parse --git-path hooks/pre-commit)
mkdir -p "$(dirname "$hook")"

if [[ -e "$hook" || -L "$hook" ]]; then
    echo "Existing pre-commit hook left unchanged: $hook" >&2
    exit 1
fi

# noclobber protects against a concurrent installer.
(
    set -o noclobber
    cat > "$hook" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
cargo fmt --all -- --check
HOOK
)
chmod +x "$hook"
echo "Installed formatting-only pre-commit hook: $hook"
echo "The hook may be shared by Git worktrees; no source files were modified."
