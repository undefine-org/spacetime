//! Studio edits: pure `site in -> changed files out` functions.
//!
//! Every edit is addressed the way the browser can honestly address it:
//!
//! - **text** — "this text, which used to read `old`, now reads `new`", plus
//!   hints (the list, row and field it was rendered from; the element it sits
//!   in). The source is searched for every place that could produce `old`:
//!   a string in a list, a static run of markup, and as a last resort the exact
//!   characters anywhere in the page's files. The hints narrow the candidates;
//!   the edit is applied only when exactly ONE remains.
//! - **field / move / duplicate / delete** — a list by binding and a row by index.
//! - **decl** — a style declaration by span, guarded by its current text.
//! - **mark / unmark** — "wrap these characters of this text in a mark", or
//!   "take this marked text out of its mark": the text is placed like a text
//!   edit, then the mark is written in the language of that place ([`marks`]).
//!
//! Nothing here reads or writes the disk. [`apply`] returns the full new text
//! of each changed file; the caller writes them.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::literal::{self, Lit};
use super::markup;
use super::marks::{self, Lang, Mark};
use super::outline::{List, Site, comment_spans};
use super::{Span, normalize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Op {
    Text {
        old: String,
        new: String,
        #[serde(default)]
        hint: Hint,
    },
    Field {
        binding: String,
        index: usize,
        field: String,
        value: String,
    },
    Move {
        binding: String,
        from: usize,
        to: usize,
    },
    Duplicate {
        binding: String,
        index: usize,
    },
    Delete {
        binding: String,
        index: usize,
    },
    Decl {
        file: String,
        start: usize,
        end: usize,
        expect: String,
        value: String,
    },
    /// Replace exactly the bytes `start..end`, which must still read `expect`
    /// (PLAN-155: a clip's `at 2s`, a keyframe stop, a drum cell — the words
    /// `score.json` hands out).
    Span {
        file: String,
        start: usize,
        end: usize,
        expect: String,
        value: String,
    },
    /// Insert `text` at byte `at`, where the source must still read `before`
    /// just ahead of it (a param added to a part, a stop to a chain).
    Insert {
        file: String,
        at: usize,
        before: String,
        text: String,
    },
    /// Wrap `start..end` (UTF-16 offsets, as the browser counts) of the text
    /// that reads `old` in the mark `mark`, its params filled from `attrs`.
    Mark {
        old: String,
        start: usize,
        end: usize,
        mark: String,
        #[serde(default)]
        attrs: BTreeMap<String, String>,
        #[serde(default)]
        hint: Hint,
    },
    /// Take the text `text`, written in the mark `mark`, out of it. `hint`
    /// describes the element the mark sits in.
    Unmark {
        text: String,
        mark: String,
        #[serde(default)]
        hint: Hint,
    },
    /// Replace a whole file (the code pane), guarded by the hash of the text
    /// the edit was made on ([`source_hash`]).
    Source {
        file: String,
        base: String,
        text: String,
    },
}

/// The guard a whole-file edit carries: FNV-1a of the text, in hex.
pub fn source_hash(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// What the browser knows about where a text was shown.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Hint {
    /// The list the text was rendered from, when the page could tell.
    #[serde(default)]
    pub binding: Option<String>,
    #[serde(default)]
    pub index: Option<usize>,
    #[serde(default)]
    pub field: Option<String>,
    /// The element the text node sits in.
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub classes: Vec<String>,
    /// That element's whole text, normalized.
    #[serde(default)]
    pub element_text: Option<String>,
}

/// One place in the source that could produce a text.
#[derive(Debug, Clone, PartialEq)]
enum Place {
    Data { file: usize, span: Span, binding: String, index: Option<usize>, field: Option<String> },
    Markup { file: usize, span: Span, tag: String, classes: Vec<String>, element_text: Option<String> },
    Raw { file: usize, span: Span },
}

/// Changed files: path -> new full text.
pub type Changes = BTreeMap<String, String>;

pub fn apply(site: &Site, op: &Op) -> Result<Changes, String> {
    match op {
        Op::Text { old, new, hint } => text(site, old, new, hint),
        Op::Field { binding, index, field, value } => set_field(site, binding, *index, field, value),
        Op::Move { binding, from, to } => move_row(site, binding, *from, *to),
        Op::Duplicate { binding, index } => duplicate_row(site, binding, *index),
        Op::Delete { binding, index } => delete_row(site, binding, *index),
        Op::Decl { file, start, end, expect, value } => decl(site, file, *start, *end, expect, value),
        Op::Span { file, start, end, expect, value } => {
            let fi = file_index(site, file)?;
            let current = site.files[fi].text.get(*start..*end).ok_or("the span is outside the file")?;
            if current != expect {
                return Err(STALE.to_string());
            }
            Ok(splice(site, fi, Span::new(*start, *end), value))
        }
        Op::Insert { file, at, before, text } => {
            let fi = file_index(site, file)?;
            let src = &site.files[fi].text;
            let ahead = at.checked_sub(before.len()).and_then(|a| src.get(a..*at));
            if ahead != Some(before.as_str()) {
                return Err(STALE.to_string());
            }
            Ok(splice(site, fi, Span::new(*at, *at), text))
        }
        Op::Mark { old, start, end, mark, attrs, hint } => wrap(site, old, *start, *end, mark, attrs, hint),
        Op::Unmark { text, mark, hint } => unwrap(site, text, mark, hint),
        Op::Source { file, base, text } => {
            let fi = file_index(site, file)?;
            if source_hash(&site.files[fi].text) != *base {
                return Err(STALE.to_string());
            }
            let mut changes = Changes::new();
            changes.insert(site.files[fi].path.clone(), text.clone());
            Ok(changes)
        }
    }
}

const STALE: &str = "the source changed under this edit; reload and try again";

fn file_index(site: &Site, file: &str) -> Result<usize, String> {
    site.files.iter().position(|f| f.path == file).ok_or_else(|| format!("{file} is not part of this page"))
}

// ---- text ------------------------------------------------------------------

fn text(site: &Site, old: &str, new: &str, hint: &Hint) -> Result<Changes, String> {
    if normalize(new) == normalize(old) && !normalize(old).is_empty() {
        return Ok(Changes::new());
    }
    let chosen = locate(site, old, hint)?;
    replace_place(site, &chosen, new)
}

/// The one place in the source that produces the text `old`.
fn locate(site: &Site, old: &str, hint: &Hint) -> Result<Place, String> {
    let want = normalize(old);
    if want.is_empty() {
        return Err("nothing to edit: the old text is empty".to_string());
    }
    let mut places = data_places(site, &want);
    places.extend(markup_places(site, &want));

    match places.len() {
        1 => Ok(places.remove(0)),
        0 => raw_place(site, old.trim()),
        _ => {
            let narrowed: Vec<Place> = places.iter().filter(|p| fits(p, hint)).cloned().collect();
            match narrowed.len() {
                1 => Ok(narrowed.into_iter().next().unwrap()),
                0 => Err(format!("\"{}\" appears in {} places and none matches where it was shown", short(&want), places.len())),
                n => Err(format!("\"{}\" appears in {n} places; edit it from the list or the source", short(&want))),
            }
        }
    }
}

fn data_places(site: &Site, want: &str) -> Vec<Place> {
    let mut out = Vec::new();
    for list in site.lists() {
        let Lit::Array { items, .. } = &list.value else {
            if let Lit::Str { span, value } = &list.value {
                if normalize(value) == want {
                    out.push(Place::Data { file: list.file, span: *span, binding: list.binding.clone(), index: None, field: None });
                }
            }
            continue;
        };
        for (index, row) in items.iter().enumerate() {
            collect_strings(row, &mut |span, value, field| {
                if normalize(value) == want {
                    out.push(Place::Data {
                        file: list.file,
                        span,
                        binding: list.binding.clone(),
                        index: Some(index),
                        field: field.map(str::to_string),
                    });
                }
            }, None);
        }
    }
    out
}

fn collect_strings<'a>(lit: &'a Lit, f: &mut dyn FnMut(Span, &str, Option<&'a str>), field: Option<&'a str>) {
    match lit {
        Lit::Str { span, value } => f(*span, value, field),
        Lit::Array { items, .. } => items.iter().for_each(|i| collect_strings(i, f, field)),
        Lit::Object { entries, .. } => {
            for e in entries {
                collect_strings(&e.value, f, Some(field.unwrap_or(e.key.as_str())));
            }
        }
        Lit::Other { .. } => {}
    }
}

fn markup_places(site: &Site, want: &str) -> Vec<Place> {
    let mut out = Vec::new();
    for fi in 0..site.files.len() {
        let els = site.elements(fi);
        for el in &els {
            for run in el.runs.iter().filter(|r| r.is_static) {
                if normalize(&run.text) == want {
                    out.push(Place::Markup {
                        file: fi,
                        span: run.span,
                        tag: el.tag.clone(),
                        classes: el.classes.clone(),
                        element_text: el.static_text(&els),
                    });
                }
            }
        }
    }
    out
}

/// The last resort: the exact characters, once, outside comments, anywhere in
/// the page's files (literate prose lands here).
fn raw_place(site: &Site, old: &str) -> Result<Place, String> {
    let mut found = Vec::new();
    for (fi, file) in site.files.iter().enumerate() {
        let comments = comment_spans(&file.text);
        let mut from = 0;
        while let Some(pos) = file.text[from..].find(old) {
            let at = from + pos;
            from = at + old.len().max(1);
            if !comments.iter().any(|c| c.start <= at && at < c.end) {
                found.push(Place::Raw { file: fi, span: Span::new(at, at + old.len()) });
            }
        }
    }
    match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(format!("\"{}\" is not in the source: it is computed, not written", short(old))),
        n => Err(format!("\"{}\" appears {n} times in the source; edit it there", short(old))),
    }
}

