# Record the Java "review an agent's change" demo GIF:
#
#   python3 scripts/zoo.py clone --only spring-petclinic
#   vhs docs/demos/04-java-agent-review.tape
#
# Run from the repo root. Requires `vhs` and a repopilot binary (the hidden
# setup prefers target/release). The scenario is applied by
# scripts/demo-java-agent-edit.sh: a plausible "add a thumbnail endpoint"
# edit to Spring PetClinic that shells out to ImageMagick with an unsanitized
# request parameter. Recorded on the pinned zoo checkout, staged into /tmp so
# no local paths appear on camera.

Output docs/demos/04-java-agent-review.gif

Set Shell bash
Set FontSize 14
Set Width 1280
Set Height 760
Set Padding 24
Set TypingSpeed 55ms

# Stage the checkout and apply the agent edit off-camera.
Hide
Type `export PATH="$PWD/target/release:$PATH" && rm -rf /tmp/petclinic && git clone -q --local .zoo/spring-petclinic /tmp/petclinic && bash scripts/demo-java-agent-edit.sh /tmp/petclinic >/dev/null && cd /tmp/petclinic && clear`
Enter
Show

# Looks harmless: one new endpoint, seven lines.
Type "git diff --stat"
Enter
Sleep 2.5s

# What actually moved: a request parameter reaching a shell command.
Type "repopilot review ."
Enter
Sleep 11s

# Clean up the staged checkout off-camera.
Hide
Type `cd / && rm -rf /tmp/petclinic`
Enter
