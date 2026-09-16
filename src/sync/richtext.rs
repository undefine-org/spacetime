//! Rich-text sanitization for the CMS (Stage-2 Wave-3, FUP-029).
//!
//! Rich text in Spacetime is a CONSTRAINED subtree of the same HTML/AST the
//! compiler already parses (`HtmlExpr`) — not a bespoke document format. The
//! policy is a single allow/denylist over node kinds + attributes; "the list IS
//! the policy" (per FUP-029). This is the SECURITY-BEARING enforcement point:
//! the server sanitizes every rich-text value before it touches disk, so a
//! malicious or buggy client cannot persist `<script>`, event handlers, or
//! `javascript:` URLs into content.
//!
//! We reuse `lol_html` (already a dependency, used by the export pipeline) to
//! parse + rewrite, rather than regex — a real HTML tokenizer is both stronger
//! and AGENTS.md-aligned (don't hand-roll an HTML parser).

use lol_html::{RewriteStrSettings, element, rewrite_str};

/// Tags allowed in rich-text content. Everything structural a writer needs:
/// headings, paragraphs, lists, quotes, media, links, and inline marks. Any tag
/// NOT in this set is unwrapped (its children/text are kept, the tag dropped).
const ALLOWED_TAGS: &[&str] = &[
    // structure
    "h1",
    "h2",
    "h3",
    "h4",
    "p",
    "ul",
    "ol",
    "li",
    "blockquote",
    "figure",
    "figcaption",
    "img",
    "a",
    "hr",
    "br",
    "pre",
    // inline marks
    "strong",
    "em",
    "b",
    "i",
    "code",
    "u",
    "s",
    "mark",
    "sub",
    "sup",
    "span",
];

/// Attributes allowed on any element. Everything else (notably `on*` event
/// handlers and inline `style`) is stripped. `href`/`src` are allowed but
/// scheme-checked below.
const ALLOWED_ATTRS: &[&str] = &[
    "href",
    "src",
    "alt",
    "title",
    "class",
    // Spacetime data-binding + provenance hooks are legitimate inside content:
    "data-st-id",
    "data-st-origin",
    "data-st-bind",
];

/// Returns true if `s` is the kind of string that might carry rich-text HTML.
/// Plain scalar values (a name, a number-as-string) never contain `<…>`, so we
/// skip sanitization for them — keeps plain-text edits byte-exact.
pub fn looks_like_html(s: &str) -> bool {
    // Gate on '<' ALONE: an unclosed tag like `<img src=x onerror=...` (no '>')
    // is auto-completed to live HTML by a browser under innerHTML, so it MUST be
    // sanitized too. Plain values without any '<' can never be markup.
    s.contains('<')
}

/// Sanitize a rich-text HTML string against the allow/denylist. Disallowed
/// elements are unwrapped (text preserved); disallowed attributes are removed;
/// `javascript:`/`data:`/`vbscript:` URLs on href/src are dropped.
///
/// Infallible: on a parser error it falls back to escaping the whole string
/// (fail-closed — never persist unsanitized markup).
pub fn sanitize_richtext(html: &str) -> String {
    let element_content_handlers = vec![
        // Strip dangerous/unknown elements: keep children, drop the tag.
        element!("*", |el| {
            let tag = el.tag_name().to_ascii_lowercase();
            if !ALLOWED_TAGS.contains(&tag.as_str()) {
                // Raw-text / RCData / script-data elements: lol_html tokenizes
                // their inner content as a SINGLE text token the `*` handler
                // never visits, so keep-content would re-emit inner markup
                // verbatim as LIVE html (mXSS). DROP these entirely.
                if is_rawtext_tag(&tag) {
                    el.remove();
                } else {
                    el.remove_and_keep_content();
                }
                return Ok(());
            }
            // Prune attributes not on the allowlist (drops on* handlers, style…).
            let attrs: Vec<String> = el.attributes().iter().map(|a| a.name()).collect();
            for name in attrs {
                if !ALLOWED_ATTRS.contains(&name.as_str()) {
                    el.remove_attribute(&name);
                }
            }
            // Scheme-check URL attributes: only allow safe schemes / relative.
            for url_attr in ["href", "src"] {
                if let Some(val) = el.get_attribute(url_attr)
                    && !is_safe_url(&val)
                {
                    el.remove_attribute(url_attr);
                }
            }
            Ok(())
        }),
    ];

    let rewritten = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers,
            ..RewriteStrSettings::default()
        },
    )
    .unwrap_or_else(|_| escape_html(html));

    // Defense-in-depth / fail-closed: lol_html emits an UNTERMINATED trailing
    // tag (e.g. `<img src=x onerror=...` with no '>') as verbatim text without
    // ever invoking the element handler, so a dangerous residue can survive a
    // single pass. If the output still contains an event-handler attribute or a
    // script-capable URL scheme, escape the whole value (never persist live
    // markup). This catches unclosed-tag and any other tokenizer-edge bypass.
    if has_dangerous_residue(&rewritten) {
        return escape_html(html);
    }
    rewritten
}