fn fits(place: &Place, hint: &Hint) -> bool {
    match place {
        Place::Data { binding, index, field, .. } => {
            hint.binding.as_ref().is_none_or(|b| b == binding)
                && hint.index.is_none_or(|i| Some(i) == *index)
                && hint.field.as_ref().is_none_or(|f| Some(f) == field.as_ref())
                && (hint.binding.is_some() || hint.index.is_some() || hint.field.is_some())
        }
        Place::Markup { tag, classes, element_text, .. } => {
            hint.binding.is_none()
                && hint.tag.as_ref().is_none_or(|t| t == tag)
                && (hint.classes.is_empty() || hint.classes.iter().any(|c| classes.contains(c)))
                && hint
                    .element_text
                    .as_ref()
                    .is_none_or(|t| element_text.as_ref().is_none_or(|e| normalize(e) == normalize(t)))
        }
        Place::Raw { .. } => false,
    }
}

fn replace_place(site: &Site, place: &Place, new: &str) -> Result<Changes, String> {
    let (file, span, replacement) = match place {
        Place::Data { file, span, .. } => (*file, *span, literal::encode_str(new)),
        Place::Markup { file, span, .. } => {
            let raw = &site.files[*file].text[span.start..span.end];
            let lead = &raw[..raw.len() - raw.trim_start().len()];
            let trail = &raw[raw.trim_end().len()..];
            (*file, *span, format!("{lead}{}{trail}", markup::encode_text(new.trim())))
        }
        Place::Raw { file, span } => (*file, *span, new.trim().to_string()),
    };
    Ok(splice(site, file, span, &replacement))
}

