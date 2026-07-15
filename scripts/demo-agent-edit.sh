#!/usr/bin/env bash
set -euo pipefail

# Apply the "agent edit" scenario to a Wagtail checkout: a plausible
# image-optimization change that also drops a permission check and pipes
# request input into a shell command. Used by docs/demos/03-agent-review.tape.
#
# The target is normally the pinned zoo clone:
#
#   python3 scripts/zoo.py clone --only wagtail
#   scripts/demo-agent-edit.sh .zoo/wagtail
#
# Idempotent: resets the touched file before applying.
#
# Usage: demo-agent-edit.sh <wagtail-checkout>

TARGET="${1:?usage: demo-agent-edit.sh <wagtail-checkout>}"
FILE="wagtail/images/views/images.py"

cd "$TARGET"
git checkout -- "$FILE"

git apply --whitespace=nowarn - <<'PATCH'
diff --git a/wagtail/images/views/images.py b/wagtail/images/views/images.py
--- a/wagtail/images/views/images.py
+++ b/wagtail/images/views/images.py
@@ -1,5 +1,6 @@
 import json
 import os
+import subprocess
 from tempfile import SpooledTemporaryFile

 from django.conf import settings
@@ -261,12 +262,17 @@ class EditView(generic.EditView):
         return kwargs

     def get_object(self, queryset=None):
-        obj = super().get_object(queryset)
-        if not permission_policy.user_has_permission_for_instance(
-            self.request.user, "change", obj
-        ):
-            raise PermissionDenied
-        return obj
+        return super().get_object(queryset)
+
+    def save_instance(self):
+        instance = super().save_instance()
+        quality = self.request.GET.get("quality", "85")
+        subprocess.run(
+            "mogrify -quality " + quality + " " + instance.file.path,
+            shell=True,
+            check=False,
+        )
+        return instance

     def get_success_message(self):
         return _("Image '%(image_title)s' updated.") % {
PATCH

echo "agent edit applied to $TARGET/$FILE"
