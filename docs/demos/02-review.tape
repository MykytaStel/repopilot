# Record the RepoPilot review demo GIF:
#
#   vhs docs/demos/02-review.tape
#
# Run from the repo root. Requires `vhs` (https://github.com/charmbracelet/vhs)
# and `repopilot` on PATH (e.g. `cargo install --path .`, or build and add
# target/release to PATH). The hidden setup reuses scripts/demo-setup.sh, so the
# recorded scenario always matches scripts/demo-boundary-review.sh.

Output docs/demos/02-review.gif

Set Shell bash
Set FontSize 15
Set Width 1280
Set Height 940
Set Padding 24
Set TypingSpeed 55ms

# Build the before/after scenario off-camera and step into it.
Hide
Type `DEMO=$(mktemp -d) && bash scripts/demo-setup.sh "$DEMO" && cd "$DEMO" && clear`
Enter
Show

# The "aha": a one-line login fix that quietly loosened CORS and touched auth.
Type "repopilot review ."
Enter
Sleep 7s

# Tidy up off-camera.
Hide
Type `cd - >/dev/null && rm -rf "$DEMO"`
Enter