// ---- marks -------------------------------------------------------------------

fn find_mark(site: &Site, name: &str) -> Result<Mark, String> {
    marks::marks(site).into_iter().find(|m| m.name == name).ok_or_else(|| format!("no mark named {name}"))
}

/// The byte offset in `s` of the UTF-16 offset `at`.
fn byte_at(s: &str, at: usize) -> usize {
    let mut units = 0;
    for (i, c) in s.char_indices() {
        if units >= at {
            return i;
        }
        units += c.len_utf16();
    }
    s.len()
}

fn wrap(site: &Site, old: &str, start: usize, end: usize, name: &str, attrs: &BTreeMap<String, String>, hint: &Hint) -> Result<Changes, String> {
    let mark = find_mark(site, name)?;
    // The selection, without the spaces a double-click takes along.
    let (mut s, mut e) = (byte_at(old, start.min(end)), byte_at(old, start.max(end)));
    let picked = &old[s..e];
    s += picked.len() - picked.trim_start().len();
    e -= picked.len() - picked.trim_end().len();
    if s >= e {
        return Err("select some text first".to_string());
    }
    let (file, from, to, lang) = match locate(site, old, hint)? {
        Place::Data { binding, .. } => {
            return Err(format!("this text is a plain string in ${binding}; marks go on markup and prose"));
        }
        Place::Markup { file, span, .. } => {
            let src = markup::collapse(markup::raw_units(&site.files[file].text[span.start..span.end]));
            let shown = markup::collapse(old.char_indices().map(|(i, c)| (c, i, i + c.len_utf8())).collect());
            if src.len() != shown.len() {
                return Err("the text on the page no longer matches its source; reload".to_string());
            }
            let i = shown.iter().take_while(|u| u.1 < s).count();
            let j = shown.iter().take_while(|u| u.1 < e).count();
            (file, span.start + src[i].1, span.start + src[j - 1].2, Lang::Markup)
        }
        Place::Raw { file, span } => {
            let lead = old.len() - old.trim_start().len();
            let from = span.start + s.saturating_sub(lead);
            let to = (span.start + e.saturating_sub(lead)).min(span.end);
            let text = &site.files[file].text;
            (file, from, to, marks::lang_at(&site.files[file].path, text, from))
        }
    };
    let (open, close) = mark.wrap(lang, attrs)?;
    let text = &site.files[file].text;
    let replacement = format!("{open}{}{close}", &text[from..to]);
    Ok(splice(site, file, Span::new(from, to), &replacement))
}

