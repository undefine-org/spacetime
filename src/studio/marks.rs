//! Inline marks — what the formatting bar offers, and how each one is written.
//!
//! A mark is the editable module's own construct (`@editable-mark`, PLAN-031):
//! a template whose body is the HTML projection with one `&sel` hole. The
//! studio reads the same declarations — the standard set in
//! `stdlib/editable/builtins.st`, then the page's own, which win by name — and
//! splits each projection at `&sel` into the text written BEFORE the selection
//! and the text written AFTER it. Applying a mark to source is then a splice.
//!
//! Prose is not markup: in the prose of a literate file (`.st.md`, `.st.org`)
//! a mark is written in that language's own syntax when it has one (`**x**`,
//! `/x/`, `[[href][x]]`), so the file stays a document a person reads.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::outline::Site;

const BUILTINS: &str = include_str!("../../stdlib/editable/builtins.st");

/// One inline mark.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    pub name: String,
    /// The projection's root tag (`strong`, `a`, `mark`).
    pub tag: String,
    /// The root's classes, so `<mark class="accent">` and `<mark>` differ.
    pub classes: Vec<String>,
    pub icon: Option<String>,
    pub shortcut: Option<String>,
    /// Free params: name and declared type (`href`, `url`).
    pub params: Vec<(String, Option<String>)>,
    /// The projection around `&sel`, holes still in place.
    pub before: String,
    pub after: String,
}

/// The language a byte of a file is written in, for marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Markup,
    Markdown,
    Org,
}

impl Mark {
    /// The source text around a selection, with `attrs` filling the holes.
    pub fn wrap(&self, lang: Lang, attrs: &BTreeMap<String, String>) -> Result<(String, String), String> {
        for (p, _) in &self.params {
            if attrs.get(p).is_none_or(|v| v.trim().is_empty()) {
                return Err(format!("{} needs {}", self.name, p));
            }
        }
        if lang == Lang::Markup {
            let fill = |s: &str| {
                let mut s = s.to_string();
                for (k, v) in attrs {
                    s = s.replace(&format!("`${k}`"), &attr_escape(v));
                }
                s
            };
            return Ok((fill(&self.before), fill(&self.after)));
        }
        let href = attrs.get("href").map(String::as_str).unwrap_or("");
        let plain = self.classes.is_empty();
        let prose = match (lang, self.tag.as_str()) {
            (Lang::Markdown, "strong" | "b") if plain => Some(("**".into(), "**".into())),
            (Lang::Markdown, "em" | "i") if plain => Some(("*".into(), "*".into())),
            (Lang::Markdown, "code") if plain => Some(("`".into(), "`".into())),
            (Lang::Markdown, "s" | "del") if plain => Some(("~~".into(), "~~".into())),
            (Lang::Markdown, "a") => Some(("[".into(), format!("]({href})"))),
            (Lang::Org, "strong" | "b") if plain => Some(("*".into(), "*".into())),
            (Lang::Org, "em" | "i") if plain => Some(("/".into(), "/".into())),
            (Lang::Org, "code") if plain => Some(("~".into(), "~".into())),
            (Lang::Org, "u") if plain => Some(("_".into(), "_".into())),
            (Lang::Org, "s" | "del") if plain => Some(("+".into(), "+".into())),
            (Lang::Org, "a") => Some((format!("[[{href}]["), "]]".into())),
            _ => None,
        };
        match (prose, lang) {
            (Some(p), _) => Ok(p),
            // Markdown carries inline HTML; Org prose does not.
            (None, Lang::Markdown) => self.wrap(Lang::Markup, attrs),
            _ => Err(format!("Org prose has no way to write {}", self.name)),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "tag": self.tag,
            "classes": self.classes,
            "icon": self.icon,
            "shortcut": self.shortcut,
            "params": self.params.iter().map(|(n, t)| json!({ "name": n, "type": t })).collect::<Vec<_>>(),
        })
    }
}

fn attr_escape(v: &str) -> String {
    v.replace('&', "&amp;").replace('"', "&quot;").replace('`', "&#96;").replace('<', "&lt;")
}

/// The marks a page can use: the standard set, then the page's own.
pub fn marks(site: &Site) -> Vec<Mark> {
    let mut by_name: BTreeMap<String, Mark> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let sources = std::iter::once(BUILTINS).chain(site.files.iter().filter(|f| f.path.ends_with(".st")).map(|f| f.text.as_str()));
    for text in sources {
        for m in declared(text) {
            if !by_name.contains_key(&m.name) {
                order.push(m.name.clone());
            }
            by_name.insert(m.name.clone(), m);
        }
    }
    order.into_iter().filter_map(|n| by_name.remove(&n)).collect()
}

