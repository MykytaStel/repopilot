# Post URLs

A post lives at `/posts/<slug>`. The slug is the title in lowercase ASCII,
with words joined by hyphens: "Café Crème" becomes `cafe-creme`.

Posts from the old blog used underscores. nginx redirects those URLs to the
hyphenated form, so the application never generates underscores.