fn unwrap(site: &Site, shown: &str, name: &str, hint: &Hint) -> Result<Changes, String> {
    let mark = find_mark(site, name)?;
    let want = normalize(shown);
    if want.is_empty() {
        return Err("nothing to unmark".to_string());
    }
    // (file, whole span, inner span)
    let mut found: Vec<(usize, Span, Span)> = Vec::new();
    let mut fitting: Vec<(usize, Span, Span)> = Vec::new();
    for fi in 0..site.files.len() {
        let els = site.elements(fi);
        for el in &els {
            if el.tag != mark.tag || !mark.classes.iter().all(|c| el.classes.contains(c)) {
                continue;
            }
            if el.static_text(&els).as_deref() != Some(want.as_str()) || el.span == el.open {
                continue;
            }
            let text = &site.files[fi].text;
            let close = text[..el.span.end].rfind("</").filter(|c| *c >= el.open.end).unwrap_or(el.open.end);
            let hit = (fi, el.span, Span::new(el.open.end, close));
            let parent = el.parent.map(|p| &els[p]);
            let fits = parent.is_some_and(|p| {
                hint.tag.as_ref().is_none_or(|t| *t == p.tag)
                    && (hint.classes.is_empty() || hint.classes.iter().any(|c| p.classes.contains(c)))
                    && hint.element_text.as_ref().is_none_or(|t| p.static_text(&els).is_none_or(|e| normalize(&e) == normalize(t)))
            });
            if fits {
                fitting.push(hit);
            }
            found.push(hit);
        }
    }
    found.extend(prose_marks(site, &mark, shown.trim()));
    let chosen = match (found.len(), fitting.len()) {
        (1, _) => found[0],
        (_, 1) => fitting[0],
        (0, _) => return Err(format!("\"{}\" is not written as {} in the source", short(&want), mark.label)),
        (n, _) => return Err(format!("\"{}\" is written as {} in {n} places; unmark it in the source", short(&want), mark.label)),
    };
    let (file, whole, inner) = chosen;
    let inner_text = site.files[file].text[inner.start..inner.end].to_string();
    Ok(splice(site, file, whole, &inner_text))
}

