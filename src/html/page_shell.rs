//! Shared page-shell renderer for full-Spacetime (`.st`-only) routes.
//!
//! A route that has an `index.st` but NO authored `index.html` is a "full-Spacetime page": its
//! body markup is produced by the compiler (file-scope `<tag>` literals -> `CompiledSpacetime.html`,
//! PLAN-023 W1 + FEAT-078 hydration markers). Both the dev server and the static export must wrap
//! that markup in an identical HTML document shell \u2014 the ONLY difference is the asset hrefs (dev:
//! `/__spacetime/...?entry=`; export: the production bundle path) and a dev-only live-reload tail.
//!
//! Keeping ONE renderer here means the two paths can never drift (the dev server previously owned
//! a private `synthesize_st_only_shell`; export had none, so `.st`-only routes exported zero
//! pages \u2014 FUP-039). `body_html` is injected verbatim AHEAD of the runtime script so the page has
//! real SSG content (SEO / no-JS) and the runtime hydrates after.

/// Inputs for [`render_page_shell`]. `css_href`/`js_href` are written verbatim into the asset
/// tags (the caller is responsible for any needed escaping of those hrefs). `tail` is appended
/// just before `</body>` after the runtime script (dev server uses it for live-reload; export
/// passes `""`).
pub struct PageShell<'a> {
    pub title: &'a str,
    pub css_href: &'a str,
    pub js_href: &'a str,
    pub body_html: &'a str,
    pub tail: &'a str,
    /// `lang` for the generated `<html lang="…">`. A `.st` entry can override it by
    /// authoring a top-level `<html lang="fr">` wrapper (GH-21): the host spelling must
    /// work, and `<html lang>` is the host spelling for the document language.
    pub lang: &'a str,
}

/// Head-eligible element names: authoring one of these at file scope in a `.st` entry is
/// unambiguously a document-metadata intent, never page content. `<title>` and `<meta>` are
/// the load-bearing pair for SEO/social; `<link>` covers icons/preconnect; `<base>` is included
/// for completeness. `<style>`/`<script>` are deliberately NOT hoisted — Spacetime owns those
/// (styles come from the compiled stylesheet, and a page-authored script would violate the
/// no-custom-JS rule), so hoisting them would silently bless a bypass.
const HEAD_TAGS: [&str; 4] = ["title", "meta", "link", "base"];

/// Result of scanning a full-Spacetime entry's body markup for document-level metadata.
#[derive(Debug, Default, PartialEq)]
pub struct HeadExtraction {
    pub head: String,
    pub body: String,
    /// `lang` from an authored top-level `<html lang="…">` wrapper, if any (GH-21).
    pub lang: Option<String>,
}

