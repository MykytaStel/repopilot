#!/usr/bin/env bash
# Verify that every public release channel serves the exact tagged artifacts:
# GitHub Release archives and checksums, the crates.io package checksum, npm
# package integrity (root and platform packages), and Homebrew formula digests.
#
# Used by the tag workflow, by the manual `Verify publication` workflow after a
# recovery, and locally:
#
#   VERSION=v0.23.0 SOURCE_DIR=/path/to/tag/checkout scripts/verify-publication.sh
#
# SOURCE_DIR must be a checkout of the release tag (default: current directory);
# expected crate and npm digests are rebuilt from it. Exits 0 only when every
# channel matches, 1 on a mismatch or when channels do not converge in time.
set -euo pipefail

TOOLS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${VERSION:?VERSION must be the release tag, e.g. v0.23.0}"
SOURCE_DIR="$(cd "${SOURCE_DIR:-.}" && pwd)"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:-MykytaStel/repopilot}"
# crates.io rejects API requests without a descriptive User-Agent (HTTP 403).
CRATES_USER_AGENT="${CRATES_USER_AGENT:-repopilot-release (https://github.com/MykytaStel/repopilot)}"
VERIFY_ATTEMPTS="${VERIFY_ATTEMPTS:-20}"
VERIFY_SLEEP_SECONDS="${VERIFY_SLEEP_SECONDS:-15}"
VERSION_NUMBER="${VERSION#v}"

verify_tmp="$(mktemp -d)"
trap 'rm -rf "$verify_tmp"' EXIT

gh release download "$VERSION" \
  --repo "$GITHUB_REPOSITORY" \
  --dir "$verify_tmp" \
  --pattern "repopilot-${VERSION}-*" \
  --clobber
for target in \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  aarch64-apple-darwin \
  x86_64-pc-windows-msvc; do
  if [[ "$target" == x86_64-pc-windows-msvc ]]; then
    archive="repopilot-${VERSION}-${target}.zip"
  else
    archive="repopilot-${VERSION}-${target}.tar.gz"
  fi
  test -f "$verify_tmp/$archive"
  test -f "$verify_tmp/$archive.sha256"
  (cd "$verify_tmp" && tr -d '\r' < "$archive.sha256" | sha256sum -c -)
done

(cd "$SOURCE_DIR" && cargo package --allow-dirty --no-verify >/dev/null)
crate_checksum="$(sha256sum "$SOURCE_DIR/target/package/repopilot-${VERSION_NUMBER}.crate" | awk '{print $1}')"
node "$TOOLS_DIR/scripts/build-npm-platform-packages.js" \
  --dist "$verify_tmp" \
  --out "$verify_tmp/npm-platform-packages" \
  --version "$VERSION_NUMBER" \
  > "$verify_tmp/npm-platform-packages.json"
root_integrity="$(cd "$SOURCE_DIR" && npm pack --dry-run --json | jq -r '.[0].integrity')"

retry_needed=false
record_state() {
  local result state
  result="$(python3 "$TOOLS_DIR/scripts/publication_state.py" classify "$@")"
  echo "$result"
  state="$(jq -r '.state' <<<"$result")"
  case "$state" in
    published-matching) ;;
    published-mismatch)
      echo "::error::published-mismatch; refusing to report a release as converged"
      exit 1
      ;;
    *) retry_needed=true ;;
  esac
}

check_npm() {
  local package="$1" expected_digest="$2" metadata error_file
  error_file="$(mktemp)"
  if metadata="$(npm view "${package}@${VERSION_NUMBER}" version dist.integrity --json 2>"$error_file")"; then
    record_state \
      --channel "$package" \
      --expected-version "$VERSION_NUMBER" \
      --expected-digest "$expected_digest" \
      --observed-version "$(jq -r 'if type == "string" then . else .version end' <<<"$metadata")" \
      --observed-digest "$(jq -r 'if type == "string" then "" else .["dist.integrity"] // "" end' <<<"$metadata")"
  elif grep -Eq 'E401|E403|401 Unauthorized|403 Forbidden' "$error_file"; then
    record_state --channel "$package" --expected-version "$VERSION_NUMBER" --error-kind auth
  elif grep -Eq 'E404|404 Not Found|code E404' "$error_file"; then
    record_state --channel "$package" --expected-version "$VERSION_NUMBER"
  elif grep -Eiq 'E429|429 Too Many Requests|rate limit' "$error_file"; then
    record_state --channel "$package" --expected-version "$VERSION_NUMBER" --error-kind rate-limit
  elif grep -Eq 'E5[0-9]{2}|5[0-9]{2} (Internal|Bad|Service|Gateway)' "$error_file"; then
    record_state --channel "$package" --expected-version "$VERSION_NUMBER" --error-kind service
  else
    record_state --channel "$package" --expected-version "$VERSION_NUMBER" --error-kind network
  fi
  rm -f "$error_file"
}

