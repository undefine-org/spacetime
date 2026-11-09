//! Inline marks — what the formatting bar offers, and how each one is written.
//!
//! There is no mark construct, and no vocabulary here. A mark is an ordinary
//! FORM in scope whose signature says it wraps text:
//! `@form markup --accent(&sel) { <mark class="accent">`&sel`</mark> }` —
//! exactly one content slot (`&sel`, any name), every other param a typed
//! scalar, one root element of phrasing content (it sits inside a sentence).
//! The standard ones (`--bold`, `--italic`, `--strike`, `--code`, `--link`) are
//! such forms in `stdlib/editable/marks.st`; a page's own win by name.
//!
//! How a mark is PRESENTED is inferred, the way the admin infers a field's
//! widget: its label from its name, its doc from its `///` block, its icon from
//! its initial, each param's control from the scalar table's `%widget`.
//! `@cms(--accent) { icon: "✦"; shortcut: "mod+e"; }` only overrides that —
//! display hints, nothing at runtime.
//!
//! Applying a mark to source is a splice of what its projection writes before
//! and after the slot. In literate prose a mark is written in the prose's own
//! syntax when it has one ([`crate::literate::prose_mark`], the renderer's
//! table read backwards); otherwise Markdown takes the markup and Org refuses.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::markup;
use super::outline::Site;

/// One typed param a mark asks for when applied (a link's `href`).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<String>,
    /// The control that edits it, from the scalar table (`"media"`, `"color"`).
    pub widget: String,
}

/// One inline mark.
#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    /// The element (`strong`) or the form (`--accent`).
    pub name: String,
    pub label: String,
    pub doc: String,
    /// The projection's root tag and classes: how the mark is recognised.
    pub tag: String,
    pub classes: Vec<String>,
    pub icon: String,
    pub shortcut: Option<String>,
    pub params: Vec<Param>,
    /// The projection around the slot, param holes still in place.
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

/// The standard marks: ordinary forms, read like the page's own.
const STANDARD: &str = include_str!("../../stdlib/editable/marks.st");

/// HTML's phrasing content: the roots a mark may have, so it can sit inside a
/// sentence. (A flow root — `p`, `div`, `blockquote` — takes over a block.)
const PHRASING: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "del", "dfn", "em", "i", "ins", "kbd", "mark", "q", "s",
    "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var",
];

impl Mark {
    /// The source text around a selection, with `attrs` filling the params.
    pub fn wrap(&self, lang: Lang, attrs: &BTreeMap<String, String>) -> Result<(String, String), String> {
        for p in &self.params {
            let Some(v) = attrs.get(&p.name).map(|v| v.trim()).filter(|v| !v.is_empty()) else {
                return Err(format!("{} needs {}", self.label, p.name));
            };
            // A `url` param is a link, never a script (the editable schema's rule).
            let scheme: String = v.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
            if p.ty.as_deref() == Some("url")
                && (scheme.starts_with("javascript:") || scheme.starts_with("vbscript:") || scheme.starts_with("data:text/html"))
            {
                return Err(format!("{} must be a link, not a script", p.name));
            }
        }
        if lang == Lang::Markup {
            let fill = |s: &str| {
                let mut s = s.to_string();
                for (k, v) in attrs {
                    s = s.replace(&format!("`${k}`"), &attr_escape(v.trim()));
                }
                s
            };
            return Ok((fill(&self.before), fill(&self.after)));
        }
        let org = lang == Lang::Org;
        let href = attrs.get("href").map(|h| h.trim()).unwrap_or("");
        let plain_link = self.tag == "a" && self.classes.is_empty() && self.params.len() == 1 && self.params[0].name == "href";
        let prose = if plain_link {
            Some(if org { (format!("[[{href}]["), "]]".to_string()) } else { ("[".to_string(), format!("]({href})")) })
        } else if self.classes.is_empty() && self.params.is_empty() {
            crate::literate::prose_mark(org, &self.tag)
        } else {
            None
        };
        match (prose, lang) {
            (Some(p), _) => Ok(p),
            // Markdown carries inline HTML; Org prose does not.
            (None, Lang::Markdown) => self.wrap(Lang::Markup, attrs),
            _ => Err(format!("Org prose has no way to write {}", self.label)),
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "label": self.label,
            "doc": self.doc,
            "tag": self.tag,
            "classes": self.classes,
            "icon": self.icon,
            "shortcut": self.shortcut,
            "params": self.params.iter().map(|p| json!({ "name": p.name, "type": p.ty, "widget": p.widget })).collect::<Vec<_>>(),
        })
    }
}

