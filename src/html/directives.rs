//! Nested-directive detection inside HTML literals (I4 / gh-8).
//!
//! A directive written INSIDE HTML markup —
//!
//! ```st
//! <div class="x"> @scroll fade(start: 0, end: 1) { opacity: 0 -> 1; } </div>
//! ```
//!
//! — cannot bind: directives attach through SELECTORS, never through nesting
//! (a directive must target elements that may not exist yet — an `@each` row,
//! a spliced fragment). The CST renders the whole element as ONE opaque
//! `HTML_RAW` token, so the nested directive is indistinguishable from page
//! text at the CST level. That is exactly the bug: the directive source is
//! silently painted onto the page as visible text.
//!
//! This module scans the raw markup for a directive *invocation* shape —
//! `@ident`, optionally followed by a driver/named `ident`, then `(args)` and/or
//! `{ body }` — and reports its extent so the caller can diagnose it (E0900)
//! and strip it from the rendered HTML. The shape gate (must reach `(` or `{`)
//! is what keeps literal `@`-containing text (`@email`, `Contact @ noon`,
//! `//@ comment`) from being misread as a directive.

/// True when `b` can start a directive name.
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

/// True when `b` continues a directive name.
fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// Skip a quoted run starting at `i` (the opening quote); returns the index just
/// past the closing quote.
fn skip_quoted(b: &[u8], i: usize) -> usize {
    let q = b[i];
    let mut j = i + 1;
    while j < b.len() {
        if b[j] == b'\\' {
            j += 2;
            continue;
        }
        if b[j] == q {
            return j + 1;
        }
        j += 1;
    }
    j
}

/// Extent (end-exclusive) of a directive invocation that starts at `start` (the
/// `@` byte). Caller has already confirmed the invocation shape. Consumes the
/// `@ident`, any `ident(...)` args and any `{ ... }` body via a balanced
/// brace/paren/bracket scan; returns at the first depth-0 non-whitespace char
/// after at least one group has been opened and closed.
fn directive_extent(b: &[u8], start: usize) -> usize {
    let mut depth: i32 = 0;
    let mut closed = false;
    let mut i = start;
    while i < b.len() {
        match b[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    closed = true;
                }
            }
            b'"' | b'\'' => {
                i = skip_quoted(b, i);
                continue;
            }
            _ if depth == 0 && closed && !b[i].is_ascii_whitespace() => return i,
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// Tags whose content is VERBATIM (rendered as-is, never as live page text a
/// nested directive would leak into): source-code display (`<pre>/<code>`),
/// raw scripts/styles, and text areas. A directive-shaped sequence inside one
/// is documentation or raw content, not a bind attempt (I4 / gh-8) — the
/// literate `.st.md` files self-display their own source in `<pre
/// class="lit-src"><code>` blocks, which would otherwise storm E0900.
const VERBATIM_TAGS: [&[u8]; 6] = [b"pre", b"code", b"script", b"style", b"textarea", b"template"];

/// If `html` at byte `i` opens a verbatim container, return the index just past
/// its matching close tag (`</tag …>`); otherwise `None`.
fn skip_verbatim_container(html: &str, b: &[u8], i: usize) -> Option<usize> {
    for tag in VERBATIM_TAGS {
        if b[i..].len() > tag.len()
            && b[i..i + tag.len()].eq_ignore_ascii_case(tag)
            && (b[i + tag.len()].is_ascii_whitespace() || b[i + tag.len()] == b'>')
        {
            // Skip to the matching close tag.
            let close = format!("</{}", std::str::from_utf8(tag).unwrap());
            if let Some(rel) = html[i..].to_ascii_lowercase().find(&close) {
                return Some(i + rel + close.len());
            }
            return Some(b.len());
        }
    }
    None
}

/// Find the first nested-directive invocation in `html`.
///
/// Returns `(start, end, name)` — the byte range (end-exclusive) of the whole
/// invocation (`@name … { … }`) and the directive name. `None` when the markup
/// holds no directive-shaped sequence (only literal `@` text, `//@` comments,
/// attribute values, or code inside a verbatim `<pre>/<code>/<script>` block).
pub fn find_nested_directive(html: &str) -> Option<(usize, usize, String)> {
    let b = html.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // Skip HTML comments wholesale.
        if b[i] == b'<' && html[i..].starts_with("<!--") {
            if let Some(rel) = html[i + 4..].find("-->") {
                i += 4 + rel + 3;
                continue;
            }
        }
        // Skip verbatim containers (`<pre>`, `<code>`, `<script>`, …).
        if b[i] == b'<' {
            if let Some(end) = skip_verbatim_container(html, b, i + 1) {
                i = end;
                continue;
            }
        }
        if b[i] != b'@' {
            i += 1;
            continue;
        }
        // A `@` preceded by `/` (`//@`, `/*@`, `/@`), a quote (attribute value
        // or quoted text), `=` (attribute), or `<` (tag) is not a directive
        // invocation in text position.
        let prev = if i > 0 { b[i - 1] } else { 0 };
        if prev == b'/' || prev == b'=' || prev == b'<' || prev == b'"' || prev == b'\'' {
            i += 1;
            continue;
        }
        // Parse `@ident`.
        let mut j = i + 1;
        if j >= b.len() || !is_ident_start(b[j]) {
            i += 1;
            continue;
        }
        while j < b.len() && is_ident(b[j]) {
            j += 1;
        }
        let name = html[i + 1..j].to_string();
        // Skip whitespace, then require `(`/`{` directly, or `ident(`/`ident{`.
        let mut k = j;
        while k < b.len() && b[k].is_ascii_whitespace() {
            k += 1;
        }
        let invocation = if k < b.len() && (b[k] == b'(' || b[k] == b'{') {
            true
        } else if k < b.len() && is_ident_start(b[k]) {
            let mut k2 = k;
            while k2 < b.len() && is_ident(b[k2]) {
                k2 += 1;
            }
            while k2 < b.len() && b[k2].is_ascii_whitespace() {
                k2 += 1;
            }
            k2 < b.len() && (b[k2] == b'(' || b[k2] == b'{')
        } else {
            false
        };
        if invocation {
            let end = directive_extent(b, i);
            return Some((i, end, name));
        }
        i += 1;
    }
    None
}

