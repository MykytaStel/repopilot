# Record the "review an agent's change" demo GIF:
#
#   python3 scripts/zoo.py clone --only wagtail   # once, pins the checkout
#   vhs docs/demos/03-agent-review.tape
#
# Run from the repo root. Requires `vhs` and a repopilot binary (the hidden
# setup prefers target/release, falling back to PATH). The scenario is applied
# by scripts/demo-agent-edit.sh: a plausible "optimize images" edit to Wagtail
# that also drops a permission check and pipes request input into a shell
# command. Recorded on the real pinned Wagtail checkout from the zoo, staged
# into /tmp/wagtail so no local paths appear on camera.

Output docs/demos/03-agent-review.gif

Set Shell bash
Set FontSize 14
Set Width 1280
Set Height 960
Set Padding 24
Set TypingSpeed 55ms

# Stage the checkout and apply the agent edit off-camera.
Hide
Type `export PATH="$PWD/target/release:$PATH" && rm -rf /tmp/wagtail && git clone -q --local .zoo/wagtail /tmp/wagtail && bash scripts/demo-agent-edit.sh /tmp/wagtail >/dev/null && cd /tmp/wagtail && clear`
Enter
Show

# Looks harmless: one file, a dozen lines.
Type "git diff --stat"
Enter
Sleep 2.5s

# What actually moved: auth check gone, request input reaching a shell.
Type "repopilot review ."
Enter
Sleep 12s

# Clean up the staged checkout off-camera.
Hide
Type `cd / && rm -rf /tmp/wagtail`
Enter
