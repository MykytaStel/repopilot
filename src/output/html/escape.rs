pub(super) fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::escape_html;

    #[test]
    fn neutralizes_script_tags() {
        let escaped = escape_html("<script>alert('x')</script>");
        assert_eq!(
            escaped, "&lt;script&gt;alert('x')&lt;/script&gt;",
            "angle brackets must be escaped so a snippet cannot open a tag"
        );
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
    }

    #[test]
    fn neutralizes_attribute_injection() {
        // A snippet rendered inside `title="..."` must not be able to close the
        // attribute and inject a new one.
        let escaped = escape_html(r#"" onmouseover="steal()"#);
        assert_eq!(escaped, "&quot; onmouseover=&quot;steal()");
        assert!(!escaped.contains('"'));
    }

    #[test]
    fn escapes_ampersand_before_entities_to_avoid_double_encoding() {
        // `&` is replaced first, so a literal `&lt;` in source becomes
        // `&amp;lt;` (the entity is shown verbatim) rather than `&lt;`.
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("&lt;"), "&amp;lt;");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        let snippet = "let total = sum(items); // ok";
        assert_eq!(escape_html(snippet), snippet);
    }

    #[test]
    fn round_trips_a_realistic_code_snippet() {
        let snippet = r#"const html = `<div class="x">${user}</div>`;"#;
        let escaped = escape_html(snippet);
        assert_eq!(
            escaped,
            r#"const html = `&lt;div class=&quot;x&quot;&gt;${user}&lt;/div&gt;`;"#
        );
        // None of the four HTML-significant characters survive unescaped.
        for dangerous in ['<', '>', '"'] {
            assert!(
                !escaped.contains(dangerous),
                "{dangerous:?} leaked through escaping"
            );
        }
    }
}