/// `text` written in `mark`'s prose syntax in the prose of literate files.
fn prose_marks(site: &Site, mark: &Mark, text: &str) -> Vec<(usize, Span, Span)> {
    let mut out = Vec::new();
    if text.is_empty() {
        return out;
    }
    for (fi, file) in site.files.iter().enumerate() {
        let src = &file.text;
        let mut from = 0;
        while let Some(pos) = src[from..].find(text) {
            let a = from + pos;
            let b = a + text.len();
            from = b;
            let lang = marks::lang_at(&file.path, src, a);
            if lang == Lang::Markup {
                continue;
            }
            let inner = Span::new(a, b);
            if mark.tag == "a" {
                // [text](href) / [[href][text]]
                let (head, tail_open, tail_close) = if lang == Lang::Markdown { ("[", "](", ")") } else { ("][", "]]", "") };
                if !src[..a].ends_with(head) || !src[b..].starts_with(tail_open) {
                    continue;
                }
                let start = if lang == Lang::Markdown { a - 1 } else { match src[..a].rfind("[[") { Some(s) => s, None => continue } };
                let end = if lang == Lang::Markdown {
                    match src[b + 2..].find(tail_close) { Some(e) => b + 2 + e + 1, None => continue }
                } else {
                    b + 2
                };
                out.push((fi, Span::new(start, end), inner));
            } else if let Ok((open, close)) = mark.wrap(lang, &BTreeMap::new()) {
                if src[..a].ends_with(&open) && src[b..].starts_with(&close) {
                    out.push((fi, Span::new(a - open.len(), b + close.len()), inner));
                }
            }
        }
    }
    out
}

// ---- lists -----------------------------------------------------------------

fn find_list(site: &Site, binding: &str) -> Result<(List, Vec<Lit>), String> {
    let binding = binding.trim_start_matches('$');
    let matches: Vec<List> = site.lists().into_iter().filter(|l| l.binding == binding).collect();
    let list = match matches.len() {
        1 => matches.into_iter().next().unwrap(),
        0 => return Err(format!("no list named ${binding} in this page's files")),
        n => return Err(format!("${binding} is declared {n} times; refusing to guess")),
    };
    let Lit::Array { items, .. } = &list.value else {
        return Err(format!("${binding} is not a list"));
    };
    let items = items.clone();
    Ok((list, items))
}

fn row<'a>(items: &'a [Lit], binding: &str, index: usize) -> Result<&'a Lit, String> {
    items.get(index).ok_or_else(|| format!("${binding} has no row {index} (it has {})", items.len()))
}

fn set_field(site: &Site, binding: &str, index: usize, field: &str, value: &str) -> Result<Changes, String> {
    let (list, items) = find_list(site, binding)?;
    let row = row(&items, binding, index)?;
    let target = if field.is_empty() {
        row
    } else {
        row.get(field).ok_or_else(|| format!("row {index} of ${binding} has no field '{field}'"))?
    };
    let replacement = match target {
        Lit::Str { .. } => literal::encode_str(value),
        Lit::Other { .. } if value == "true" || value == "false" || value.trim().parse::<f64>().is_ok() => value.trim().to_string(),
        Lit::Other { .. } => literal::encode_str(value),
        _ => return Err(format!("'{field}' is not a plain value; edit it in the source")),
    };
    Ok(splice(site, list.file, target.span(), &replacement))
}

/// Rewrite a list's rows in a new order, keeping every separator where it was.
fn reorder(site: &Site, list: &List, items: &[Lit], order: &[usize]) -> Changes {
    let text = &site.files[list.file].text;
    let first = items[0].span().start;
    let last = items[items.len() - 1].span().end;
    let gaps: Vec<&str> = items.windows(2).map(|w| &text[w[0].span().end..w[1].span().start]).collect();
    let fallback_gap = gaps.first().copied().unwrap_or(", ");
    let mut out = String::new();
    for (i, &k) in order.iter().enumerate() {
        if i > 0 {
            out.push_str(gaps.get(i - 1).copied().unwrap_or(fallback_gap));
        }
        out.push_str(&text[items[k].span().start..items[k].span().end]);
    }
    splice(site, list.file, Span::new(first, last), &out)
}

