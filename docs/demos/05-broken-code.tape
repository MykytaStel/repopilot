# Record the "catch broken code" demo GIF:
#
#   vhs docs/demos/05-broken-code.tape
#
# Run from the repo root. Requires `vhs` (https://github.com/charmbracelet/vhs)
# and `repopilot` on PATH (e.g. `cargo install --path .`, or build and add
# target/release to PATH). The hidden setup reuses scripts/demo-broken-code.sh,
# so the recorded scenario always matches scripts/demo-broken-code-review.sh.

Output docs/demos/05-broken-code.gif

Set Shell bash
Set FontSize 15
Set Width 1280
Set Height 940
Set Padding 24
Set TypingSpeed 55ms

# Build the before/after scenario off-camera and step into it.
Hide
Type `DEMO=$(mktemp -d) && bash scripts/demo-broken-code.sh "$DEMO" && cd "$DEMO" && clear`
Enter
Show

# Looks like a tidy rename-and-move; nothing here says it's broken.
Type "git diff --stat"
Enter
Sleep 2.5s

# No compiler ran. Two callers still point at names that no longer exist.
Type "repopilot review ."
Enter
Sleep 9s

# Tidy up off-camera.
Hide
Type `cd - >/dev/null && rm -rf "$DEMO"`
Enter