/// True when `b` is a markup/text boundary for whitespace-collapse purposes:
/// whitespace, a tag delimiter (`<`/`>`), or the string edge. Merging across
/// these is never word-joining, so the surrounding whitespace is safely dropped
/// (a directive as an element's sole content leaves the element text-empty).
fn is_boundary(b: u8) -> bool {
    b.is_ascii_whitespace() || b == b'<' || b == b'>'
}

/// Strip every nested-directive invocation from `html`, returning the clean
/// markup (compiler input is never painted as page text). Each directive is
/// removed WITH its immediately-surrounding whitespace so a directive that was
/// an element's sole content leaves the element text-empty; when the directive
/// sits between two text words (`<p>Hello @x World</p>`) a single space is kept
/// so words never join.
pub fn strip_nested_directives(html: &str) -> String {
    let mut out = html.to_string();
    loop {
        let Some((start, end, _)) = find_nested_directive(&out) else {
            break;
        };
        let b = out.as_bytes();
        // Widen to surrounding whitespace runs.
        let mut s = start;
        while s > 0 && b[s - 1].is_ascii_whitespace() {
            s -= 1;
        }
        let mut e = end;
        while e < b.len() && b[e].is_ascii_whitespace() {
            e += 1;
        }
        // Merge decision: keep one space only when real text flanks BOTH sides.
        let before_is_text = s > 0 && !is_boundary(b[s - 1]);
        let after_is_text = e < b.len() && !is_boundary(b[e]);
        if before_is_text && after_is_text {
            out.replace_range(s..e, " ");
        } else {
            out.replace_range(s..e, "");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_scroll_in_div() {
        let html = "<div class=\"x\"> @scroll fade(start: 0, end: 1) { opacity: 0 -> 1; } </div>";
        let (s, e, name) = find_nested_directive(html).expect("nested @scroll detected");
        assert_eq!(name, "scroll");
        assert!(html[s..].starts_with("@scroll"));
        // The extent consumes the whole invocation INCLUDING its body and stops
        // before the closing `</div>` tag.
        assert!(html[s..e].contains("opacity: 0 -> 1"), "body included in extent");
        assert!(!html[s..e].contains("</div>"), "extent stops before the closing tag");
    }

    #[test]
    fn literal_at_text_is_not_a_directive() {
        for text in [
            "<p>Contact us @ noon please</p>",
            "<p>Email me @example.com</p>",
            "<p>//@ not a comment directive</p>",
            "<div data-x=\"@media(small)\">hi</div>",
        ] {
            assert!(find_nested_directive(text).is_none(), "false positive on {text}");
        }
    }

    #[test]
    fn strips_directive_leaving_empty_text() {
        let html = "<div class=\"x\"> @scroll fade(start: 0, end: 1) { opacity: 0 -> 1; } </div>";
        let clean = strip_nested_directives(html);
        assert_eq!(clean, "<div class=\"x\"></div>", "sole-content directive leaves the element text-empty");
    }

    #[test]
    fn strips_directive_between_words_keeps_space() {
        let html = "<p>Hello @scroll fade() { x: 1 } World</p>";
        let clean = strip_nested_directives(html);
        assert_eq!(clean, "<p>Hello World</p>");
    }

    #[test]
    fn skips_html_comments() {
        let html = "<div><!-- @scroll fade() { x } --></div>";
        assert!(find_nested_directive(html).is_none());
    }

    #[test]
    fn skips_verbatim_code_blocks() {
        // Literate `.st.md` files self-display their own source in `<pre
        // class="lit-src"><code>` blocks — a directive inside is documentation,
        // not a nested bind.
        let html = "<pre class=\"lit-src\" data-lit-src=\"x\"><code>@type Task {\n  title: string\n}\n.plus { @on &.click { $c <- 1; } }\n</code></pre>";
        assert!(find_nested_directive(html).is_none(), "directives in <pre><code> are doc, not binds");
        assert_eq!(strip_nested_directives(html), html);
    }
}

/// Find a COMPONENT CALL written directly in HTML markup: `&name(args)`.
///
/// PLAN-144 W1 / BUG-344. A template invocation is a directive, so it binds
/// through a selector — never through nesting. Written into markup it used to
/// be HTML-escaped and shipped as body copy (`&amp;card("T")`) with the build
/// reporting success, which is the same silence `find_nested_directive` was
/// written to end for `@each`. Same rule, same diagnostic shape.
///
/// Strictly the CALL shape `&ident(` (PLAN-144 Q2): a BARE `&name` is a scope
/// reference — a score subject (`&showcase-hero-mtn__title for 35%`), a
/// selector target, or literal prose — and must never be refused. An HTML
/// entity (`&amp;`, `&#39;`) is never a call either: it has no `(`.
///
/// Returns `(start, end, name)` on the first call found, mirroring
/// `find_nested_directive`'s contract.
pub fn find_nested_component_call(html: &str) -> Option<(usize, usize, String)> {
    let b = html.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // Skip HTML comments wholesale.
        if b[i] == b'<' && html[i..].starts_with("<!--") {
            if let Some(rel) = html[i + 4..].find("-->") {
                i += 4 + rel + 3;
                continue;
            }
        }
        // Skip verbatim containers (`<pre>`, `<code>`, `<script>`, …) — a call
        // shown as example text there is CONTENT, not compiler input.
        if b[i] == b'<' {
            if let Some(end) = skip_verbatim_container(html, b, i + 1) {
                i = end;
                continue;
            }
        }
        if b[i] != b'&' {
            i += 1;
            continue;
        }
        // An `&` inside an attribute value or immediately after `=`/`<` is not a
        // call in text position.
        let prev = if i > 0 { b[i - 1] } else { 0 };
        if prev == b'=' || prev == b'<' || prev == b'"' || prev == b'\'' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if j >= b.len() || !is_ident_start(b[j]) {
            i += 1;
            continue;
        }
        while j < b.len() && (is_ident(b[j]) || b[j] == b'-') {
            j += 1;
        }
        // The call shape requires `(` IMMEDIATELY after the name. A bare
        // `&name` (or `&amp;`, which ends at `;`) is not a call.
        if j >= b.len() || b[j] != b'(' {
            i = j.max(i + 1);
            continue;
        }
        let name = html[i + 1..j].to_string();
        // Find the matching `)`, respecting quotes and nesting.
        let mut k = j;
        let mut depth = 0i32;
        let mut quote: Option<u8> = None;
        let mut escape = false;
        while k < b.len() {
            let c = b[k];
            if let Some(q) = quote {
                if escape {
                    escape = false;
                } else if c == b'\\' {
                    escape = true;
                } else if c == q {
                    quote = None;
                }
            } else if c == b'"' || c == b'\'' {
                quote = Some(c);
            } else if c == b'(' {
                depth += 1;
            } else if c == b')' {
                depth -= 1;
                if depth == 0 {
                    k += 1;
                    break;
                }
            }
            k += 1;
        }
        if depth == 0 && k > j {
            // Consume an optional trailing `;` so the strip leaves no orphan.
            let end = if k < b.len() && b[k] == b';' { k + 1 } else { k };
            return Some((i, end, name));
        }
        i = j;
    }
    None
}

/// Strip every nested component call from `html`, so compiler input is never
/// painted as page text. Mirrors `strip_nested_directives`, including its
/// whitespace-collapse rule (a call that was an element's sole content leaves
/// the element text-empty; one between two words keeps a single space).
pub fn strip_nested_component_calls(html: &str) -> String {
    let mut out = html.to_string();
    loop {
        let Some((start, end, _)) = find_nested_component_call(&out) else {
            break;
        };
        let b = out.as_bytes();
        let left_boundary = start == 0 || is_boundary(b[start - 1]);
        let right_boundary = end >= b.len() || is_boundary(b[end]);
        let (cut_start, cut_end) = if left_boundary && right_boundary {
            let mut s = start;
            while s > 0 && b[s - 1].is_ascii_whitespace() {
                s -= 1;
            }
            let mut e = end;
            while e < b.len() && b[e].is_ascii_whitespace() {
                e += 1;
            }
            (s, e)
        } else {
            (start, end)
        };
        let joined = format!("{}{}", &out[..cut_start], &out[cut_end..]);
        out = joined;
    }
    out
}