/// The inline marks declared in one file, read by the real parser.
pub fn declared(text: &str) -> Vec<Mark> {
    if !text.contains("@editable-mark") {
        return Vec::new();
    }
    let Ok(ast) = crate::parser::parse(text) else { return Vec::new() };
    let (schemas, _) = crate::editable::collect_schemas_from_matches(&ast.matches, &ast.scopes);
    let html: BTreeMap<&str, &str> = ast
        .scopes
        .iter()
        .filter_map(|s| s.selector.strip_prefix("@template:").map(|n| (n, s.html.as_str())))
        .collect();
    schemas
        .into_iter()
        .filter(|s| s.kind == crate::editable::EditableKind::Mark)
        .filter_map(|s| {
            let body = html.get(s.name.as_str())?.trim();
            let (before, after) = body.split_once("`&sel`")?;
            let params = ast
                .matches
                .iter()
                .find(|m| m.macro_name == "editable-mark" && m.get_ident("name") == Some(s.name.as_str()))
                .and_then(|m| m.get_param_list("params").cloned())
                .unwrap_or_default()
                .into_iter()
                .filter(|p| p.name != "sel")
                .map(|p| (p.name.clone(), p.type_ref.clone()))
                .collect();
            Some(Mark {
                classes: root_classes(before),
                name: s.name,
                tag: s.tag,
                icon: s.icon,
                shortcut: s.shortcut,
                params,
                before: before.to_string(),
                after: after.to_string(),
            })
        })
        .collect()
}

fn root_classes(open: &str) -> Vec<String> {
    let Some(at) = open.find("class=\"") else { return Vec::new() };
    let rest = &open[at + 7..];
    let value = &rest[..rest.find('"').unwrap_or(rest.len())];
    if value.contains('`') {
        return Vec::new();
    }
    value.split_whitespace().map(str::to_string).collect()
}

/// The language the byte `at` of `path` is written in: markup in a `.st` file
/// and in a literate file's code blocks, prose elsewhere in a literate file.
pub fn lang_at(path: &str, text: &str, at: usize) -> Lang {
    let (prose, open, close): (Lang, &str, &str) = if path.ends_with(".st.md") {
        (Lang::Markdown, "```", "```")
    } else if path.ends_with(".st.org") {
        (Lang::Org, "#+begin_src", "#+end_src")
    } else {
        return Lang::Markup;
    };
    let mut in_code = false;
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        if pos + line.len() > at {
            return if in_code { Lang::Markup } else { prose };
        }
        let t = line.trim_start().to_ascii_lowercase();
        if !in_code && t.starts_with(open) {
            in_code = true;
        } else if in_code && t.starts_with(close) {
            in_code = false;
        }
        pos += line.len();
    }
    prose
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_split_at_the_selection() {
        let all = marks(&Site::from_files(&[("index.st", "<p>hi</p>")]));
        let bold = all.iter().find(|m| m.name == "bold").expect("bold");
        assert_eq!((bold.before.as_str(), bold.after.as_str()), ("<strong>", "</strong>"));
        assert_eq!(bold.icon.as_deref(), Some("B"));
        let link = all.iter().find(|m| m.name == "link").expect("link");
        assert_eq!(link.params, vec![("href".to_string(), Some("url".to_string()))]);
        let attrs = BTreeMap::from([("href".to_string(), "https://x.y/?a=\"b\"".to_string())]);
        assert_eq!(link.wrap(Lang::Markup, &attrs).unwrap().0, "<a href=\"https://x.y/?a=&quot;b&quot;\">");
        assert_eq!(link.wrap(Lang::Markdown, &attrs).unwrap().1, "](https://x.y/?a=\"b\")");
        assert!(link.wrap(Lang::Markup, &BTreeMap::new()).is_err());
    }

    #[test]
    fn a_page_declares_its_own_marks_and_overrides_by_name() {
        let page = "@editable-mark &accent(&sel) {\n  shortcut: \"mod+e\";\n  icon: \"✦\";\n  <mark class=\"accent\">`&sel`</mark>\n}\n\
                    @editable-mark &bold(&sel) { <b>`&sel`</b> }\n";
        let all = marks(&Site::from_files(&[("index.st", page)]));
        let accent = all.iter().find(|m| m.name == "accent").expect("accent");
        assert_eq!(accent.icon.as_deref(), Some("✦"));
        assert_eq!(accent.shortcut.as_deref(), Some("mod+e"));
        assert_eq!(accent.classes, vec!["accent"]);
        assert_eq!(accent.before, "<mark class=\"accent\">");
        assert_eq!(accent.wrap(Lang::Org, &BTreeMap::new()).unwrap_err(), "Org prose has no way to write accent");
        assert_eq!(accent.wrap(Lang::Markdown, &BTreeMap::new()).unwrap().0, "<mark class=\"accent\">");
        assert_eq!(all.iter().find(|m| m.name == "bold").unwrap().tag, "b");
    }

    #[test]
    fn literate_files_are_prose_outside_their_code() {
        let md = "Intro *x*.\n```st\n<p>code</p>\n```\nMore prose.\n";
        assert_eq!(lang_at("a.st.md", md, 2), Lang::Markdown);
        assert_eq!(lang_at("a.st.md", md, md.find("code").unwrap()), Lang::Markup);
        assert_eq!(lang_at("a.st.md", md, md.find("More").unwrap()), Lang::Markdown);
        let org = "Prose.\n#+BEGIN_SRC st\n<p>x</p>\n#+END_SRC\n";
        assert_eq!(lang_at("a.st.org", org, org.find("<p>").unwrap()), Lang::Markup);
        assert_eq!(lang_at("a.st.org", org, 0), Lang::Org);
        assert_eq!(lang_at("a.st", "anything", 0), Lang::Markup);
    }
}
