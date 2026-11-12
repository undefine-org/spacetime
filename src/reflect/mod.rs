//! Reflect — the program as tools see it (PLAN-156 P1).
//!
//! One projection of a page's source, shared by every tool that reads code:
//! the studio, the admin, the LSP and MCP agents. For every form match in
//! every file of the page it gives
//!
//! - **where it is**: its file, its byte span, its id;
//! - **what it is**: the directive and the exact `%macro` the parser chose,
//!   the `///` doc written above it, the entries its macro `%registers`
//!   (`clip`, `part`, …) with their fields resolved for THIS call;
//! - **what it says**: every param its macro's `%form` declares — name,
//!   capture type, default, whether it is optional — and, for the ones the
//!   author wrote, the words and their span; the ones left out are marked
//!   `written: null` and carry the form's default;
//! - **what it holds**: the matches nested in its body, by id.
//!
//! Nothing here knows a macro by name. A tool that wants « every clip » asks
//! for the matches whose entries include `clip`; the stdlib says which those
//! are, next to the forms that define them.
//!
//! Spans are byte offsets into the file text as it is on disk, so an edit
//! addressed from here is an edit `crate::rewrite` can check and apply.

use std::collections::{BTreeMap, HashSet};

use serde_json::{Value, json};

use crate::MetaRegistry;
use crate::parser::meta_ast::{
    CaptureModifier, FormInlineElement, MacroDefAst, MetaDef, ParamDefault, RegisterItem, RegisterValue,
    RegistersArg,
};
use crate::syntax::{CapturedValue, FormMatch};