/// Split `body_html` into (head_elements, remaining_body) plus an authored `<html lang>`.
///
/// A full-Spacetime page has no authored `index.html`, so its ONLY way to set a `<title>`, a
/// description, or the document `<html lang>` is to author it in the `.st` entry — where it lands
/// in the body markup and is inert (a `<title>` inside `<body>` does not set the document title,
/// a `<meta name="description">` there is ignored by crawlers, and `<html lang>` cannot even be
/// authored because the shell owns the `<html>` element). Hoisting them is what makes authoring
/// them mean what the author plainly intended (GH-21 / PLAN-035).
///
/// An authored top-level `<html lang="fr">…</html>` wrapper is recognized: its `lang` is
/// returned to feed the shell's own `<html lang>` (the shell is the real `<html>`), and the
/// wrapper tags are stripped so only their children (the page content) reach the body.
///
/// Only TOP-LEVEL occurrences are hoisted: the scan tracks nesting depth and lifts a head tag
/// solely at depth 0, so a `<link>` inside a `<nav>` (a real anchor-ish case in user markup) and
/// any `<meta>` nested in a component stay exactly where they were authored.
fn extract_head_elements(body_html: &str) -> HeadExtraction {
    let mut head = String::new();
    let mut body = String::with_capacity(body_html.len());
    let mut lang: Option<String> = None;
    let bytes = body_html.as_bytes();
    let mut i = 0usize;
    let mut depth = 0i32;

    while i < bytes.len() {
        if bytes[i] != b'<' {
            body.push(body_html[i..].chars().next().unwrap_or('<'));
            i += body_html[i..].chars().next().map_or(1, |c| c.len_utf8());
            continue;
        }
        // Comments and doctype/CDATA-ish: copy through verbatim, never parsed as tags.
        if body_html[i..].starts_with("<!DOCTYPE") || body_html[i..].starts_with("<!doctype") {
            let end = body_html[i..].find('>').map_or(bytes.len(), |p| i + p + 1);
            body.push_str(&body_html[i..end]);
            i = end;
            continue;
        }
        if body_html[i..].starts_with("<!--") {
            let end = body_html[i..].find("-->").map_or(bytes.len(), |p| i + p + 3);
            body.push_str(&body_html[i..end]);
            i = end;
            continue;
        }
        let Some(gt_rel) = body_html[i..].find('>') else {
            body.push_str(&body_html[i..]);
            break;
        };
        let tag_end = i + gt_rel + 1;
        let raw = &body_html[i..tag_end];
        let is_close = raw.starts_with("</");
        let name: String = raw
            .trim_start_matches("</")
            .trim_start_matches('<')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();

        // An authored top-level `<html …>` wrapper: the shell OWNS `<html>` (it is generated),
        // so the author's wrapper cannot stand — extract its `lang` (GH-21) and drop the wrapper
        // tags, letting only the children reach the body. `html` cannot nest, and `depth == 0`
        // guards against a stray one inside page markup.
        if name == "html" && depth == 0 {
            if !is_close && lang.is_none() {
                lang = extract_attr(raw, "lang");
            }
            i = tag_end;
            continue;
        }

        let head_eligible = depth == 0 && !is_close && HEAD_TAGS.contains(&name.as_str());
        if head_eligible {
            // `<title>` has a text child; `<meta>`/`<link>`/`<base>` are void. Take the whole
            // element in the first case so the hoist doesn't orphan its content in the body.
            let elem_end = if name == "title" {
                body_html[tag_end..]
                    .find("</title>")
                    .map_or(tag_end, |p| tag_end + p + "</title>".len())
            } else {
                tag_end
            };
            head.push_str("    ");
            head.push_str(body_html[i..elem_end].trim());
            head.push('\n');
            i = elem_end;
            // Swallow one trailing newline so hoisting doesn't leave a blank line behind.
            if body_html[i..].starts_with('\n') {
                i += 1;
            }
            continue;
        }

        // Depth tracking ignores void/self-closing elements, which never open a scope.
        let self_closing = raw.ends_with("/>")
            || matches!(
                name.as_str(),
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            );
        if !name.is_empty() && !self_closing {
            depth += if is_close { -1 } else { 1 };
            depth = depth.max(0);
        }
        body.push_str(raw);
        i = tag_end;
    }

    HeadExtraction {
        head,
        body,
        lang,
    }
}

/// Extract the value of an HTML attribute from a raw open-tag string, e.g. `lang` from
/// `<html lang="fr">`. Handles double-quoted, single-quoted, and bare values. Only a whole
/// attribute name matches (a preceding ident char or `-` disqualifies a substring like the
/// `lang` inside `language=`).
fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{}=", name);
    let bytes = tag.as_bytes();
    let mut pos = 0usize;
    while let Some(rel) = tag[pos..].find(&needle) {
        let start = pos + rel;
        let prev_ok = start == 0
            || !bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'-';
        if prev_ok {
            let rest = tag[start + needle.len()..].trim_start();
            let value = if let Some(q) = rest.strip_prefix('"') {
                q.split('"').next().map(|s| s.to_string())
            } else if let Some(q) = rest.strip_prefix('\'') {
                q.split('\'').next().map(|s| s.to_string())
            } else {
                rest.split(|c: char| c.is_ascii_whitespace() || c == '>')
                    .next()
                    .map(|s| s.to_string())
            };
            if value.is_some() {
                return value;
            }
        }
        pos = start + needle.len();
    }
    None
}
/// Render a complete HTML document for a full-Spacetime page: a FOUC-prevention style, the CSS
/// link, the compiler-produced body markup, then the runtime script (+ optional dev tail).
///
/// Head-eligible elements authored at the top level of the `.st` entry (`<title>`, `<meta>`,
/// `<link>`, `<base>`) are hoisted into `<head>`. An authored `<title>` REPLACES the default —
/// otherwise every `.st`-only page in existence would ship as "Spacetime", which is a real SEO
/// defect for any site built this way. An authored top-level `<html lang="fr">` wrapper sets the
/// generated `<html lang>` (defaulting to the caller's `shell.lang`, itself `"en"`).
pub fn render_page_shell(shell: &PageShell<'_>) -> String {
    let extraction = extract_head_elements(shell.body_html);
    let authored_title = extraction.head.contains("<title");
    let title_line = if authored_title {
        String::new()
    } else {
        format!("    <title>{}</title>\n", shell.title)
    };
    let lang = extraction.lang.as_deref().unwrap_or(shell.lang);
    let shell = PageShell {
        body_html: &extraction.body,
        lang,
        ..*shell
    };
    render_shell_inner(&shell, &title_line, &extraction.head)
}

