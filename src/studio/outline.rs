//! The shape of a page's code — what the explorer and the inspector draw.
//!
//! [`Site::load`] gathers the files a page is made of (the entry, its
//! `@import`s, transitively, and the project's `_prelude.st`). [`Site::outline`]
//! then reads, per file:
//!
//! - **lists**: every `@data inline` / `@data stored` value, read losslessly;
//! - **markup**: every element literal, with its static text runs;
//! - **templates**: every `@template &name(params)` with its `///` doc;
//! - **rules**: every selector scope with its declarations and form splices,
//!   from the real parser's spans.
//!
//! Nothing here is cached: an outline is rebuilt from disk for every request,
//! so the spans it hands out describe the bytes that are there now.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::literal::{self, Lit};
use super::markup::{self, Element};
use super::Span;

/// One source file of the page, by its path relative to the site directory.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
}

/// An `@data inline` / `@data stored` declaration whose value was readable.
#[derive(Debug, Clone)]
pub struct List {
    pub binding: String,
    pub kind: String,
    pub file: usize,
    pub value: Lit,
}

/// The files of one page.
#[derive(Debug, Clone)]
pub struct Site {
    pub dir: PathBuf,
    pub entry: String,
    pub files: Vec<SourceFile>,
}

impl Site {
    /// Load `entry` (relative to `dir`) and everything it imports.
    pub fn load(dir: &Path, entry: &str) -> Result<Site, String> {
        let mut site = Site { dir: dir.to_path_buf(), entry: entry.to_string(), files: Vec::new() };
        let prelude = dir.join("_prelude.st");
        if prelude.exists() {
            site.add("_prelude.st")?;
        }
        site.add(entry)?;
        Ok(site)
    }

    /// Build a site from in-memory files (tests).
    pub fn from_files(files: &[(&str, &str)]) -> Site {
        Site {
            dir: PathBuf::new(),
            entry: files.first().map(|f| f.0.to_string()).unwrap_or_default(),
            files: files
                .iter()
                .map(|(p, t)| SourceFile { path: p.to_string(), text: t.to_string() })
                .collect(),
        }
    }

