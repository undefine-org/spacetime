//! A lossless tokenizer for the HTML element literals inside a `.st` file.
//!
//! `.st` interleaves markup with CSS-shaped rules and directives, so this does
//! not parse a document: it finds element tags wherever markup can appear and
//! records, for each element, its tag, classes and the static text runs that are
//! its direct children, each with its byte span. A run containing a backtick
//! hole is dynamic (its text comes from data) and is marked so; the studio edits
//! those through the data instead.
//!
//! A `<` opens a tag only when followed by a letter, `/letter` or `!--`, which
//! rules out `<-` bindings and `a < b` comparisons. Comment regions and `@data`
//! literal regions are excluded by the caller (a string value may itself contain
//! `<em>`, which is data, not markup).

use super::{Span, normalize};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Element {
    /// Index of this element in the file's element list.
    pub index: usize,
    pub parent: Option<usize>,
    pub depth: usize,
    pub tag: String,
    pub classes: Vec<String>,
    pub id: Option<String>,
    /// From `<` to the end of the opening tag.
    pub open: Span,
    /// From `<` to the end of the closing tag (or of the open tag when void).
    pub span: Span,
    /// Direct-child text runs, in order.
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Run {
    pub span: Span,
    /// Decoded text (entities resolved), whitespace as authored.
    pub text: String,
    /// False when the run contains a backtick hole.
    pub is_static: bool,
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
];

/// Tokenize every element literal in `src`, skipping `excluded` byte ranges.
pub fn elements(src: &str, excluded: &[Span]) -> Vec<Element> {
    let b = src.as_bytes();
    let mut out: Vec<Element> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut text_start: Option<usize> = None;
    let mut i = 0usize;

    let flush_text = |out: &mut Vec<Element>, stack: &[usize], from: Option<usize>, to: usize| {
        let (Some(from), Some(&top)) = (from, stack.last()) else { return };
        if to <= from {
            return;
        }
        let raw = &src[from..to];
        if raw.trim().is_empty() {
            return;
        }
        out[top].runs.push(Run {
            span: Span::new(from, to),
            text: decode_entities(raw),
            is_static: !raw.contains('`'),
        });
    };

    while i < b.len() {
        if let Some(ex) = excluded.iter().find(|ex| ex.start <= i && i < ex.end) {
            flush_text(&mut out, &stack, text_start.take(), i);
            i = ex.end;
            continue;
        }
        if b[i] != b'<' {
            if text_start.is_none() && !stack.is_empty() {
                text_start = Some(i);
            }
            i += src[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            continue;
        }
        let next = b.get(i + 1).copied().unwrap_or(0);
        if src[i..].starts_with("<!--") {
            flush_text(&mut out, &stack, text_start.take(), i);
            i = src[i..].find("-->").map(|e| i + e + 3).unwrap_or(b.len());
            continue;
        }
        let closing = next == b'/' && b.get(i + 2).is_some_and(|c| c.is_ascii_alphabetic());
        if !(next.is_ascii_alphabetic() || closing) {
            if text_start.is_none() && !stack.is_empty() {
                text_start = Some(i);
            }
            i += 1;
            continue;
        }
        let Some(tag_end) = tag_end(src, i) else {
            i += 1;
            continue;
        };
        flush_text(&mut out, &stack, text_start.take(), i);
        let inner = &src[i + 1 + closing as usize..tag_end - 1];
        let name: String = inner
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect::<String>()
            .to_ascii_lowercase();
        if closing {
            // Pop to the matching open element (tolerating unclosed children).
            if let Some(pos) = stack.iter().rposition(|&e| out[e].tag == name) {
                for &e in &stack[pos..] {
                    out[e].span.end = tag_end;
                }
                stack.truncate(pos);
            }
        } else {
            let attrs = &inner[name.len()..];
            let classes = attr(attrs, "class")
                .map(|v| v.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default();
            let index = out.len();
            out.push(Element {
                index,
                parent: stack.last().copied(),
                depth: stack.len(),
                tag: name.clone(),
                classes,
                id: attr(attrs, "id"),
                open: Span::new(i, tag_end),
                span: Span::new(i, tag_end),
                runs: Vec::new(),
            });
            let self_closing = inner.trim_end().ends_with('/');
            if !self_closing && !VOID.contains(&name.as_str()) {
                stack.push(index);
            }
        }
        i = tag_end;
    }
    out
}

/// End (exclusive) of the tag starting at `start`, honouring quoted attribute
/// values and backtick holes so a `>` inside them does not end the tag.
fn tag_end(src: &str, start: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = start + 1;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == b'"' || c == b'\'' || c == b'`' => quote = Some(c),
            None if c == b'>' => return Some(i + 1),
            None if c == b'<' || (c == b'\n' && i - start > 2000) => return None,
            None => {}
        }
        i += 1;
    }
    None
}

/// The value of attribute `name` in an attribute string, if present and static.
fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs;
    while let Some(pos) = rest.find(name) {
        let before_ok = pos == 0 || rest.as_bytes()[pos - 1].is_ascii_whitespace();
        let after = &rest[pos + name.len()..];
        let after_trim = after.trim_start();
        if before_ok && after_trim.starts_with('=') {
            let v = after_trim[1..].trim_start();
            let q = v.chars().next()?;
            if q == '"' || q == '\'' {
                let end = v[1..].find(q)?;
                return Some(v[1..1 + end].to_string());
            }
            return None;
        }
        rest = &rest[pos + name.len()..];
    }
    None
}

