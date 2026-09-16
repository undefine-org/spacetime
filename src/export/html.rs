//! HTML transform passes for emitted pages.
//!
//! Composable rewriter passes: strip dev attrs, strip dev/prod __spacetime
//! tags, inject the production CSS/JS bundle links.
//!
//! Stubbed in Wave 0; populated TDD-style in Wave 1 by the HTML task.

/// Process an HTML document end-to-end: strip dev-only attributes and tags,
/// inject the production bundle links. Idempotent.
#[allow(dead_code)]
pub(crate) fn process_html(html: &str, css_href: &str, js_href: &str) -> String {
    use lol_html::{HtmlRewriter, Settings, element};

    let css_tag = format!(r#"<link rel="stylesheet" href="{}">"#, css_href);
    let js_tag = format!(r#"<script src="{}"></script>"#, js_href);

    let mut output = Vec::new();
    {
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![
                    // Strip dev attributes from every element
                    element!("*", |el| {
                        for attr in &[
                            "data-st-id",
                            "data-st-origin",
                            "data-st-bind",
                            "data-st-source",
                            "data-st-index",
                            "data-st-key",
                            "data-st-filter",
                            "data-st-editing",
                            "data-st-saving",
                        ] {
                            el.remove_attribute(attr);
                        }
                        Ok(())
                    }),
                    // Remove dev/prod __spacetime tags and previously-injected bundle tags
                    element!("script, link[rel='stylesheet']", |el| {
                        let is_spacetime = el
                            .get_attribute("src")
                            .map(|s| s.contains("__spacetime") || s.ends_with("/spacetime.js"))
                            .unwrap_or(false)
                            || el
                                .get_attribute("href")
                                .map(|h| h.contains("__spacetime") || h.ends_with("/spacetime.css"))
                                .unwrap_or(false);
                        if is_spacetime {
                            el.remove();
                        }
                        Ok(())
                    }),
                    // Inject CSS before </head>
                    element!("head", {
                        let css_tag = css_tag.clone();
                        move |el| {
                            el.append(&css_tag, lol_html::html_content::ContentType::Html);
                            Ok(())
                        }
                    }),
                    // Inject JS before </body>
                    element!("body", {
                        let js_tag = js_tag.clone();
                        move |el| {
                            el.append(&js_tag, lol_html::html_content::ContentType::Html);
                            Ok(())
                        }
                    }),
                ],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );
        let _ = rewriter.write(html.as_bytes());
        let _ = rewriter.end();
    }
    String::from_utf8(output).unwrap_or_else(|_| html.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_all_dev_attrs() {
        let html = r#"<html><head></head><body><div data-st-id="1" data-st-origin="o" data-st-bind="b" data-st-source="s" data-st-index="0" data-st-key="k" data-st-filter="f" data-st-editing="e" data-st-saving="s2" class="keep">text</div></body></html>"#;
        let out = process_html(html, "/spacetime.css", "/spacetime.js");
        assert!(!out.contains("data-st-id"));
        assert!(!out.contains("data-st-origin"));
        assert!(!out.contains("data-st-bind"));
        assert!(!out.contains("data-st-source"));
        assert!(!out.contains("data-st-index"));
        assert!(!out.contains("data-st-key"));
        assert!(!out.contains("data-st-filter"));
        assert!(!out.contains("data-st-editing"));
        assert!(!out.contains("data-st-saving"));
        assert!(out.contains("class=\"keep\""));
        assert!(out.contains(">text<"));
    }

    #[test]
    fn strips_dev_runtime_tags() {
        let html = r#"<html><head></head><body><script src="/__spacetime/dev/runtime.js"></script></body></html>"#;
        let out = process_html(html, "/spacetime.css", "/spacetime.js");
        assert!(!out.contains("__spacetime"));
    }

    #[test]
    fn strips_prod_spacetime_tags() {
        let html = r#"<html><head><link href="/__spacetime/runtime.js" rel="stylesheet"></head><body><script src="/__spacetime/runtime.js"></script></body></html>"#;
        let out = process_html(html, "/spacetime.css", "/spacetime.js");
        assert!(!out.contains("__spacetime"));
    }

    #[test]
    fn injects_provided_css_href() {
        let html = r#"<html><head></head><body></body></html>"#;
        let out = process_html(html, "/about/spacetime.css", "/about/spacetime.js");
        assert!(out.contains(r#"href="/about/spacetime.css""#));
    }

    #[test]
    fn injects_provided_js_href() {
        let html = r#"<html><head></head><body></body></html>"#;
        let out = process_html(html, "/about/spacetime.css", "/about/spacetime.js");
        assert!(out.contains(r#"src="/about/spacetime.js""#));
    }

    #[test]
    fn preserves_non_dev_content() {
        let html = r#"<html><head><title>My Site</title></head><body><h1 class="foo">Hello</h1><p>World</p></body></html>"#;
        let out = process_html(html, "/spacetime.css", "/spacetime.js");
        assert!(out.contains("<h1"));
        assert!(out.contains("class=\"foo\""));
        assert!(out.contains("Hello"));
        assert!(out.contains("World"));
        assert!(out.contains("My Site"));
    }

    #[test]
    fn idempotent_when_run_twice() {
        let html = r#"<html><head></head><body><h1>Hi</h1></body></html>"#;
        let once = process_html(html, "/spacetime.css", "/spacetime.js");
        let twice = process_html(&once, "/spacetime.css", "/spacetime.js");
        assert_eq!(once, twice);
    }

    #[test]
    fn injection_inside_head_and_body() {
        let html = r#"<html><head><title>T</title></head><body><h1>H</h1></body></html>"#;
        let out = process_html(html, "/s.css", "/s.js");
        let link_pos = out
            .find(r#"<link rel="stylesheet" href="/s.css">"#)
            .unwrap();
        let head_end = out.find("</head>").unwrap();
        let script_pos = out.find(r#"<script src="/s.js"></script>"#).unwrap();
        let body_end = out.find("</body>").unwrap();
        assert!(link_pos < head_end, "link must be before </head>");
        assert!(script_pos < body_end, "script must be before </body>");
    }
}