/// True if a (supposedly sanitized) string still contains an event-handler
/// attribute (`on...=`) or a script-capable URL scheme — the fail-closed signal.
fn has_dangerous_residue(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if lower.contains("javascript:") || lower.contains("vbscript:") {
        return true;
    }
    // `on<word>=` event handler pattern (e.g. onerror=, onclick=, on load =).
    let bytes = lower.as_bytes();
    let mut i = 0;
    while let Some(pos) = lower[i..].find("on") {
        let start = i + pos;
        let mut j = start + 2;
        // require at least one ascii-alpha after `on`
        let mut saw_alpha = false;
        while j < bytes.len() && (bytes[j] as char).is_ascii_alphabetic() {
            j += 1;
            saw_alpha = true;
        }
        // skip optional whitespace, then look for '='
        while j < bytes.len() && (bytes[j] as char).is_whitespace() {
            j += 1;
        }
        if saw_alpha && j < bytes.len() && bytes[j] == b'=' {
            // ensure `on` starts at a token boundary (preceded by space/</quote)
            let prev_ok = start == 0
                || matches!(
                    bytes[start - 1] as char,
                    ' ' | '\t' | '\n' | '\r' | '"' | '\'' | '<' | '/'
                );
            if prev_ok {
                return true;
            }
        }
        i = start + 2;
    }
    false
}

/// Raw-text / RCData / script-data / plaintext elements whose inner content
/// lol_html surfaces as one opaque text token (never parsed as children). These
/// MUST be dropped whole — keeping their content would re-emit inner markup as
/// live HTML (mXSS).
fn is_rawtext_tag(tag: &str) -> bool {
    matches!(
        tag,
        "script"
            | "style"
            | "iframe"
            | "xmp"
            | "noembed"
            | "noframes"
            | "noscript"
            | "textarea"
            | "title"
            | "plaintext"
            | "template"
            | "svg"
            | "math"
    )
}

/// A URL is safe if it is relative, an anchor, or uses http/https/mailto/tel.
/// Rejects javascript:, data:, vbscript:, and other script-capable schemes.
fn is_safe_url(url: &str) -> bool {
    let u = url.trim();
    let lower = u.to_ascii_lowercase();
    // Strip leading control/whitespace chars that browsers ignore in schemes.
    let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.starts_with("javascript:")
        || compact.starts_with("data:")
        || compact.starts_with("vbscript:")
    {
        return false;
    }
    // Relative / anchor / fragment / query — safe.
    if u.starts_with('/') || u.starts_with('#') || u.starts_with('?') || u.starts_with('.') {
        return true;
    }
    // Has a scheme? Only allow a known-safe set.
    if let Some(idx) = compact.find(':') {
        // If the colon is after a '/', it's a path (e.g. foo/bar:baz) — relative.
        let before_colon = &compact[..idx];
        if before_colon.contains('/') {
            return true;
        }
        return matches!(before_colon, "http" | "https" | "mailto" | "tel");
    }
    // No scheme, no leading slash: a bare relative path like "page.html".
    true
}