fn move_row(site: &Site, binding: &str, from: usize, to: usize) -> Result<Changes, String> {
    let (list, items) = find_list(site, binding)?;
    row(&items, binding, from)?;
    if to >= items.len() {
        return Err(format!("${binding} has no position {to}"));
    }
    if from == to {
        return Ok(Changes::new());
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    let moved = order.remove(from);
    order.insert(to, moved);
    Ok(reorder(site, &list, &items, &order))
}

fn duplicate_row(site: &Site, binding: &str, index: usize) -> Result<Changes, String> {
    let (list, items) = find_list(site, binding)?;
    let src = row(&items, binding, index)?.span();
    let text = &site.files[list.file].text;
    let gap = if index + 1 < items.len() {
        &text[src.end..items[index + 1].span().start]
    } else if index > 0 {
        &text[items[index - 1].span().end..items[index].span().start]
    } else {
        ", "
    };
    let copy = format!("{gap}{}", &text[src.start..src.end]);
    Ok(splice(site, list.file, Span::new(src.end, src.end), &copy))
}

fn delete_row(site: &Site, binding: &str, index: usize) -> Result<Changes, String> {
    let (list, items) = find_list(site, binding)?;
    let target = row(&items, binding, index)?.span();
    // Take the row and ONE separator: the following one, or the preceding one
    // for the last row, so the remaining rows keep their spacing.
    let span = if index + 1 < items.len() {
        Span::new(target.start, items[index + 1].span().start)
    } else if index > 0 {
        Span::new(items[index - 1].span().end, target.end)
    } else {
        target
    };
    Ok(splice(site, list.file, span, ""))
}

// ---- style declarations ----------------------------------------------------------

fn decl(site: &Site, file: &str, start: usize, end: usize, expect: &str, value: &str) -> Result<Changes, String> {
    let fi = site.files.iter().position(|f| f.path == file).ok_or_else(|| format!("{file} is not part of this page"))?;
    let text = &site.files[fi].text;
    let current = text.get(start..end).ok_or("the declaration span is outside the file")?;
    if current != expect {
        return Err("the source changed under this edit; reload and try again".to_string());
    }
    // The parser's declaration span covers the VALUE only (`1100px`); a span
    // that also holds `prop:` keeps its property and trailing `;`.
    let replacement = match current.find(':') {
        Some(colon) => {
            let tail_end = current.trim_end().trim_end_matches(';').len();
            format!("{} {}{}", &current[..=colon], value.trim(), &current[tail_end..])
        }
        None => value.trim().to_string(),
    };
    Ok(splice(site, fi, Span::new(start, end), &replacement))
}

// ---- shared -------------------------------------------------------------------

fn splice(site: &Site, file: usize, span: Span, replacement: &str) -> Changes {
    let text = &site.files[file].text;
    let mut out = String::with_capacity(text.len() + replacement.len());
    out.push_str(&text[..span.start]);
    out.push_str(replacement);
    out.push_str(&text[span.end..]);
    let mut changes = Changes::new();
    changes.insert(site.files[file].path.clone(), out);
    changes
}

fn short(text: &str) -> String {
    let t: String = text.chars().take(48).collect();
    if t.len() < text.len() { format!("{t}…") } else { t }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"@data inline $tenets Tenet[] : [
  { "n": "01", "pre": "AI isn't ", "note": "Same text" },
  { "n": "02", "pre": "Small teams ", "note": "Same text" },
  { "n": "03", "pre": "Open source ", "note": "Other" }
];
<h1 class="hero__title">I write <em>stories</em>.</h1>
<p class="a">Hello</p>
<p class="b">Hello</p>
.hero__title { color: red; }
"#;

    fn site() -> Site {
        Site::from_files(&[("index.st", PAGE)])
    }

    fn run(op: Op) -> Result<String, String> {
        apply(&site(), &op).map(|c| c.get("index.st").cloned().unwrap_or_else(|| PAGE.to_string()))
    }

    fn text_op(old: &str, new: &str, hint: Hint) -> Op {
        Op::Text { old: old.into(), new: new.into(), hint }
    }

    #[test]
    fn unique_markup_text_is_replaced_in_place() {
        let out = run(text_op("I write", "I build", Hint::default())).unwrap();
        assert!(out.contains("<h1 class=\"hero__title\">I build <em>stories</em>.</h1>"));
    }

    #[test]
    fn unique_data_text_is_replaced_as_a_string_literal() {
        let out = run(text_op("AI isn't", "AI \"is not\"", Hint::default())).unwrap();
        assert!(out.contains(r#""pre": "AI \"is not\"""#));
    }

    #[test]
    fn duplicates_need_a_hint_and_are_refused_without_one() {
        assert!(run(text_op("Same text", "X", Hint::default())).is_err());
        let hint = Hint { binding: Some("tenets".into()), index: Some(1), ..Hint::default() };
        let out = run(text_op("Same text", "X", hint)).unwrap();
        assert!(out.contains(r#""pre": "Small teams ", "note": "X""#));
        assert!(out.contains(r#""pre": "AI isn't ", "note": "Same text""#));
        let hint = Hint { classes: vec!["b".into()], tag: Some("p".into()), ..Hint::default() };
        let out = run(text_op("Hello", "Bye", hint)).unwrap();
        assert!(out.contains("<p class=\"a\">Hello</p>\n<p class=\"b\">Bye</p>"));
    }

    #[test]
    fn computed_text_is_refused() {
        let err = run(text_op("10 things", "x", Hint::default())).unwrap_err();
        assert!(err.contains("computed"), "{err}");
    }

    #[test]
    fn rows_move_keep_their_separators() {
        let out = run(Op::Move { binding: "tenets".into(), from: 2, to: 0 }).unwrap();
        let a = out.find("\"03\"").unwrap();
        let b = out.find("\"01\"").unwrap();
        let c = out.find("\"02\"").unwrap();
        assert!(a < b && b < c);
        assert!(out.starts_with("@data inline $tenets Tenet[] : [\n  { \"n\": \"03\""));
        assert!(out.contains("\"Other\" },\n  { \"n\": \"01\""));
        assert!(out.contains("\"Same text\" }\n];"));
    }

    #[test]
    fn rows_duplicate_and_delete() {
        let out = run(Op::Duplicate { binding: "tenets".into(), index: 2 }).unwrap();
        assert_eq!(out.matches("\"03\"").count(), 2);
        let out = run(Op::Delete { binding: "$tenets".into(), index: 0 }).unwrap();
        assert!(!out.contains("\"01\""));
        assert!(out.contains(": [\n  { \"n\": \"02\""));
        let out = run(Op::Delete { binding: "tenets".into(), index: 2 }).unwrap();
        assert!(out.contains("\"Same text\" }\n];"));
    }

    #[test]
    fn fields_keep_their_type() {
        let out = run(Op::Field { binding: "tenets".into(), index: 0, field: "n".into(), value: "1".into() }).unwrap();
        assert!(out.contains(r#"{ "n": "1", "pre""#));
        assert!(run(Op::Field { binding: "tenets".into(), index: 9, field: "n".into(), value: "x".into() }).is_err());
    }

    #[test]
    fn declarations_are_guarded_by_their_text() {
        let start = PAGE.find("color: red;").unwrap();
        let end = start + "color: red;".len();
        let op = Op::Decl { file: "index.st".into(), start, end, expect: "color: red;".into(), value: "blue".into() };
        assert!(run(op).unwrap().contains(".hero__title { color: blue; }"));
        let v = PAGE.find("red;").unwrap();
        let value_only = Op::Decl { file: "index.st".into(), start: v, end: v + 3, expect: "red".into(), value: "teal".into() };
        assert!(run(value_only).unwrap().contains(".hero__title { color: teal; }"));
        let stale = Op::Decl { file: "index.st".into(), start, end, expect: "color: green;".into(), value: "blue".into() };
        assert!(run(stale).is_err());
    }

    fn mark_op(old: &str, sel: &str, mark: &str, attrs: &[(&str, &str)]) -> Op {
        let start = old.find(sel).unwrap();
        Op::Mark {
            old: old.into(),
            start: old[..start].encode_utf16().count(),
            end: old[..start + sel.len()].encode_utf16().count(),
            mark: mark.into(),
            attrs: attrs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            hint: Hint::default(),
        }
    }

    #[test]
    fn marks_wrap_the_selected_characters_of_markup() {
        // a double-click takes the trailing space along; the mark does not
        let out = run(mark_op("I write ", "write ", "--bold", &[])).unwrap();
        assert!(out.contains("<h1 class=\"hero__title\">I <strong>write</strong> <em>stories</em>.</h1>"), "{out}");
        let out = run(mark_op("I write ", "I", "--link", &[("href", "/who")])).unwrap();
        assert!(out.contains("><a href=\"/who\">I</a> write <em>"), "{out}");
        assert!(run(mark_op("I write ", "I", "--link", &[])).unwrap_err().contains("href"));
        let err = run(mark_op("AI isn't ", "AI", "--bold", &[])).unwrap_err();
        assert!(err.contains("$tenets"), "{err}");
    }

    #[test]
    fn marks_map_through_entities_and_reflowed_whitespace() {
        let page = "<p class=\"x\">Fish &amp; chips,\n    twice</p>\n";
        let site = Site::from_files(&[("index.st", page)]);
        let old = "Fish & chips,\n    twice";
        let start = old.find("chips").unwrap();
        let op = Op::Mark { old: old.into(), start, end: old.len(), mark: "--italic".into(), attrs: BTreeMap::new(), hint: Hint::default() };
        let out = apply(&site, &op).unwrap()["index.st"].clone();
        assert_eq!(out, "<p class=\"x\">Fish &amp; <em>chips,\n    twice</em></p>\n");
    }

    #[test]
    fn marks_in_prose_use_the_prose_syntax() {
        let md = "```st\n<p>code</p>\n```\nWe are still in the design phase.\n";
        let site = Site::from_files(&[("post.st.md", md)]);
        let old = "We are still in the design phase.";
        let s = old.find("design").unwrap();
        let op = Op::Mark { old: old.into(), start: s, end: s + 6, mark: "--bold".into(), attrs: BTreeMap::new(), hint: Hint::default() };
        let out = apply(&site, &op).unwrap()["post.st.md"].clone();
        assert!(out.contains("in the **design** phase."), "{out}");
        let site = Site::from_files(&[("post.st.md", out.as_str())]);
        let op = Op::Unmark { text: "design".into(), mark: "--bold".into(), hint: Hint::default() };
        assert_eq!(apply(&site, &op).unwrap()["post.st.md"], md);

        let org = "Read the [[https://x.y][notes]] and /this/.\n";
        let site = Site::from_files(&[("p.st.org", org)]);
        let op = Op::Unmark { text: "notes".into(), mark: "--link".into(), hint: Hint::default() };
        assert_eq!(apply(&site, &op).unwrap()["p.st.org"], "Read the notes and /this/.\n");
        let op = Op::Unmark { text: "this".into(), mark: "--italic".into(), hint: Hint::default() };
        assert_eq!(apply(&site, &op).unwrap()["p.st.org"], "Read the [[https://x.y][notes]] and this.\n");
    }

    #[test]
    fn unmark_takes_markup_text_out_of_its_element() {
        let out = run(Op::Unmark { text: "stories".into(), mark: "--italic".into(), hint: Hint::default() }).unwrap();
        assert!(out.contains("<h1 class=\"hero__title\">I write stories.</h1>"), "{out}");
        let err = run(Op::Unmark { text: "Hello".into(), mark: "--bold".into(), hint: Hint::default() }).unwrap_err();
        assert!(err.contains("not written as Bold"), "{err}");
    }
}