    fn add(&mut self, rel: &str) -> Result<(), String> {
        if self.files.iter().any(|f| f.path == rel) {
            return Ok(());
        }
        let path = self.dir.join(rel);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("read {rel}: {e}"))?;
        let imports = imports(&text);
        self.files.push(SourceFile { path: rel.to_string(), text });
        let base = Path::new(rel).parent().map(Path::to_path_buf).unwrap_or_default();
        for import in imports {
            if import.starts_with("stdlib/") || import.starts_with("std:") {
                continue;
            }
            if let Some(found) = self.resolve(&base, &import) {
                self.add(&found)?;
            }
        }
        Ok(())
    }

    /// Resolve an import path against the importing file's directory, trying the
    /// `.st` and literate extensions. Paths that leave the site are dropped.
    fn resolve(&self, base: &Path, import: &str) -> Option<String> {
        let joined = normalize_path(&base.join(import));
        let candidates = [
            joined.clone(),
            format!("{joined}.st"),
            format!("{joined}.st.md"),
            format!("{joined}.st.org"),
        ];
        candidates
            .into_iter()
            .filter(|c| !c.starts_with(".."))
            .find(|c| self.dir.join(c).is_file())
    }

    /// Every list in every file.
    pub fn lists(&self) -> Vec<List> {
        let mut out = Vec::new();
        for (fi, file) in self.files.iter().enumerate() {
            for (binding, kind, start) in data_decls(&file.text) {
                if let Ok(value) = literal::read(&file.text, start) {
                    out.push(List { binding, kind, file: fi, value });
                }
            }
        }
        out
    }

    /// The byte ranges of a file that are not markup: comments and data values.
    pub fn excluded(&self, file: usize) -> Vec<Span> {
        let text = &self.files[file].text;
        let mut ex = comment_spans(text);
        for list in self.lists().into_iter().filter(|l| l.file == file) {
            ex.push(list.value.span());
        }
        ex
    }

    /// Every element literal in `file`.
    pub fn elements(&self, file: usize) -> Vec<Element> {
        markup::elements(&self.files[file].text, &self.excluded(file))
    }

    /// The outline as JSON for the studio.
    pub fn outline(&self) -> Value {
        let lists: Vec<Value> = self
            .lists()
            .iter()
            .filter_map(|l| {
                let Lit::Array { items, span } = &l.value else { return None };
                let text = &self.files[l.file].text;
                let rows: Vec<Value> = items
                    .iter()
                    .enumerate()
                    .map(|(index, row)| row_json(index, row, text))
                    .collect();
                Some(json!({
                    "binding": l.binding,
                    "kind": l.kind,
                    "file": self.files[l.file].path,
                    "span": [span.start, span.end],
                    "rows": rows,
                    "count": items.len(),
                }))
            })
            .collect();

        let markup: Vec<Value> = (0..self.files.len())
            .map(|fi| {
                let els = self.elements(fi);
                let nodes: Vec<Value> = els
                    .iter()
                    .map(|e| {
                        let label = match e.classes.first() {
                            Some(c) => format!("{}.{}", e.tag, c),
                            None => e.tag.clone(),
                        };
                        let text = e.static_text(&els);
                        json!({
                            "index": e.index,
                            "parent": e.parent,
                            "depth": e.depth,
                            "tag": e.tag,
                            "classes": e.classes,
                            "label": label,
                            "text": text.clone().unwrap_or_else(|| dynamic_preview(e)),
                            "static": text.is_some(),
                            "span": [e.span.start, e.span.end],
                        })
                    })
                    .collect();
                json!({ "file": self.files[fi].path, "nodes": nodes })
            })
            .collect();

        let templates: Vec<Value> = self
            .files
            .iter()
            .flat_map(|f| templates(&f.text).into_iter().map(move |t| (f.path.clone(), t)))
            .map(|(file, t)| json!({ "name": t.0, "params": t.1, "doc": t.2, "group": t.3, "file": file }))
            .collect();

        let rules: Vec<Value> = self
            .files
            .iter()
            .filter(|f| f.path.ends_with(".st"))
            .flat_map(|f| rules(&f.text).into_iter().map(move |r| (f.path.clone(), r)))
            .map(|(file, mut r)| {
                r["file"] = json!(file);
                r
            })
            .collect();

        let title = (0..self.files.len())
            .flat_map(|fi| {
                let els = self.elements(fi);
                els.iter().filter(|e| e.tag == "title").filter_map(|e| e.static_text(&els)).collect::<Vec<_>>()
            })
            .next()
            .unwrap_or_default();

        json!({
            "entry": self.entry,
            "title": title,
            "files": self.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
            "lists": lists,
            "markup": markup,
            "templates": templates,
            "rules": rules,
        })
    }
}

fn row_json(index: usize, row: &Lit, text: &str) -> Value {
    let span = row.span();
    match row {
        Lit::Object { entries, .. } => {
            let fields: Vec<Value> = entries
                .iter()
                .map(|e| {
                    let kind = match &e.value {
                        Lit::Str { .. } => "string",
                        Lit::Other { raw, .. } if raw == "true" || raw == "false" => "bool",
                        Lit::Other { raw, .. } if raw.parse::<f64>().is_ok() => "number",
                        Lit::Other { .. } => "other",
                        _ => "nested",
                    };
                    json!({ "key": e.key, "value": e.value.text(text), "kind": kind })
                })
                .collect();
            json!({ "index": index, "label": row_label(row, text), "fields": fields, "span": [span.start, span.end] })
        }
        other => json!({
            "index": index,
            "label": other.text(text),
            "fields": [{ "key": "", "value": other.text(text), "kind": "string" }],
            "span": [span.start, span.end],
        }),
    }
}

/// The text that best names a row in a list: a title-like field, else the
/// first string field.
pub fn row_label(row: &Lit, text: &str) -> String {
    const PREFERRED: &[&str] = &["title", "name", "label", "line", "what", "mark", "text", "id"];
    if let Lit::Object { entries, .. } = row {
        for key in PREFERRED {
            if let Some(e) = entries.iter().find(|e| e.key == *key) {
                if let Lit::Str { value, .. } = &e.value {
                    if !value.trim().is_empty() {
                        return value.clone();
                    }
                }
            }
        }
        if let Some(e) = entries.iter().find(|e| matches!(e.value, Lit::Str { .. })) {
            return e.value.text(text);
        }
    }
    row.text(text)
}