check_homebrew_digests() {
  local formula="$1" target expected
  for target in \
    aarch64-apple-darwin \
    x86_64-apple-darwin \
    aarch64-unknown-linux-gnu \
    x86_64-unknown-linux-gnu; do
    expected="$(awk '{print $1}' "$verify_tmp/repopilot-${VERSION}-${target}.tar.gz.sha256")"
    if [[ -z "$expected" ]] || ! grep -Fq "sha256 \"$expected\"" <<<"$formula"; then
      echo "::error::published-mismatch; Homebrew formula digest does not match ${target}"
      exit 1
    fi
  done
}

# The formula indents its `version` line inside the class body.
formula_version_of() {
  sed -nE 's/^[[:space:]]*version "([^"]+)".*/\1/p' <<<"$1" | head -n 1
}

for attempt in $(seq 1 "$VERIFY_ATTEMPTS"); do
  echo "Publication verification attempt ${attempt}/${VERIFY_ATTEMPTS}"
  retry_needed=false
  if crate_metadata="$(curl -fsSL -A "$CRATES_USER_AGENT" "https://crates.io/api/v1/crates/repopilot/${VERSION_NUMBER}" 2>"$verify_tmp/crates.err")"; then
    record_state \
      --channel crates \
      --expected-version "$VERSION_NUMBER" \
      --observed-version "$(jq -r '.version.num' <<<"$crate_metadata")" \
      --expected-digest "$crate_checksum" \
      --observed-digest "$(jq -r '.version.checksum // empty' <<<"$crate_metadata")"
  elif grep -Eq '401|403|unauthorized|forbidden' "$verify_tmp/crates.err"; then
    record_state --channel crates --expected-version "$VERSION_NUMBER" --error-kind auth
  elif grep -Eq '429|too many requests|rate limit' "$verify_tmp/crates.err"; then
    record_state --channel crates --expected-version "$VERSION_NUMBER" --error-kind rate-limit
  elif grep -Eq '5[0-9]{2}|service unavailable|bad gateway' "$verify_tmp/crates.err"; then
    record_state --channel crates --expected-version "$VERSION_NUMBER" --error-kind service
  elif grep -Eq '404|not found' "$verify_tmp/crates.err"; then
    record_state --channel crates --expected-version "$VERSION_NUMBER"
  else
    record_state --channel crates --expected-version "$VERSION_NUMBER" --error-kind network
  fi

  check_npm "repopilot" "$root_integrity"
  # shellcheck disable=SC2016 # the JavaScript template literal is not shell
  while IFS=$'\t' read -r package package_dir; do
    package_integrity="$(cd "$package_dir" && npm pack --dry-run --json | jq -r '.[0].integrity')"
    check_npm "$package" "$package_integrity"
  done < <(node -e 'const fs=require("node:fs"); for (const pkg of JSON.parse(fs.readFileSync(process.argv[1], "utf8"))) console.log(`${pkg.packageName}\t${pkg.directory}`)' "$verify_tmp/npm-platform-packages.json")

  if formula="$(curl -fsSL "https://raw.githubusercontent.com/MykytaStel/homebrew-repopilot/main/Formula/repopilot.rb" 2>"$verify_tmp/formula.err")"; then
    formula_version="$(formula_version_of "$formula")"
    if [[ "$formula_version" == "$VERSION_NUMBER" ]]; then
      check_homebrew_digests "$formula"
      record_state --channel homebrew --expected-version "$VERSION_NUMBER" --observed-version "$formula_version"
    elif [[ -z "$formula_version" ]]; then
      record_state --channel homebrew --expected-version "$VERSION_NUMBER"
    else
      record_state --channel homebrew --expected-version "$VERSION_NUMBER" --observed-version "$formula_version"
    fi
  elif grep -Eq '401|403|unauthorized|forbidden' "$verify_tmp/formula.err"; then
    record_state --channel homebrew --expected-version "$VERSION_NUMBER" --error-kind auth
  elif grep -Eq '404|not found' "$verify_tmp/formula.err"; then
    record_state --channel homebrew --expected-version "$VERSION_NUMBER"
  elif grep -Eq '429|too many requests|rate limit' "$verify_tmp/formula.err"; then
    record_state --channel homebrew --expected-version "$VERSION_NUMBER" --error-kind rate-limit
  elif grep -Eq '5[0-9]{2}|service unavailable|bad gateway' "$verify_tmp/formula.err"; then
    record_state --channel homebrew --expected-version "$VERSION_NUMBER" --error-kind service
  else
    record_state --channel homebrew --expected-version "$VERSION_NUMBER" --error-kind network
  fi

  if [[ "$retry_needed" == false ]]; then
    echo "All public release channels match ${VERSION}."
    exit 0
  fi
  if (( attempt < VERIFY_ATTEMPTS )); then
    sleep "$VERIFY_SLEEP_SECONDS"
  fi
done

echo "::error::One or more public release channels did not converge to ${VERSION_NUMBER}"
exit 1
