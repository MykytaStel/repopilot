# Record the "weakened tests" demo GIF:
#
#   vhs docs/demos/06-weakened-tests.tape
#
# Run from the repo root. Requires `vhs` (https://github.com/charmbracelet/vhs)
# and `repopilot` on PATH (e.g. `cargo install --path .`, or build and add
# target/release to PATH). The hidden setup reuses scripts/demo-weakened-tests.sh,
# so the recorded scenario always matches the README example.

Output docs/demos/06-weakened-tests.gif

Set Shell bash
Set FontSize 14
Set Width 1560
Set Height 940
Set Padding 24
Set TypingSpeed 55ms

# Build the before/after scenario off-camera and step into it.
Hide
Type `DEMO=$(mktemp -d)/pricing && bash scripts/demo-weakened-tests.sh "$DEMO" && cd "$DEMO" && clear`
Enter
Show

# A small pricing change. Every test that still runs passes, and CI stays green.
Type "git diff --stat"
Enter
Sleep 2.5s

# The review reports each check the change weakened.
Type "repopilot review ."
Enter
Sleep 9s

# Tidy up off-camera.
Hide
Type `cd - >/dev/null && rm -rf "$(dirname "$DEMO")"`
Enter