fn attr_escape(v: &str) -> String {
    v.replace('&', "&amp;").replace('"', "&quot;").replace('`', "&#96;").replace('<', "&lt;")
}

/// The marks a page can use: the standard forms, then every form in the
/// page's files whose signature wraps text (a page's own wins by name), each
/// with its `@cms` hints applied.
pub fn marks(site: &Site) -> Vec<Mark> {
    let mut out: Vec<Mark> = Vec::new();
    let mut hints: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let sources = std::iter::once(STANDARD).chain(site.files.iter().filter(|f| f.path.ends_with(".st")).map(|f| f.text.as_str()));
    for text in sources {
        let (forms, h) = declared(text);
        for m in forms {
            match out.iter_mut().find(|o| o.name == m.name) {
                Some(o) => *o = m,
                None => out.push(m),
            }
        }
        hints.extend(h);
    }
    for m in &mut out {
        let Some(h) = hints.get(&m.name) else { continue };
        if let Some(v) = h.get("icon") {
            m.icon = v.clone();
        }
        if let Some(v) = h.get("shortcut") {
            m.shortcut = Some(v.clone()).filter(|s| !s.is_empty() && s != "none");
        }
        if let Some(v) = h.get("label") {
            m.label = v.clone();
        }
    }
    out
}

/// The control the scalar table gives a param's type (`text` when untyped).
fn widget_for(ty: Option<&str>) -> String {
    let Some(ty) = ty else { return "text".to_string() };
    let (registry, _) = crate::compiler::cached_stdlib_registry();
    registry.scalar_types().find(|s| s.id == ty).map(|s| s.widget.clone()).unwrap_or_else(|| "text".to_string())
}

/// The forms of one file that wrap text, and the `@cms(--form)` hints in it.
pub fn declared(text: &str) -> (Vec<Mark>, BTreeMap<String, BTreeMap<String, String>>) {
    let mut forms = Vec::new();
    let mut hints = BTreeMap::new();
    if !text.contains("@form") && !text.contains("@cms") {
        return (forms, hints);
    }
    let Ok(ast) = crate::parser::parse(text) else { return (forms, hints) };
    for fm in &ast.matches {
        match fm.macro_name.as_str() {
            "cms" => {
                let Some(name) = fm.get_ident("type").filter(|t| t.starts_with("--")) else { continue };
                let props = fm.get_properties("hints").cloned().unwrap_or_default();
                let h: BTreeMap<String, String> =
                    props.into_iter().map(|p| (p.name, p.type_ref.trim().trim_matches('"').to_string())).collect();
                hints.insert(name.to_string(), h);
            }
            "form" if fm.get_ident("kind") == Some("markup") => {
                if let Some(m) = wrapping_form(fm) {
                    forms.push(m);
                }
            }
            _ => {}
        }
    }
    (forms, hints)
}

