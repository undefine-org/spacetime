//! Markup forms used as ELEMENTS in text (PLAN-157 D6):
//!
//! ```st
//! @form markup --accent(&sel) { <mark class="accent">`&sel`</mark> }
//! <p>I write <--accent>software</--accent> the way I write stories.</p>
//! <p><--link href="/who">Me</--link></p>
//! ```
//!
//! The text NAMES the form, the way `--role-shout;` names a style form in a
//! scope: the source keeps the name, the compiler expands it. A form element
//! becomes the form's projection, its slot (its one element param) filled with
//! the element's content and its params with the element's attributes (or
//! their defaults). `<--` can never open an HTML element — a tag name starts
//! with a letter — so the spelling is unambiguous, and it is never served.
//!
//! The expansion is TEXTUAL and representation-agnostic: it runs on an HTML
//! skeleton (holes as `\u{E000}n\u{E001}` markers) and on raw markup (holes as
//! backticks) alike, moving the content and attribute values verbatim. That
//! is why a form used as an element may hold only its params and its slot: a
//! hole of its own (`` `$brand.x` ``) would have no place in a skeleton's hole
//! list. Nested form elements expand inside out; a form whose projection uses
//! another form element expands that too, and a cycle is an error.

use std::collections::{BTreeMap, HashMap};

use crate::syntax::{TemplateParamDef, TemplateParamKind};

/// A declared form, as an element can use it.
#[derive(Debug, Clone)]
pub struct ElementForm {
    pub kind: String,
    pub params: Vec<TemplateParamDef>,
    pub body: String,
}

/// One problem with a form element; the caller turns it into a diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum FormElementError {
    /// No form by this name is declared (E0947).
    Unknown(String),
    /// The form exists but is not a markup form (E0959).
    Kind { name: String, kind: String },
    /// An attribute the form has no param for, a required param left out,
    /// content for a form without a slot, a hole the element cannot carry,
    /// an unclosed element or a cycle (E0960).
    Use { name: String, message: String },
}

impl FormElementError {
    pub fn message(&self) -> String {
        match self {
            FormElementError::Unknown(n) => format!("unknown form `<{n}>`: no `@form markup {n}` is declared"),
            FormElementError::Kind { name, kind } => {
                format!("`<{name}>` names a {kind} form; only a markup form can be used as an element")
            }
            FormElementError::Use { name, message } => format!("`<{name}>`: {message}"),
        }
    }
}

/// True when `text` may hold a form element (the cheap gate).
pub fn mentions_form_element(text: &str) -> bool {
    text.contains("<--")
}

/// Expand every form element in `text`. `None` when there was none.
pub fn expand(text: &str, forms: &HashMap<String, ElementForm>, errors: &mut Vec<FormElementError>) -> Option<String> {
    expand_inner(text, forms, errors, &mut Vec::new(), false)
}

/// Retain each composed form's identity on its projected roots. These inert
/// attributes let tools select the real element without guessing its anatomy.
pub fn expand_identified(text: &str, forms: &HashMap<String, ElementForm>, errors: &mut Vec<FormElementError>) -> Option<String> {
    expand_inner(text, forms, errors, &mut Vec::new(), true)
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// The form name of an element opening at `at` (`<--name`), and where it ends.
fn open_name(text: &str, at: usize) -> Option<(&str, usize)> {
    let rest = text.get(at..)?;
    if !rest.starts_with("<--") {
        return None;
    }
    let len = rest[1..].find(|c: char| !is_name_char(c)).unwrap_or(rest.len() - 1);
    let name = &rest[1..1 + len];
    name[2..].starts_with(|c: char| c.is_ascii_alphabetic()).then_some((name, at + 1 + len))
}

/// The end of a tag whose name ends at `from`: past its `>`, skipping quoted
/// values and backtick holes. Returns (attributes text, end, self-closing).
fn tag_end(text: &str, from: usize) -> Option<(&str, usize, bool)> {
    let mut quote: Option<char> = None;
    for (i, c) in text[from..].char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(c),
            (None, '>') => {
                let attrs = text[from..from + i].trim_end();
                let self_closing = attrs.ends_with('/');
                let attrs = attrs.trim_end_matches('/').trim();
                return Some((attrs, from + i + 1, self_closing));
            }
            _ => {}
        }
    }
    None
}