/// Resolve the entities authors write in `.st` markup.
pub fn decode_entities(raw: &str) -> String {
    raw.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#96;", "`")
        .replace("&amp;", "&")
}

/// Encode plain text for a markup text run: `<` and the hole backtick are the
/// only characters `.st` markup cannot carry literally.
pub fn encode_text(text: &str) -> String {
    text.replace('<', "&lt;").replace('`', "\\`")
}

impl Element {
    /// The element's full static text (its runs and its descendants'), in
    /// document order, normalized. `None` when any part of it is dynamic.
    pub fn static_text(&self, all: &[Element]) -> Option<String> {
        let mut parts: Vec<(usize, String)> = Vec::new();
        let mut dynamic = false;
        collect(self, all, &mut parts, &mut dynamic);
        if dynamic {
            return None;
        }
        parts.sort_by_key(|(at, _)| *at);
        Some(normalize(&parts.into_iter().map(|(_, t)| t).collect::<String>()))
    }
}

fn collect(el: &Element, all: &[Element], parts: &mut Vec<(usize, String)>, dynamic: &mut bool) {
    for run in &el.runs {
        if !run.is_static {
            *dynamic = true;
        }
        parts.push((run.span.start, run.text.clone()));
    }
    for child in all.iter().filter(|c| c.parent == Some(el.index)) {
        collect(child, all, parts, dynamic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_elements_classes_and_runs() {
        let src = "x { a <- $b; }\n<h1 class=\"hero__title big\">I write <em>stories</em>.</h1>\n<p>`$t.n` done</p>";
        let els = elements(src, &[]);
        assert_eq!(els.len(), 3);
        assert_eq!(els[0].tag, "h1");
        assert_eq!(els[0].classes, vec!["hero__title", "big"]);
        assert_eq!(els[0].runs.len(), 2);
        assert_eq!(els[0].runs[0].text, "I write ");
        assert_eq!(&src[els[0].runs[0].span.start..els[0].runs[0].span.end], "I write ");
        assert_eq!(els[1].parent, Some(0));
        assert_eq!(els[0].static_text(&els).as_deref(), Some("I write stories."));
        assert!(!els[2].runs[0].is_static);
        assert_eq!(els[2].static_text(&els), None);
        assert_eq!(&src[els[0].span.start..els[0].span.end], "<h1 class=\"hero__title big\">I write <em>stories</em>.</h1>");
    }

    #[test]
    fn void_and_self_closing_tags_do_not_nest() {
        let src = "<div><img src=\"a.png\" alt=\"x > y\"><input /><p>after</p></div>";
        let els = elements(src, &[]);
        let p = els.iter().find(|e| e.tag == "p").unwrap();
        assert_eq!(p.depth, 1);
        assert_eq!(p.runs[0].text, "after");
    }

    #[test]
    fn excluded_regions_are_not_markup() {
        let src = "@data inline $x : [\"<em>no</em>\"];\n<b>yes</b>";
        let ex = [Span::new(18, 34)];
        let els = elements(src, &ex);
        assert_eq!(els.len(), 1);
        assert_eq!(els[0].tag, "b");
    }
}