fn dynamic_preview(e: &Element) -> String {
    e.runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `@import "path"` targets, in order (comments ignored).
fn imports(text: &str) -> Vec<String> {
    let ex = comment_spans(text);
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = text[from..].find("@import") {
        let at = from + pos;
        from = at + 7;
        if ex.iter().any(|s| s.start <= at && at < s.end) {
            continue;
        }
        let rest = text[at + 7..].trim_start();
        if let Some(q) = rest.strip_prefix('"') {
            if let Some(end) = q.find('"') {
                out.push(q[..end].to_string());
            }
        }
    }
    out
}

/// `(binding, kind, value_start)` for each `@data inline|stored $name …: <value>`.
pub fn data_decls(text: &str) -> Vec<(String, String, usize)> {
    let ex = comment_spans(text);
    let mut out = Vec::new();
    for kind in ["inline", "stored"] {
        let needle = format!("@data {kind} $");
        let mut from = 0;
        while let Some(pos) = text[from..].find(&needle) {
            let at = from + pos;
            from = at + needle.len();
            if ex.iter().any(|s| s.start <= at && at < s.end) {
                continue;
            }
            let name_start = at + needle.len();
            let name: String = text[name_start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            // The value follows the first `:` on the declaration (after the
            // optional type), before the statement ends.
            let decl = &text[name_start..];
            let stop = decl.find(';').unwrap_or(decl.len());
            if let Some(colon) = decl[..stop].find(':') {
                out.push((name, kind.to_string(), name_start + colon + 1));
            } else if let Some(colon) = decl.find(':') {
                // A multi-line array value contains `;` only after it ends.
                out.push((name, kind.to_string(), name_start + colon + 1));
            }
        }
    }
    out.sort_by_key(|d| d.2);
    out
}

/// `/* … */` blocks and `//` line comments (a `//` counts only at the start of
/// a line, after whitespace, so `https://` in text is never a comment).
pub fn comment_spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    let mut line_start = true;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let end = text[i + 2..].find("*/").map(|e| i + 2 + e + 2).unwrap_or(b.len());
            out.push(Span::new(i, end));
            i = end;
            line_start = false;
            continue;
        }
        if line_start && b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            let end = text[i..].find('\n').map(|e| i + e).unwrap_or(b.len());
            out.push(Span::new(i, end));
            i = end;
            continue;
        }
        if b[i] == b'\n' {
            line_start = true;
        } else if !b[i].is_ascii_whitespace() {
            line_start = false;
        }
        i += 1;
    }
    out
}

/// `(name, params, doc, group)` for each `@template &name(params)`.
fn templates(text: &str) -> Vec<(String, String, String, String)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = text[from..].find("@template &") {
        let at = from + pos;
        from = at + 11;
        let rest = &text[at + 11..];
        let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        let params = rest[name.len()..]
            .strip_prefix('(')
            .and_then(|p| p.find(')').map(|e| p[..e].to_string()))
            .unwrap_or_default();
        // The `///` block directly above.
        let mut doc_lines: Vec<&str> = text[..at]
            .lines()
            .rev()
            .skip_while(|l| l.trim().is_empty())
            .take_while(|l| l.trim_start().starts_with("///"))
            .map(|l| l.trim_start().trim_start_matches('/').trim())
            .collect();
        doc_lines.reverse();
        let group = doc_lines
            .iter()
            .find_map(|l| l.strip_prefix("group:").map(|g| g.trim().to_string()))
            .unwrap_or_default();
        let doc = doc_lines
            .iter()
            .filter(|l| !l.contains(':') || !l.split(':').next().unwrap_or("").chars().all(|c| c.is_ascii_alphabetic()))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        out.push((name, params, doc, group));
    }
    out
}