/// `name="v"`, `name='v'`, `name=v`, `name` — values verbatim.
fn attributes(raw: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let mut rest = raw.trim();
    while !rest.is_empty() {
        let name_len = rest.find(|c: char| c == '=' || c.is_whitespace()).unwrap_or(rest.len());
        let name = &rest[..name_len];
        if name.is_empty() {
            return Err(format!("cannot read the attributes `{raw}`"));
        }
        rest = rest[name_len..].trim_start();
        let value = if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            match after.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let end = after[1..].find(q).ok_or_else(|| format!("unclosed value for `{name}`"))?;
                    rest = &after[1 + end + 1..];
                    after[1..1 + end].to_string()
                }
                _ => {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    rest = &after[end..];
                    after[..end].to_string()
                }
            }
        } else {
            "true".to_string()
        };
        out.insert(name.to_string(), value);
        rest = rest.trim_start();
    }
    Ok(out)
}

/// Where the element opened at `open_end` closes: (close start, close end),
/// counting nested elements of the same name.
fn close_of(text: &str, name: &str, open_end: usize) -> Option<(usize, usize)> {
    let close = format!("</{name}>");
    let mut depth = 1;
    let mut i = open_end;
    loop {
        let next_close = text[i..].find(&close).map(|p| i + p)?;
        // a nested opening of the same name, before that close
        let mut j = i;
        while let Some(p) = text[j..next_close].find("<--").map(|p| j + p) {
            if let Some((n, after)) = open_name(text, p) {
                if n == name {
                    if let Some((_, end, self_closing)) = tag_end(text, after) {
                        if !self_closing {
                            depth += 1;
                        }
                        j = end;
                        continue;
                    }
                }
            }
            j = p + 3;
        }
        depth -= 1;
        if depth == 0 {
            return Some((next_close, next_close + close.len()));
        }
        i = next_close + close.len();
    }
}

/// Every backtick hole in a projection body.
fn holes(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(a) = rest.find('`') {
        let Some(b) = rest[a + 1..].find('`') else { break };
        out.push(&rest[a + 1..a + 1 + b]);
        rest = &rest[a + 1 + b + 1..];
    }
    out
}

const SLOT: &str = "\u{E002}slot\u{E003}";

