#!/usr/bin/env bash
set -euo pipefail

# Apply the "agent edit" scenario to a Spring PetClinic checkout: a plausible
# "add a thumbnail endpoint" change that shells out to ImageMagick with an
# unsanitized request parameter — a request-input-to-subprocess flow that the
# Java taint frontend catches.
#
#   python3 scripts/zoo.py clone --only spring-petclinic
#   scripts/demo-java-agent-edit.sh .zoo/spring-petclinic
#   repopilot review .zoo/spring-petclinic
#
# Idempotent: resets the touched file before applying.
#
# Usage: demo-java-agent-edit.sh <spring-petclinic-checkout>

TARGET="${1:?usage: demo-java-agent-edit.sh <spring-petclinic-checkout>}"
FILE="src/main/java/org/springframework/samples/petclinic/owner/OwnerController.java"

cd "$TARGET"
git checkout -- "$FILE"

git apply --whitespace=nowarn - <<'PATCH'
diff --git a/src/main/java/org/springframework/samples/petclinic/owner/OwnerController.java b/src/main/java/org/springframework/samples/petclinic/owner/OwnerController.java
--- a/src/main/java/org/springframework/samples/petclinic/owner/OwnerController.java
+++ b/src/main/java/org/springframework/samples/petclinic/owner/OwnerController.java
@@ -138,6 +138,13 @@ class OwnerController {
 		return VIEWS_OWNER_CREATE_OR_UPDATE_FORM;
 	}

+	@GetMapping("/owners/{ownerId}/thumbnail")
+	public String generateThumbnail(HttpServletRequest request) throws Exception {
+		String size = request.getParameter("size");
+		Runtime.getRuntime().exec("convert avatar.png -resize " + size + " thumb.png");
+		return "redirect:/owners";
+	}
+
 	@PostMapping("/owners/{ownerId}/edit")
 	public String processUpdateOwnerForm(@Valid Owner owner, BindingResult result, @PathVariable("ownerId") int ownerId,
 			RedirectAttributes redirectAttributes) {
PATCH

echo "agent edit applied to $TARGET/$FILE"