/// Selector rules with their declarations and splices, from the real parser.
fn rules(text: &str) -> Vec<Value> {
    let Ok(ast) = crate::parser::parse(text) else { return Vec::new() };
    let mut out = Vec::new();
    for scope in &ast.scopes {
        if scope.selector.starts_with('@') {
            continue;
        }
        push_rule(&mut out, &scope.selector, &scope.css_declarations, &scope.form_refs, scope.span.start, scope.span.end, text);
        for nested in &scope.nested_scopes {
            push_nested(&mut out, nested, text);
        }
    }
    out
}

fn push_nested(out: &mut Vec<Value>, scope: &crate::parser::ast::NestedScope, text: &str) {
    let selector = if scope.composed_selector.is_empty() { &scope.selector } else { &scope.composed_selector };
    push_rule(out, selector, &scope.css_declarations, &scope.form_refs, scope.span.start, scope.span.end, text);
    for nested in &scope.nested_scopes {
        push_nested(out, nested, text);
    }
}

fn push_rule(
    out: &mut Vec<Value>,
    selector: &str,
    decls: &[crate::parser::ast::CssDeclaration],
    forms: &[crate::parser::ast::FormRefUse],
    start: usize,
    end: usize,
    text: &str,
) {
    let decls: Vec<Value> = decls
        .iter()
        .filter(|d| !d.is_injection && d.span.end > d.span.start && d.span.end <= text.len())
        .map(|d| {
            json!({
                "prop": d.property,
                "value": d.value,
                "span": [d.span.start, d.span.end],
                "source": &text[d.span.start..d.span.end],
            })
        })
        .collect();
    let forms: Vec<Value> = forms
        .iter()
        .map(|f| json!({ "name": f.name, "args": f.args }))
        .collect();
    if decls.is_empty() && forms.is_empty() {
        return;
    }
    out.push(json!({ "selector": selector, "span": [start, end], "decls": decls, "forms": forms }));
}

/// Lexically normalize a relative path (`a/../b` -> `b`), without touching disk.
fn normalize_path(p: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                if parts.pop().is_none() {
                    parts.push("..".to_string());
                }
            }
            std::path::Component::CurDir => {}
            other => parts.push(other.as_os_str().to_string_lossy().to_string()),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"@import "_parts"
/* a comment with <b>tags</b> and @data inline $no : [1]; */
@data inline $tenets Tenet[] : [
  { "n": "01", "line": "AI isn't automation" },
  { "n": "02", "line": "Small teams" }
];
@data inline $q string : "";
//@todo: not markup <i>here</i>
<h1 class="hero__title">I write <em>stories</em>.</h1>
.hero__title { color: red; --role-shout; }
"#;

    #[test]
    fn finds_lists_skipping_comments() {
        let site = Site::from_files(&[("index.st", PAGE)]);
        let lists = site.lists();
        assert_eq!(lists.iter().map(|l| l.binding.as_str()).collect::<Vec<_>>(), vec!["tenets", "q"]);
        let Lit::Array { items, .. } = &lists[0].value else { panic!() };
        assert_eq!(row_label(&items[1], PAGE), "Small teams");
    }

    #[test]
    fn markup_excludes_comments_and_data() {
        let site = Site::from_files(&[("index.st", PAGE)]);
        let els = site.elements(0);
        assert_eq!(els.iter().map(|e| e.tag.as_str()).collect::<Vec<_>>(), vec!["h1", "em"]);
    }

    #[test]
    fn outline_reports_rules_templates_and_rows() {
        let parts = "/// A tenet row.\n/// group: manifesto\n@template &tenet($t) {\n  <li>`$t.line`</li>\n}\n";
        let site = Site::from_files(&[("index.st", PAGE), ("_parts.st", parts)]);
        let o = site.outline();
        assert_eq!(o["lists"][0]["rows"][0]["fields"][1]["value"], "AI isn't automation");
        assert_eq!(o["templates"][0]["name"], "tenet");
        assert_eq!(o["templates"][0]["group"], "manifesto");
        assert_eq!(o["templates"][0]["doc"], "A tenet row.");
        assert_eq!(imports(PAGE), vec!["_parts"]);
    }

    #[test]
    fn paths_normalize_without_escaping() {
        assert_eq!(normalize_path(Path::new("writing/../_chrome")), "_chrome");
        assert_eq!(normalize_path(Path::new("../x")), "../x");
    }
}
