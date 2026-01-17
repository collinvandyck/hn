/// Strip HTML tags and decode entities for terminal display.
///
/// Converts common HTML formatting to markdown-style equivalents:
/// - `<i>` → `_italic_`
/// - `<b>` → `*bold*`
/// - `<code>` → `` `code` ``
/// - `<pre>` → fenced code blocks
/// - Links → just the link text
///
/// Also decodes HTML entities and normalizes whitespace.
pub fn strip_html(html: &str) -> String {
    html.replace("<p>", "\n\n")
        .replace("</p>", "")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<i>", "_")
        .replace("</i>", "_")
        .replace("<b>", "*")
        .replace("</b>", "*")
        .replace("<code>", "`")
        .replace("</code>", "`")
        .replace("<pre>", "\n```\n")
        .replace("</pre>", "\n```\n")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&#x2F;", "/")
        .split("<a ")
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_string()
            } else if let Some(start) = part.find('>')
                && let Some(end) = part.find("</a>")
            {
                let link_text = &part[start + 1..end];
                let rest = &part[end + 4..];
                format!("{}{}", link_text, rest)
            } else {
                part.to_string()
            }
        })
        .collect::<String>()
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_basic_tags() {
        assert_eq!(strip_html("<p>Hello</p><p>World</p>"), "Hello World");
        assert_eq!(strip_html("Line1<br>Line2"), "Line1 Line2");
    }

    #[test]
    fn test_strip_html_formatting() {
        assert_eq!(strip_html("<i>italic</i>"), "_italic_");
        assert_eq!(strip_html("<b>bold</b>"), "*bold*");
        assert_eq!(strip_html("<code>code</code>"), "`code`");
    }

    #[test]
    fn test_strip_html_entities() {
        assert_eq!(strip_html("&lt;tag&gt;"), "<tag>");
        assert_eq!(strip_html("&amp;&quot;&#x27;"), "&\"'");
        assert_eq!(strip_html("path&#x2F;to&#x2F;file"), "path/to/file");
    }

    #[test]
    fn test_strip_html_links() {
        let html = r#"Check <a href="https://example.com">this link</a> out"#;
        assert_eq!(strip_html(html), "Check this link out");
    }

    #[test]
    fn test_strip_html_collapses_whitespace() {
        assert_eq!(strip_html("  too   many    spaces  "), "too many spaces");
        assert_eq!(strip_html("<p>  \n\n  </p>text"), "text");
    }
}