/// A markup form whose signature wraps text: one content slot, typed scalars,
/// one phrasing root holding the slot once.
fn wrapping_form(fm: &crate::syntax::FormMatch) -> Option<Mark> {
    use crate::syntax::TemplateParamKind;
    let name = fm.get_ident("name")?.to_string();
    let params = fm.get_param_list("params").cloned().unwrap_or_default();
    let mut slots = params.iter().filter(|p| p.kind == TemplateParamKind::Element);
    let slot = slots.next()?;
    if slots.next().is_some() || slot.collection {
        return None;
    }
    let body = fm.get_string("body")?.trim();
    let hole = format!("`&{}`", slot.name);
    if body.matches(&hole).count() != 1 {
        return None;
    }
    let els = markup::elements(body, &[]);
    let root = els.first()?;
    if els.iter().filter(|e| e.parent.is_none()).count() != 1 || root.span != super::Span::new(0, body.len()) {
        return None;
    }
    if !PHRASING.contains(&root.tag.as_str()) {
        return None;
    }
    let (before, after) = body.split_once(&hole)?;
    let label = {
        let words = name.trim_start_matches('-').replace('-', " ");
        let mut c = words.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    };
    let icon = label.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    Some(Mark {
        doc: fm.doc.clone().unwrap_or_default().trim().to_string(),
        tag: root.tag.clone(),
        classes: root.classes.clone(),
        icon,
        shortcut: None,
        params: params
            .iter()
            .filter(|p| p.kind != TemplateParamKind::Element)
            .map(|p| Param { name: p.name.clone(), ty: p.type_ref.clone(), widget: widget_for(p.type_ref.as_deref()) })
            .collect(),
        before: before.to_string(),
        after: after.to_string(),
        label,
        name,
    })
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

    fn site(page: &str) -> Vec<Mark> {
        marks(&Site::from_files(&[("index.st", page)]))
    }

    #[test]
    fn the_standard_marks_are_stdlib_forms() {
        let all = site("<p>hi</p>");
        let bold = all.iter().find(|m| m.name == "--bold").expect("--bold");
        assert_eq!((bold.before.as_str(), bold.after.as_str(), bold.icon.as_str()), ("<strong>", "</strong>", "B"));
        assert_eq!(bold.wrap(Lang::Markdown, &BTreeMap::new()).unwrap(), ("**".into(), "**".into()));
        assert_eq!(bold.wrap(Lang::Org, &BTreeMap::new()).unwrap(), ("*".into(), "*".into()));
        let link = all.iter().find(|m| m.name == "--link").expect("--link");
        assert_eq!(link.params[0].widget, "media", "the url scalar's own control");
        let attrs = BTreeMap::from([("href".to_string(), "https://x.y/?a=\"b\"".to_string())]);
        assert_eq!(link.wrap(Lang::Markup, &attrs).unwrap().0, "<a href=\"https://x.y/?a=&quot;b&quot;\">");
        assert_eq!(link.wrap(Lang::Markdown, &attrs).unwrap().1, "](https://x.y/?a=\"b\")");
        assert_eq!(link.wrap(Lang::Org, &attrs).unwrap().0, "[[https://x.y/?a=\"b\"][");
        assert!(link.wrap(Lang::Markup, &BTreeMap::new()).is_err());
        let script = BTreeMap::from([("href".to_string(), " Java\tScript:alert(1)".to_string())]);
        assert_eq!(link.wrap(Lang::Markup, &script).unwrap_err(), "href must be a link, not a script");
    }

    #[test]
    fn a_page_overrides_a_standard_mark_by_name() {
        let all = site("@form markup --bold(&sel) { <b>`&sel`</b> }\n");
        let bold = all.iter().find(|m| m.name == "--bold").unwrap();
        assert_eq!((bold.tag.as_str(), bold.icon.as_str()), ("b", "B"), "the page's form, the stdlib's @cms");
        assert_eq!(all.iter().filter(|m| m.name == "--bold").count(), 1);
    }

    #[test]
    fn a_form_that_wraps_text_is_a_mark_presented_by_inference_then_cms() {
        let page = "/// The pencil's accent.\n@form markup --accent(&sel) { <mark class=\"accent\">`&sel`</mark> }\n\
                    @form markup --tone(&words, $tint color = \"red\") { <span class=\"tone\" style=\"color: `$tint`\">`&words`</span> }\n\
                    @form markup --badge { <span class=\"badge\">New</span> }\n\
                    @form markup --card(&sel) { <div class=\"card\">`&sel`</div> }\n\
                    @form markup --pair(&a, &b) { <span>`&a``&b`</span> }\n\
                    @form style --pencil { color: red; }\n\
                    @cms(--tone) { icon: \"◐\"; shortcut: \"mod+shift+t\"; }\n";
        let all = site(page);
        let names: Vec<&str> = all.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["--bold", "--italic", "--strike", "--code", "--link", "--accent", "--tone"],
            "no slot, a flow root, two slots and a style are not marks"
        );
        let accent = &all[5];
        assert_eq!((accent.label.as_str(), accent.icon.as_str(), accent.doc.as_str()), ("Accent", "A", "The pencil's accent."));
        assert_eq!((accent.tag.as_str(), accent.classes.clone()), ("mark", vec!["accent".to_string()]));
        assert_eq!(accent.shortcut, None);
        assert_eq!(accent.wrap(Lang::Markdown, &BTreeMap::new()).unwrap().0, "<mark class=\"accent\">");
        assert_eq!(accent.wrap(Lang::Org, &BTreeMap::new()).unwrap_err(), "Org prose has no way to write Accent");
        let tone = &all[6];
        assert_eq!((tone.icon.as_str(), tone.shortcut.as_deref()), ("◐", Some("mod+shift+t")));
        assert_eq!(tone.params[0].widget, "color");
        let attrs = BTreeMap::from([("tint".to_string(), "#c00".to_string())]);
        assert_eq!(tone.wrap(Lang::Markup, &attrs).unwrap().0, "<span class=\"tone\" style=\"color: #c00\">");
    }

    #[test]
    fn org_writes_only_what_its_reader_reads() {
        let page = "@form markup --under(&sel) { <u>`&sel`</u> }\n";
        let u = site(page).into_iter().find(|m| m.name == "--under").unwrap();
        assert!(u.wrap(Lang::Org, &BTreeMap::new()).is_err(), "Org's reader has no underline");
        assert!(u.wrap(Lang::Markdown, &BTreeMap::new()).is_ok(), "Markdown takes it as markup");
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