fn expand_inner(
    text: &str,
    forms: &HashMap<String, ElementForm>,
    errors: &mut Vec<FormElementError>,
    stack: &mut Vec<String>,
    identify: bool,
) -> Option<String> {
    if !mentions_form_element(text) {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut changed = false;
    while let Some(p) = text[i..].find("<--").map(|p| i + p) {
        let Some((name, name_end)) = open_name(text, p) else {
            out.push_str(&text[i..p + 3]);
            i = p + 3;
            continue;
        };
        let use_err = |message: String| FormElementError::Use { name: name.to_string(), message };
        let Some((attrs_raw, open_end, self_closing)) = tag_end(text, name_end) else {
            errors.push(use_err("the tag is never closed with `>`".into()));
            break;
        };
        let (inner, end) = if self_closing {
            ("", open_end)
        } else {
            match close_of(text, name, open_end) {
                Some((cs, ce)) => (&text[open_end..cs], ce),
                None => {
                    errors.push(use_err(format!("no `</{name}>` closes it")));
                    out.push_str(&text[i..open_end]);
                    i = open_end;
                    continue;
                }
            }
        };
        out.push_str(&text[i..p]);
        i = end;
        match element(name, attrs_raw, inner, forms, errors, stack, identify) {
            Some(expanded) => {
                out.push_str(&expanded);
                changed = true;
            }
            // left as written: the diagnostic says why
            None => out.push_str(&text[p..end]),
        }
    }
    out.push_str(&text[i..]);
    changed.then_some(out)
}

fn element(
    name: &str,
    attrs_raw: &str,
    inner: &str,
    forms: &HashMap<String, ElementForm>,
    errors: &mut Vec<FormElementError>,
    stack: &mut Vec<String>,
    identify: bool,
) -> Option<String> {
    let use_err = |message: String| FormElementError::Use { name: name.to_string(), message };
    let Some(form) = forms.get(name) else {
        errors.push(FormElementError::Unknown(name.to_string()));
        return None;
    };
    if form.kind != "markup" {
        errors.push(FormElementError::Kind { name: name.to_string(), kind: form.kind.clone() });
        return None;
    }
    if stack.iter().any(|s| s == name) {
        errors.push(use_err(format!("it uses itself ({} -> {name})", stack.join(" -> "))));
        return None;
    }
    let mut attrs = match attributes(attrs_raw) {
        Ok(a) => a,
        Err(e) => {
            errors.push(use_err(e));
            return None;
        }
    };
    let slot = form.params.iter().find(|p| p.kind == TemplateParamKind::Element);
    if slot.is_none() && !inner.trim().is_empty() {
        errors.push(use_err("it has no slot for content; write it as `<".to_string() + name + " />`"));
        return None;
    }
    // Every hole of the projection must be the slot or a param.
    let body = form.body.trim();
    for h in holes(body) {
        let h = h.trim();
        let is_slot = slot.is_some_and(|s| h == format!("&{}", s.name));
        let is_param = form.params.iter().any(|p| p.kind == TemplateParamKind::Binding && h == format!("${}", p.name));
        if !is_slot && !is_param {
            errors.push(use_err(format!(
                "its projection holds `{h}`; a form used as an element can hold only its params and its slot"
            )));
            return None;
        }
    }
    let mut projected = body.to_string();
    for p in form.params.iter().filter(|p| p.kind == TemplateParamKind::Binding) {
        let value = match attrs.remove(&p.name) {
            Some(v) => v,
            None => match &p.default {
                Some(d) => d.trim().trim_matches('"').trim_matches('\'').to_string(),
                None => {
                    errors.push(use_err(format!("`{}` is required", p.name)));
                    return None;
                }
            },
        };
        projected = projected.replace(&format!("`${}`", p.name), &value);
    }
    if let Some(extra) = attrs.keys().next() {
        let declared: Vec<String> = form.params.iter().filter(|p| p.kind == TemplateParamKind::Binding).map(|p| p.name.clone()).collect();
        errors.push(use_err(format!(
            "unknown argument `{extra}`; declared parameters: {}",
            if declared.is_empty() { "none".to_string() } else { declared.join(", ") }
        )));
        return None;
    }
    if let Some(s) = slot {
        projected = projected.replace(&format!("`&{}`", s.name), SLOT);
    }
    // A projection may use other form elements; the content is already expanded.
    stack.push(name.to_string());
    let projected = expand_inner(&projected, forms, errors, stack, identify).unwrap_or(projected);
    stack.pop();
    let content = expand_inner(inner, forms, errors, stack, identify).unwrap_or_else(|| inner.to_string());
    let mut projected = projected.replace(SLOT, &content);
    if identify {
        let roots = crate::html::source::elements(&projected, &[]);
        for root in roots.iter().filter(|e| e.parent.is_none()).rev() {
            let at = root.open.start + 1 + root.tag.len();
            projected.insert_str(at, &format!(" data-st-form-{}=\"\"", name.trim_start_matches("--")));
        }
    }
    Some(projected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(name: &str, kind: TemplateParamKind, default: Option<&str>) -> TemplateParamDef {
        TemplateParamDef {
            name: name.into(),
            kind,
            optional: default.is_some(),
            type_ref: None,
            default: default.map(str::to_string),
            collection: false,
        }
    }

    fn forms() -> HashMap<String, ElementForm> {
        use TemplateParamKind::*;
        let markup = |params: Vec<TemplateParamDef>, body: &str| ElementForm { kind: "markup".into(), params, body: body.into() };
        HashMap::from([
            ("--accent".to_string(), markup(vec![param("sel", Element, None)], "<mark class=\"accent\">`&sel`</mark>")),
            (
                "--link".to_string(),
                markup(vec![param("sel", Element, None), param("href", Binding, None)], "<a href=\"`$href`\">`&sel`</a>"),
            ),
            (
                "--tone".to_string(),
                markup(vec![param("words", Element, None), param("tint", Binding, Some("\"red\""))], "<span style=\"color: `$tint`\">`&words`</span>"),
            ),
            ("--badge".to_string(), markup(vec![], "<span class=\"badge\">New</span>")),
            ("--quote".to_string(), markup(vec![param("sel", Element, None)], "<q><--accent>`&sel`</--accent></q>")),
            ("--loop".to_string(), markup(vec![param("sel", Element, None)], "<b><--loop>`&sel`</--loop></b>")),
            ("--brand".to_string(), markup(vec![param("sel", Element, None)], "<b style=\"color: `$brand.ink`\">`&sel`</b>")),
            ("--role-shout".to_string(), ElementForm { kind: "style".into(), params: vec![], body: "font-size: 2rem;".into() }),
        ])
    }

    fn run(text: &str) -> (String, Vec<FormElementError>) {
        let mut errors = Vec::new();
        let out = expand(text, &forms(), &mut errors).unwrap_or_else(|| text.to_string());
        (out, errors)
    }

    #[test]
    fn an_element_becomes_its_projection() {
        let (out, errors) = run("<p>I write <--accent>software</--accent> the way.</p>");
        assert_eq!(out, "<p>I write <mark class=\"accent\">software</mark> the way.</p>");
        assert!(errors.is_empty());
    }

    #[test]
    fn attributes_fill_params_and_defaults_the_rest() {
        assert_eq!(run("<--link href=\"/who\">Me</--link>").0, "<a href=\"/who\">Me</a>");
        assert_eq!(run("<--tone>hot</--tone>").0, "<span style=\"color: red\">hot</span>");
        assert_eq!(run("<--tone tint='#c00'>hot</--tone>").0, "<span style=\"color: #c00\">hot</span>");
        assert_eq!(run("a <--badge /> b").0, "a <span class=\"badge\">New</span> b");
    }

    #[test]
    fn holes_and_markers_travel_verbatim() {
        // raw markup (backticks) and a skeleton (hole markers) alike
        assert_eq!(run("<--link href=\"`$url`\">`$name`</--link>").0, "<a href=\"`$url`\">`$name`</a>");
        assert_eq!(run("<--accent>\u{E000}0\u{E001}</--accent>").0, "<mark class=\"accent\">\u{E000}0\u{E001}</mark>");
    }

    #[test]
    fn elements_nest_and_forms_compose() {
        let (out, _) = run("<--link href=\"/x\"><--accent>a <--accent>b</--accent></--accent></--link>");
        assert_eq!(out, "<a href=\"/x\"><mark class=\"accent\">a <mark class=\"accent\">b</mark></mark></a>");
        assert_eq!(run("<--quote>hi</--quote>").0, "<q><mark class=\"accent\">hi</mark></q>");
    }

    #[test]
    fn misuse_is_reported_and_left_as_written() {
        let cases: &[(&str, &str)] = &[
            ("<--nope>x</--nope>", "unknown form `<--nope>`"),
            ("<--role-shout>x</--role-shout>", "names a style form"),
            ("<--link>x</--link>", "`href` is required"),
            ("<--accent size=\"2\">x</--accent>", "unknown argument `size`"),
            ("<--badge>x</--badge>", "no slot for content"),
            ("<--brand>x</--brand>", "can hold only its params and its slot"),
            ("<--accent>x", "no `</--accent>` closes it"),
            ("<--loop>x</--loop>", "it uses itself"),
        ];
        for (text, want) in cases {
            let (out, errors) = run(text);
            assert!(errors.iter().any(|e| e.message().contains(want)), "{text}: {errors:?}");
            if !text.starts_with("<--loop") {
                assert_eq!(&out, text, "left as written");
            }
        }
    }

    #[test]
    fn plain_text_and_comments_are_untouched() {
        assert_eq!(expand("a <- b, x<--3, <!-- c -->", &forms(), &mut Vec::new()), None);
    }
    #[test]
    fn composed_forms_keep_identity_without_wrappers_or_changed_content() {
        let mut errors = Vec::new();
        let out = expand_identified("<--quote>hello</--quote>", &forms(), &mut errors).unwrap();
        assert!(out.contains("<q data-st-form-quote=\"\""), "{out}");
        assert!(out.contains("<mark data-st-form-accent=\"\""), "{out}");
        assert!(out.contains(">hello</mark></q>"), "{out}");
        assert!(errors.is_empty());
    }
}