/// A file of the page: its path relative to the site, and its text.
pub struct File<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// The FNV-1a hash of a file's text, in hex: what a writer compares to know
/// the text it was addressed against is still the text on disk.
pub fn text_hash(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Reflect the page `entry` of the site at `site_dir`: its files (the entry,
/// its imports, the project's `_prelude.st`) against the stdlib registry and
/// the project overlay. What `GET /__spacetime/reflect.json` and MCP's
/// `st_reflect` answer.
pub fn page(site_dir: &std::path::Path, entry: &str) -> Result<Value, String> {
    let site = crate::studio::outline::Site::load(site_dir, entry)?;
    let (mut registry, _) = crate::compiler::cached_stdlib_registry();
    let _ = crate::compiler::load_project_overlay(&mut registry, Some(site_dir));
    // the declarations the page's imports bring (`@import "stdlib/3d"`),
    // resolved the way the page itself compiles
    let imported = crate::compiler::Compiler::from_file(&site_dir.join(entry), site_dir)
        .map(|c| c.merged_ast().meta_defs.clone())
        .unwrap_or_default();
    let files: Vec<File<'_>> = site.files.iter().map(|f| File { path: &f.path, text: &f.text }).collect();
    let mut out = reflect_with(&files, &registry, &imported, Some(site_dir));
    out["entry"] = Value::from(entry);
    Ok(out)
}

/// Reflect the files of one page. `registry` is the stdlib registry (plus the
/// project overlay); the page's own `%macro`s and `%capture_type`s are added
/// from its files, so a project macro is reflected like a stdlib one.
pub fn reflect(files: &[File<'_>], registry: &MetaRegistry) -> Value {
    reflect_with(files, registry, &[], None)
}

/// [`reflect`], with the declarations the page imports (`stdlib/3d`'s
/// `%macro`s): they take part in matching the page's files and in reading
/// their params.
pub fn reflect_with(
    files: &[File<'_>],
    registry: &MetaRegistry,
    imported: &[MetaDef],
    site_dir: Option<&std::path::Path>,
) -> Value {
    let mut registry = registry.clone();
    let mut parsed = Vec::new();
    let mut user_defs: Vec<MetaDef> = imported.to_vec();
    for f in files {
        match crate::parser::parse(f.text) {
            Ok(ast) => {
                user_defs.extend(ast.meta_defs.iter().cloned());
                parsed.push(Ok(ast));
            }
            Err(e) => parsed.push(Err(format!("{e:?}"))),
        }
    }
    for def in &user_defs {
        match def {
            MetaDef::Macro(m) => {
                let _ = registry.register_macro(m.clone());
            }
            // a keyword set's words and docs are a param's choices
            MetaDef::CaptureType(ct) => {
                let _ = registry.register_capture_type(ct.clone());
            }
            _ => {}
        }
    }

    let mut out_files = Vec::new();
    let mut matches = Vec::new();
    for (fi, (f, ast)) in files.iter().zip(parsed).enumerate() {
        let mut file = json!({ "path": f.path, "hash": text_hash(f.text) });
        let mut ast = match ast {
            Ok(ast) => ast,
            Err(e) => {
                file["error"] = Value::from(e);
                out_files.push(file);
                continue;
            }
        };
        // The page's own macros (from any of its files) take part in this
        // file's parse, as they do when the page compiles.
        if !user_defs.is_empty() {
            ast.meta_defs = user_defs.clone();
            crate::parser::rematch_with_user_macros_in(&mut ast, f.text, Some(f.path));
        }
        let mut seen = HashSet::new();
        let full = site_dir.map(|d| d.join(f.path).to_string_lossy().to_string());
        let mut walker =
            Walker { file: fi, path: full.as_deref(), text: f.text, registry: &registry, out: &mut matches };
        for scope in &ast.scopes {
            let selector = Some(scope.selector.as_str());
            for m in &scope.matches {
                if seen.insert(m.span.start) {
                    walker.visit(m, selector, None);
                }
            }
            for nested in &scope.nested_scopes {
                for m in &nested.matches {
                    if seen.insert(m.span.start) {
                        walker.visit(m, Some(nested.selector.as_str()), None);
                    }
                }
            }
        }
        for m in &ast.matches {
            if seen.insert(m.span.start) {
                walker.visit(m, None, None);
            }
        }
        out_files.push(file);
    }
    json!({ "files": out_files, "matches": matches })
}

struct Walker<'a, 'o> {
    file: usize,
    /// the file's path on disk, when known: a `%facts` value resolves next to it
    path: Option<&'a str>,
    text: &'a str,
    registry: &'a MetaRegistry,
    out: &'o mut Vec<Value>,
}

impl Walker<'_, '_> {
    fn id(&self, m: &FormMatch) -> String {
        format!("{}:{}", self.file, m.span.start)
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        self.text.get(start..end).unwrap_or("")
    }

    fn address(&self, start: usize, end: usize) -> Value {
        json!({ "file": self.file, "start": start, "end": end, "text": self.slice(start, end) })
    }

    /// Reflect `m`, then the matches nested in its captures. Returns its id.
    fn visit(&mut self, m: &FormMatch, selector: Option<&str>, parent: Option<&str>) -> String {
        let id = self.id(m);
        let def = self.registry.get_macro_for_match(m.matched_macro.as_deref(), &m.macro_name);
        let params = def.map(|d| self.params(m, d)).unwrap_or_default();
        let entries = def.map(|d| self.entries(m, d)).unwrap_or_default();
        // index now, fill children after: a child's own entry follows its parent's
        let at = self.out.len();
        self.out.push(Value::Null);
        let mut children = Vec::new();
        let mut keys: Vec<&String> = m.captures.keys().collect();
        keys.sort();
        for key in keys {
            for child in blocks(&m.captures[key]) {
                children.push(Value::from(self.visit(child, selector, Some(&id))));
            }
        }
        let (start, end) = (m.span.start as usize, m.span.end as usize);
        self.out[at] = json!({
            "id": id,
            "directive": m.macro_name,
            "macro": m.matched_macro.clone().unwrap_or_else(|| m.macro_name.clone()),
            "file": self.file,
            "start": start,
            "end": end,
            "selector": selector,
            "parent": parent,
            "doc": m.doc,
            "macro_doc": def.and_then(|d| d.doc.clone()),
            "params": params,
            "entries": entries,
            "shorthands": def.map(|d| d.shorthands.clone()).unwrap_or_default(),
            "spans": self.spans(m),
            "children": children,
        });
        id
    }

    /// The params the macro's `%form` declares, each with what the author
    /// wrote (or `null`, and the default).
    fn params(&self, m: &FormMatch, def: &MacroDefAst) -> Vec<Value> {
        let Some(form) = &def.form else { return Vec::new() };
        let mut out = Vec::new();
        // a param's type, when it is a row of the scalar table, says how to edit it
        let scalar = |ty: &str| -> Value {
            match self.registry.scalar_types().find(|s| s.id == ty) {
                Some(s) => json!({
                    "widget": s.widget, "range": [s.range.0, s.range.1], "step": s.step,
                    "unit": s.unit, "docs": s.docs,
                }),
                None => Value::Null,
            }
        };
        // the words a param may take: an inline union, or a named keyword set
        // (each word with its `///` doc)
        let choices = |ct: &crate::parser::meta_ast::CaptureType| -> Value {
            use crate::parser::meta_ast::CaptureType as C;
            match ct {
                C::Union(words) => Value::from(words.iter().map(|w| json!({ "word": w })).collect::<Vec<_>>()),
                C::Custom(name) => match self.registry.get_capture_type(name) {
                    Some(def) => match crate::syntax::events::keyword_set(&def.pattern) {
                        Some(words) => Value::from(
                            words.iter().map(|w| json!({ "word": w, "doc": def.docs.get(w) })).collect::<Vec<_>>(),
                        ),
                        None => Value::Null,
                    },
                    None => Value::Null,
                },
                _ => Value::Null,
            }
        };
        let mut push = |name: &str, var: &str, ct: &crate::parser::meta_ast::CaptureType, optional: bool, default: Option<&ParamDefault>, positional: bool, doc: Option<&String>| {
            let ty = crate::metasystem::signature::capture_type_name(ct);
            let written = m.capture_spans.get(var).map(|s| self.address(s.start as usize, s.end as usize));
            let scalar = scalar(&ty);
            let mut choices = choices(ct);
            // a value that names something in a file: the file's names
            if let Some((sibling, reader)) = self.registry.scalar_types().find(|s| s.id == ty).and_then(|s| s.facts.clone())
                && let Some(file) = m.captures.get(&sibling).and_then(|v| value_json(v).as_str().map(str::to_string))
                    .and_then(|src| crate::facts::resolve(self.path, &src))
                && let Ok(words) = crate::facts::read(&reader, &file)
            {
                choices = Value::from(words.iter().map(|w| json!({ "word": w })).collect::<Vec<_>>());
            }
            out.push(json!({
                "name": name,
                "capture": var,
                "type": ty,
                "optional": optional,
                "positional": positional,
                "default": default.map(default_text),
                "doc": doc,
                "scalar": scalar,
                "choices": choices,
                "written": written,
                "value": m.captures.get(var).map(value_json),
            }));
        };
        for el in &form.inline_elements {
            if let FormInlineElement::Capture(c, default) = el {
                push(
                    &c.var_name,
                    &c.var_name,
                    &c.capture_type,
                    matches!(c.modifier, CaptureModifier::Optional | CaptureModifier::ZeroOrMore),
                    default.as_ref(),
                    true,
                    None,
                );
            }
        }
        for p in &form.params {
            let Some(c) = p.capture() else { continue };
            push(
                &p.name,
                &c.var_name,
                &c.capture_type,
                p.default.is_some() || matches!(c.modifier, CaptureModifier::Optional | CaptureModifier::ZeroOrMore),
                p.default.as_ref(),
                false,
                p.doc.as_ref(),
            );
        }
        out
    }

    /// The macro's `%registers` clause, resolved for this call: `$var` fields
    /// become the words this match captured for them.
    fn entries(&self, m: &FormMatch, def: &MacroDefAst) -> Vec<Value> {
        let Some(reg) = &def.registers else { return Vec::new() };
        let resolve = |var: &str| -> Value {
            let var = var.trim_start_matches('$');
            m.captures.get(var).map(value_json).unwrap_or(Value::Null)
        };
        let key = reg.args.iter().find_map(|a| match a {
            RegistersArg::Positional(v) => Some(resolve(v)),
            RegistersArg::Named { .. } => None,
        });
        let mut fields = BTreeMap::new();
        for a in &reg.args {
            if let RegistersArg::Named { name, var } = a {
                fields.insert(name.clone(), resolve(var));
            }
        }
        for item in &reg.items {
            if let RegisterItem::Field(f) = item {
                let v = match &f.value {
                    RegisterValue::Var(v) => resolve(v),
                    RegisterValue::Bool(b) => json!(b),
                    RegisterValue::String(s) | RegisterValue::Ident(s) => json!(s),
                    other => json!(format!("{other:?}")),
                };
                fields.insert(f.name.clone(), v);
            }
        }
        vec![json!({ "kind": reg.name, "key": key, "fields": fields })]
    }

    /// Every capture span the parse kept, nested paths included
    /// (`entries.3.at`), as addresses: what a tool edits below the params.
    fn spans(&self, m: &FormMatch) -> BTreeMap<String, Value> {
        m.capture_spans
            .iter()
            .map(|(k, s)| (k.clone(), self.address(s.start as usize, s.end as usize)))
            .collect()
    }
}

/// The matches a capture holds as its body (`children`, `body`, …).
fn blocks(v: &CapturedValue) -> Vec<&FormMatch> {
    match v {
        CapturedValue::Block(ms) => ms.iter().collect(),
        CapturedValue::Array(items) => items.iter().flat_map(blocks).collect(),
        CapturedValue::Named(fields) => {
            let mut keys: Vec<&String> = fields.keys().collect();
            keys.sort();
            keys.into_iter().flat_map(|k| blocks(&fields[k])).collect()
        }
        _ => Vec::new(),
    }
}

/// A captured value as JSON: scalars as themselves, structures as text.
fn value_json(v: &CapturedValue) -> Value {
    match v {
        CapturedValue::String(s)
        | CapturedValue::Ident(s)
        | CapturedValue::Color(s)
        | CapturedValue::Expr(s)
        | CapturedValue::Selector(s)
        | CapturedValue::Element(s)
        | CapturedValue::Binding(s)
        | CapturedValue::TypeRef(s)
        | CapturedValue::Preset(s) => Value::from(s.clone()),
        CapturedValue::Number(n) => json!(n),
        CapturedValue::Bool(b) => json!(b),
        CapturedValue::Time(ms) => json!(format!("{ms}ms")),
        CapturedValue::Block(ms) => json!(format!("{} matches", ms.len())),
        CapturedValue::Named(f) if f.len() == 1 => f.values().next().map(value_json).unwrap_or(Value::Null),
        other => Value::from(format!("{other:?}")),
    }
}

/// A form default as the words an author would write.
fn default_text(d: &ParamDefault) -> String {
    match d {
        ParamDefault::String(s) => s.clone(),
        ParamDefault::Number(n) => {
            if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{n}") }
        }
        ParamDefault::Bool(b) => b.to_string(),
        ParamDefault::None => "none".into(),
        ParamDefault::EmptyArray => "[]".into(),
        ParamDefault::EmptyObject => "{}".into(),
        ParamDefault::Array(items) => format!("[{}]", items.iter().map(default_text).collect::<Vec<_>>().join(", ")),
        ParamDefault::Length(n, unit) => {
            if n.fract() == 0.0 { format!("{}{unit}", *n as i64) } else { format!("{n}{unit}") }
        }
    }
}

#[cfg(test)]
mod tests;