fn render_shell_inner(shell: &PageShell<'_>, title_line: &str, head_extra: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
{title_line}{head_extra}    <!-- Spacetime FOUC Prevention -->
    <style>:not(:defined){{visibility:hidden}}</style>
    <link rel="stylesheet" href="{css_href}" />
</head>
<body>
{body_html}
    <!-- Spacetime Runtime (full-Spacetime page: markup is produced by the .st entry) -->
    <script src="{js_href}"></script>{tail}
</body>
</html>
"#,
        lang = shell.lang,
        title_line = title_line,
        head_extra = head_extra,
        css_href = shell.css_href,
        js_href = shell.js_href,
        body_html = shell.body_html,
        tail = shell.tail,
    )
}

/// Escape a string for use inside a double-quoted HTML attribute (e.g. an `?entry=` href value).
pub fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_body_before_runtime_script() {
        let html = render_page_shell(&PageShell {
            title: "Spacetime",
            css_href: "/spacetime.css",
            js_href: "/spacetime.js",
            body_html: "<main><h1>Hi</h1></main>",
            lang: "en",
            tail: "",
        });
        let body_pos = html.find("<main>").unwrap();
        let script_pos = html.find("/spacetime.js").unwrap();
        assert!(
            body_pos < script_pos,
            "markup must precede the runtime script"
        );
        assert!(html.contains(r#"<link rel="stylesheet" href="/spacetime.css" />"#));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn tail_is_appended_after_script() {
        let html = render_page_shell(&PageShell {
            title: "T",
            css_href: "/c.css",
            js_href: "/j.js",
            body_html: "",
            lang: "en",
            tail: "<!--RELOAD-->",
        });
        let script_pos = html.find("/j.js").unwrap();
        let tail_pos = html.find("<!--RELOAD-->").unwrap();
        assert!(script_pos < tail_pos, "tail comes after the runtime script");
    }

    // — Head hoisting (FEAT: `.st`-only pages can set their own document metadata) ————

    #[test]
    fn authored_title_replaces_the_default_and_leaves_body() {
        let html = render_page_shell(&PageShell {
            title: "Spacetime",
            css_href: "/c.css",
            js_href: "/j.js",
            body_html: "<title>Real Title</title>\n<main><h1>Hi</h1></main>",
            lang: "en",
            tail: "",
        });
        assert!(html.contains("<title>Real Title</title>"));
        assert!(
            !html.contains("<title>Spacetime</title>"),
            "the default title must not ALSO be emitted — two <title>s is the bug"
        );
        let head_end = html.find("</head>").unwrap();
        assert!(
            html.find("<title>Real Title").unwrap() < head_end,
            "an authored title must land in <head>, where it actually sets the document title"
        );
        assert!(
            html.contains("<h1>Hi</h1>"),
            "body content survives the hoist"
        );
    }

    #[test]
    fn meta_and_link_hoist_into_head() {
        let html = render_page_shell(&PageShell {
            title: "D",
            css_href: "/c.css",
            js_href: "/j.js",
            body_html: concat!(
                "<meta name=\"description\" content=\"desc\">\n",
                "<link rel=\"icon\" href=\"/f.svg\">\n",
                "<main>body</main>"
            ),
            lang: "en",
            tail: "",
        });
        let head_end = html.find("</head>").unwrap();
        assert!(html.find("name=\"description\"").unwrap() < head_end);
        assert!(html.find("rel=\"icon\"").unwrap() < head_end);
        assert!(
            html.find("<main>").unwrap() > head_end,
            "non-head markup stays in the body"
        );
        assert!(
            html.contains("<title>D</title>"),
            "the default title still applies when none was authored"
        );
    }

    #[test]
    fn nested_head_tags_are_not_hoisted() {
        // A <meta>/<link> INSIDE page markup is content-positioned by the author; lifting it
        // would silently reorder their DOM. Only depth-0 metadata is document metadata.
        let ex = extract_head_elements("<nav><link rel=\"x\" href=\"/y\"></nav><meta name=\"a\">");
        assert!(
            !ex.head.contains("rel=\"x\""),
            "a link nested in <nav> must stay put"
        );
        assert!(ex.body.contains("rel=\"x\""));
        assert!(
            ex.head.contains("name=\"a\""),
            "the depth-0 meta after the nav still hoists"
        );
    }

    #[test]
    fn style_and_script_are_never_hoisted() {
        // Spacetime owns styles/scripts; hoisting a page-authored one would bless a bypass of
        // the compiled stylesheet and the no-custom-JS rule.
        let ex = extract_head_elements("<style>a{color:red}</style><script src=\"/x.js\"></script>");
        assert!(ex.head.is_empty(), "neither <style> nor <script> may hoist");
        assert!(ex.body.contains("<style>") && ex.body.contains("<script"));
    }

    #[test]
    fn authored_html_lang_sets_the_shell_lang() {
        // GH-21: a `.st` entry sets `<html lang>` by authoring a top-level `<html lang="…">`
        // wrapper. The shell owns the real `<html>`, so the wrapper's lang feeds it and the
        // wrapper tags are stripped — only the children reach the body.
        let html = render_page_shell(&PageShell {
            title: "D",
            css_href: "/c.css",
            js_href: "/j.js",
            body_html: "<html lang=\"fr\"><main class=\"page\"><h1>Bonjour</h1></main></html>",
            lang: "en",
            tail: "",
        });
        assert!(
            html.contains("<html lang=\"fr\">"),
            "authored lang must reach the generated <html lang>"
        );
        assert!(
            !html.contains("lang=\"en\""),
            "the default en must not win over an authored lang"
        );
        assert!(html.contains("<h1>Bonjour</h1>"), "body survives the wrapper strip");
        assert!(
            !html.contains("<html lang=\"fr\"><main"),
            "the authored <html> wrapper tags must be stripped, not double-emitted"
        );
    }

    #[test]
    fn html_lang_defaults_to_en() {
        let html = render_page_shell(&PageShell {
            title: "D",
            css_href: "/c.css",
            js_href: "/j.js",
            body_html: "<main><h1>Hi</h1></main>",
            lang: "en",
            tail: "",
        });
        assert!(html.contains("<html lang=\"en\">"));
    }

    #[test]
    fn body_without_head_tags_is_returned_unchanged() {
        let src = "<main class=\"p\"><h1>Only content</h1><img src=\"/a.png\"></main>";
        let ex = extract_head_elements(src);
        assert!(ex.head.is_empty());
        assert_eq!(ex.body, src, "a page with no metadata must be byte-identical");
    }

    #[test]
    fn comments_and_unicode_survive_the_scan() {
        let src = "<!-- a > comment --><p>caf\u{e9} \u{2014} na\u{ef}ve</p>";
        let ex = extract_head_elements(src);
        assert!(ex.head.is_empty());
        assert_eq!(
            ex.body, src,
            "comment containing '>' and multi-byte text must round-trip"
        );
    }

    #[test]
    fn escape_attr_neutralizes_quotes_and_angles() {
        assert_eq!(escape_attr("a\"b<c>&d"), "a&quot;b&lt;c&gt;&amp;d");
    }
}