/// Escape HTML special chars (fail-closed fallback).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_structural_and_inline() {
        let input = "<h2>Title</h2><p>Hello <strong>bold</strong> <a href=\"/x\">link</a></p>";
        let out = sanitize_richtext(input);
        assert!(out.contains("<h2>Title</h2>"));
        assert!(out.contains("<strong>bold</strong>"));
        assert!(out.contains("<a href=\"/x\">link</a>"));
    }

    #[test]
    fn strips_script_tag_keeps_nothing_executable() {
        let out = sanitize_richtext("<p>ok</p><script>alert(1)</script>");
        assert!(out.contains("<p>ok</p>"));
        assert!(!out.to_ascii_lowercase().contains("<script"));
    }

    #[test]
    fn rawtext_tag_inner_markup_dropped_not_reemitted() {
        // mXSS: inner markup of script/xmp/noscript/textarea must NOT survive as
        // live HTML after unwrapping. These tags are dropped WHOLE.
        for payload in [
            "<script><img src=x onerror=alert(1)></script>",
            "<xmp><img src=x onerror=alert(1)></xmp>",
            "<noscript><img src=x onerror=alert(1)></noscript>",
            "<textarea><img src=x onerror=alert(1)></textarea>",
            "<svg><script>alert(1)</script></svg>",
        ] {
            let out = sanitize_richtext(payload).to_ascii_lowercase();
            assert!(
                !out.contains("<img"),
                "inner markup leaked from {}: {}",
                payload,
                out
            );
            assert!(
                !out.contains("onerror"),
                "event handler leaked from {}: {}",
                payload,
                out
            );
        }
    }

    #[test]
    fn unclosed_tag_is_sanitized() {
        // P0: `<img ...` with no '>' must not survive as LIVE markup. lol_html
        // emits the unterminated tag verbatim, so the fail-closed residue check
        // escapes the whole value — no unescaped '<' remains, so a browser
        // cannot instantiate the element.
        assert!(looks_like_html("<img src=x onerror=alert(1)"));
        let out = sanitize_richtext("<img src=x onerror=alert(1)");
        assert!(!out.contains('<'), "no live '<' may survive: {}", out);
        assert!(
            out.contains("&lt;"),
            "dangerous markup must be escaped: {}",
            out
        );
    }

    #[test]
    fn strips_event_handlers() {
        let out = sanitize_richtext("<p onclick=\"evil()\">hi</p>");
        assert!(out.contains("<p>hi</p>") || out.contains("<p >hi</p>"));
        assert!(!out.to_ascii_lowercase().contains("onclick"));
    }

    #[test]
    fn strips_javascript_url() {
        let out = sanitize_richtext("<a href=\"javascript:alert(1)\">x</a>");
        assert!(!out.to_ascii_lowercase().contains("javascript:"));
    }

    #[test]
    fn strips_data_url_on_img() {
        let out = sanitize_richtext("<img src=\"data:text/html;base64,PHNjcmlwdD4=\">");
        assert!(!out.to_ascii_lowercase().contains("data:"));
    }

    #[test]
    fn strips_inline_style_and_unknown_tags() {
        let out = sanitize_richtext(
            "<p style=\"x\">a</p><marquee>b</marquee><iframe src=\"//e\"></iframe>",
        );
        assert!(!out.to_ascii_lowercase().contains("style="));
        assert!(!out.to_ascii_lowercase().contains("<marquee"));
        assert!(!out.to_ascii_lowercase().contains("<iframe"));
        // unwrapped content preserved
        assert!(out.contains('b'));
    }

    #[test]
    fn keeps_spacetime_binding_attrs() {
        let out = sanitize_richtext("<img src=\"/a.jpg\" data-st-bind=\"x\">");
        assert!(out.contains("data-st-bind"));
    }

    #[test]
    fn plain_text_detection() {
        assert!(!looks_like_html("Just a name"));
        // Gate is now '<' alone (fail-safe): "3 < 5" routes through the sanitizer,
        // which preserves it as text (lol_html treats `< 5` as text).
        assert!(looks_like_html("3 < 5 but no tags"));
        assert_eq!(sanitize_richtext("3 < 5 but no tags"), "3 < 5 but no tags");
        assert!(looks_like_html("<p>x</p>"));
        assert!(!looks_like_html("plain"));
    }

    #[test]
    fn allows_http_and_mailto() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("http://example.com"));
        assert!(is_safe_url("mailto:a@b.com"));
        assert!(is_safe_url("/relative"));
        assert!(is_safe_url("#anchor"));
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("data:text/html,x"));
        assert!(!is_safe_url("  javascript:alert(1)"));
    }
}
