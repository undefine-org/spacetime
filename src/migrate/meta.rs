//! Meta waves (PLAN-156): built-in rewrites of META-syntax.
//!
//! A `%migration` with `%scope meta` names `%transform`s from [`KNOWN`]. A
//! transform reads every `.st` file it is given (the stdlib, or a project's
//! macros) and returns guarded edits ([`crate::rewrite::Edit`]); the edits
//! are applied with [`crate::rewrite::apply`], so a transform that would leave
//! a file unparseable writes nothing.
//!
//! Meta-syntax is read by the compiler's own parser, not by a `%form`, which
//! is why these are code and not `%match`/`%into` rules. Each one is a general
//! rewrite of the declaration language, never a special case for one macro.
//!
//! - `docs-onto-declarations`: a `/// @param NAME - text` line moves onto the
//!   param it describes (a `///` directly above it, in the macro's `%form` or
//!   the primitive's signature); `@yields $x - text` moves above `$x` in
//!   `%exports` (or above its `%yield`); a doc that only restates the default
//!   is dropped; a primitive param a macro forwards is documented once, on
//!   the macro's form, where authors read it.
//! - `examples-to-clauses`: `/// @example title` blocks become
//!   `%example "title" { … }` clauses, which `spacetime check --examples`
//!   compiles.
//! - `forward-binds`: in a `%binds` call, the args that only forward a form
//!   param under its own name (`type: $type`) become one `...`.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::parser::meta_ast::{BindArg, BindValue, MacroDefAst, MetaDef, PrimitiveDefAst};
use crate::rewrite::Edit;

/// The transforms a `%migration` may name.
pub const KNOWN: &[&str] = &["docs-onto-declarations", "examples-to-clauses", "forward-binds"];

/// Run `transform` over `files` (path → text). Returns the edits it makes.
pub fn run(transform: &str, files: &BTreeMap<String, String>) -> Result<Vec<Edit>, String> {
    let set = Set::read(files);
    let mut edits = match transform {
        "docs-onto-declarations" => set.docs_onto_declarations(),
        "examples-to-clauses" => set.examples_to_clauses(),
        "forward-binds" => set.forward_binds(),
        other => return Err(format!("unknown transform `{other}`")),
    };
    // an edit that writes what is already there is no edit: a wave run twice
    // changes nothing the second time
    edits.retain(|e| e.text != e.expect);
    Ok(edits)
}

/// Run every transform of every `%scope meta` migration in `registry` newer
/// than `since` over `files`, in wave order. Each transform sees the files as
/// the previous one left them. Returns the changed files.
pub fn apply_waves(
    files: &BTreeMap<String, String>,
    registry: &crate::MetaRegistry,
    since: Option<&str>,
    wave: Option<&str>,
) -> Result<BTreeMap<String, String>, String> {
    let (changed, refused) = apply_waves_partial(files, registry, since, wave);
    if refused.is_empty() { Ok(changed) } else { Err(refused.join("\n")) }
}

/// [`apply_waves`], keeping what applied when some files are refused: the
/// changed files, and one line per refused file.
pub fn apply_waves_partial(
    files: &BTreeMap<String, String>,
    registry: &crate::MetaRegistry,
    since: Option<&str>,
    wave: Option<&str>,
) -> (BTreeMap<String, String>, Vec<String>) {
    let mut migs: Vec<_> = registry
        .all_migrations()
        .filter(|m| m.scope.as_deref() == Some("meta"))
        .filter(|m| since.is_none_or(|v| m.date.as_str() > v))
        .filter(|m| wave.is_none_or(|w| m.date == w))
        .collect();
    migs.sort_by(|a, b| (a.date.as_str(), a.id.as_str()).cmp(&(b.date.as_str(), b.id.as_str())));
    let mut now = files.clone();
    let mut refused = Vec::new();
    for m in migs {
        for t in &m.transforms {
            let Ok(edits) = run(t, &now) else { continue };
            let mut by_file: BTreeMap<String, Vec<Edit>> = BTreeMap::new();
            for e in edits {
                by_file.entry(e.file.clone()).or_default().push(e);
            }
            for (file, edits) in by_file {
                let tx = crate::rewrite::Transaction { edits, base: BTreeMap::new() };
                match crate::rewrite::apply(&now, &tx) {
                    Ok(changed) => now.extend(changed),
                    Err(e) => refused.push(format!("{} / {t}: {file}: {e}", m.id)),
                }
            }
        }
    }
    (now.into_iter().filter(|(k, v)| files.get(k) != Some(v)).collect(), refused)
}

// ---------------------------------------------------------------------------
// Reading the declarations
// ---------------------------------------------------------------------------

/// One `%macro` or `%primitive`, with where its parts are in its file.
struct Decl {
    file: String,
    kind: Kind,
    name: String,
    /// byte range of the declaration (`%macro` … its closing `}`)
    start: usize,
    end: usize,
    /// the `///` lines directly above it: byte range, and their text
    doc: Option<(usize, usize)>,
    tags: Tags,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Macro,
    Primitive,
}

#[derive(Default)]
struct Tags {
    /// kept lines of prose, without `///`
    prose: Vec<String>,
    /// param name → its text
    params: Vec<(String, String)>,
    /// `$x` → its text
    yields: Vec<(String, String)>,
    /// (title, code)
    examples: Vec<(String, String)>,
    /// was there a tag at all?
    any: bool,
}

struct Set<'a> {
    files: &'a BTreeMap<String, String>,
    decls: Vec<Decl>,
    macros: HashMap<String, MacroDefAst>,
    prims: HashMap<String, PrimitiveDefAst>,
}

impl<'a> Set<'a> {
    fn read(files: &'a BTreeMap<String, String>) -> Self {
        let mut decls = Vec::new();
        let mut macros = HashMap::new();
        let mut prims = HashMap::new();
        for (path, text) in files {
            let Ok(ast) = crate::parser::parse(text) else { continue };
            for def in &ast.meta_defs {
                let (kind, name, span) = match def {
                    MetaDef::Macro(m) => {
                        macros.insert(m.name.clone(), m.clone());
                        (Kind::Macro, m.name.clone(), m.span)
                    }
                    MetaDef::Primitive(p) => {
                        prims.insert(p.name.clone(), p.clone());
                        (Kind::Primitive, p.name.clone(), p.span)
                    }
                    _ => continue,
                };
                let (start, end) = (span.start as usize, span.end as usize);
                if end > text.len() || start >= end || !text[start..].starts_with('%') {
                    continue;
                }
                let doc = doc_block(text, start);
                let tags = doc.map(|(s, e)| parse_tags(&text[s..e])).unwrap_or_default();
                decls.push(Decl { file: path.clone(), kind, name, start, end, doc, tags });
            }
        }
        Set { files, decls, macros, prims }
    }

    fn text(&self, d: &Decl) -> &str {
        &self.files[&d.file]
    }

    /// For each primitive, the params macros forward to it:
    /// prim → prim param → [(macro, form param)].
    fn forwards(&self) -> HashMap<String, HashMap<String, Vec<(String, String)>>> {
        let mut out: HashMap<String, HashMap<String, Vec<(String, String)>>> = HashMap::new();
        for m in self.macros.values() {
            let by_var: HashMap<String, String> = form_params(m).into_iter().map(|(n, v)| (v, n)).collect();
            for b in &m.binds {
                for a in &b.args {
                    if let BindArg::Named { name, value: BindValue::Variable(v) } = a
                        && let Some(pname) = by_var.get(v)
                    {
                        out.entry(b.primitive.clone())
                            .or_default()
                            .entry(name.clone())
                            .or_default()
                            .push((m.name.clone(), pname.clone()));
                    }
                }
            }
        }
        out
    }

    // -----------------------------------------------------------------------
    // docs-onto-declarations
    // -----------------------------------------------------------------------

    fn docs_onto_declarations(&self) -> Vec<Edit> {
        let forwards = self.forwards();
        // the doc each macro's form param ends up with
        let mut macro_param_docs: HashMap<(String, String), String> = HashMap::new();
        for d in self.decls.iter().filter(|d| d.kind == Kind::Macro) {
            for (name, text) in &d.tags.params {
                if let Some(t) = clean_doc(name, text) {
                    macro_param_docs.insert((d.name.clone(), name.clone()), t);
                }
            }
        }
        // a primitive's doc fills a forwarding macro's undocumented param
        for d in self.decls.iter().filter(|d| d.kind == Kind::Primitive) {
            let Some(fw) = forwards.get(&d.name) else { continue };
            for (name, text) in &d.tags.params {
                let Some(t) = clean_doc(name, text) else { continue };
                for (mac, pname) in fw.get(name).into_iter().flatten() {
                    macro_param_docs.entry((mac.clone(), pname.clone())).or_insert_with(|| t.clone());
                }
            }
        }

        let mut edits = Vec::new();
        for d in &self.decls {
            if !d.tags.any {
                continue;
            }
            let text = self.text(d);
            let (ds, de) = d.doc.expect("tags come from a doc block");
            // the doc block keeps its prose (and examples, until their own wave)
            let mut kept = d.tags.prose.clone();
            for (title, code) in &d.tags.examples {
                if kept.last().is_some_and(|l| !l.is_empty()) {
                    kept.push(String::new());
                }
                kept.push(format!("@example {title}"));
                kept.extend(code.lines().map(str::to_string));
            }
            edits.push(Edit {
                file: d.file.clone(),
                start: ds,
                end: de,
                expect: text[ds..de].to_string(),
                text: render_doc_block(text, ds, &kept),
            });

            match d.kind {
                Kind::Macro => {
                    let docs: Vec<(String, String)> = self.macros.get(&d.name).map(form_params).unwrap_or_default()
                        .into_iter()
                        .filter_map(|(n, _)| macro_param_docs.get(&(d.name.clone(), n.clone())).map(|t| (n, t.clone())))
                        .collect();
                    if let Some(e) = self.document_list(d, "%form", &docs) {
                        edits.push(e);
                    }
                }
                Kind::Primitive => {
                    let fw = forwards.get(&d.name);
                    let docs: Vec<(String, String)> = d
                        .tags
                        .params
                        .iter()
                        .filter_map(|(n, t)| clean_doc(n, t).map(|t| (n.clone(), t)))
                        // documented once: on the macro that forwards it
                        .filter(|(n, _)| {
                            !fw.and_then(|f| f.get(n)).is_some_and(|ms| {
                                ms.iter().any(|(m, p)| macro_param_docs.contains_key(&(m.clone(), p.clone())))
                            })
                        })
                        .collect();
                    if let Some(e) = self.document_list(d, "%primitive", &docs) {
                        edits.push(e);
                    }
                    edits.extend(self.document_yields(d));
                }
            }
        }
        edits
    }

    /// Rewrite the param list of `d`'s `%form` (or its primitive signature) so
    /// each param in `docs` has its `///` lines above it.
    fn document_list(&self, d: &Decl, which: &str, docs: &[(String, String)]) -> Option<Edit> {
        if docs.is_empty() {
            return None;
        }
        let text = self.text(d);
        let body = &text[d.start..d.end];
        let (open, close) = if which == "%form" {
            let f = find_clause(body, "%form")?;
            let brace = body[f..].find('{')? + f;
            let end = matching(body, brace, b'{', b'}')?;
            let open = param_list_open(body, brace + 1, end)?;
            (open, matching(body, open, b'(', b')')?)
        } else {
            let open = body.find('(')?;
            (open, matching(body, open, b'(', b')')?)
        };
        let (open, close) = (d.start + open, d.start + close);
        let inner = &text[open + 1..close];
        let parts: Vec<String> = crate::parser::split_respecting_nesting_pub(inner, ',')
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let line_start = text[..open].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let base = indent_of(&text[line_start..]);
        let first_line_indent = inner
            .lines()
            .skip(1)
            .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with(')'))
            .map(indent_of);
        let ind = first_line_indent.unwrap_or_else(|| format!("{base}  "));
        let docs: HashMap<&str, &str> = docs.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
        let mut out = String::from("(\n");
        for (i, p) in parts.iter().enumerate() {
            let name = param_name(p);
            if let Some(doc) = docs.get(name.as_str()) {
                for l in doc.lines() {
                    out.push_str(&format!("{ind}/// {l}\n"));
                }
            }
            out.push_str(&ind);
            out.push_str(p);
            if i + 1 < parts.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str(&base);
        out.push(')');
        Some(Edit { file: d.file.clone(), start: open, end: close + 1, expect: text[open..=close].to_string(), text: out })
    }

    /// `@yields $x - text` → a `///` above `$x` in `%exports`, or above the
    /// `%yield … -> $x` line.
    fn document_yields(&self, d: &Decl) -> Vec<Edit> {
        let text = self.text(d);
        let mut out = Vec::new();
        let mut used = HashSet::new();
        for (var, doc) in &d.tags.yields {
            let body = &text[d.start..d.end];
            let exports = find_clause(body, "%exports").and_then(|e| {
                let b = body[e..].find('{')? + e;
                Some((b, matching(body, b, b'{', b'}')?))
            });
            let at = exports
                .and_then(|(b, e)| find_word(&body[b..e], &format!("${var}")).map(|i| b + i))
                .or_else(|| {
                    // the `%yield … -> $x` line
                    let mut from = 0;
                    while let Some(i) = body[from..].find(&format!("-> ${var}")) {
                        let at = from + i;
                        let ls = body[..at].rfind('\n').map(|n| n + 1).unwrap_or(0);
                        if body[ls..at].contains("%yield") {
                            return Some(ls);
                        }
                        from = at + 1;
                    }
                    None
                });
            let Some(at) = at else { continue };
            let ls = d.start + body[..at].rfind('\n').map(|n| n + 1).unwrap_or(0);
            if !used.insert(ls) {
                continue;
            }
            let ind = indent_of(&text[ls..]);
            let lines: String = doc.lines().map(|l| format!("{ind}/// {l}\n")).collect();
            out.push(Edit { file: d.file.clone(), start: ls, end: ls, expect: String::new(), text: lines });
        }
        out
    }

    // -----------------------------------------------------------------------
    // examples-to-clauses
    // -----------------------------------------------------------------------

    fn examples_to_clauses(&self) -> Vec<Edit> {
        // a primitive's examples go to the one macro that binds it
        let mut binders: HashMap<String, Vec<String>> = HashMap::new();
        for m in self.macros.values() {
            for b in &m.binds {
                binders.entry(b.primitive.clone()).or_default().push(m.name.clone());
            }
        }
        let mut moved: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for d in self.decls.iter().filter(|d| d.kind == Kind::Primitive && !d.tags.examples.is_empty()) {
            if let Some(ms) = binders.get(&d.name)
                && ms.len() == 1
            {
                moved.entry(ms[0].clone()).or_default().extend(d.tags.examples.iter().cloned());
            }
        }
        let mut edits = Vec::new();
        for d in &self.decls {
            let own = &d.tags.examples;
            let extra = if d.kind == Kind::Macro { moved.get(&d.name).cloned().unwrap_or_default() } else { Vec::new() };
            let leaves = d.kind == Kind::Macro || binders.get(&d.name).is_some_and(|m| m.len() == 1);
            if (own.is_empty() || !leaves) && extra.is_empty() {
                continue;
            }
            let text = self.text(d);
            if !own.is_empty() && leaves {
                let (ds, de) = d.doc.expect("examples come from a doc block");
                edits.push(Edit {
                    file: d.file.clone(),
                    start: ds,
                    end: de,
                    expect: text[ds..de].to_string(),
                    text: render_doc_block(text, ds, &d.tags.prose_with_params()),
                });
            }
            if d.kind != Kind::Macro {
                continue;
            }
            let examples: Vec<(String, String)> = own.iter().cloned().chain(extra).collect();
            // before the macro's closing brace
            let close = d.end - 1;
            let ls = text[..close].rfind('\n').map(|n| n + 1).unwrap_or(close);
            let base = indent_of(&text[text[..d.start].rfind('\n').map(|n| n + 1).unwrap_or(0)..]);
            let ind = format!("{base}  ");
            let mut add = String::new();
            for (i, (title, code)) in examples.iter().enumerate() {
                let title = match (title.trim(), i) {
                    ("", 0) => format!("@{}", d.name),
                    ("", _) => format!("@{} ({})", d.name, i + 1),
                    (t, _) => t.replace('"', "'"),
                };
                add.push_str(&format!("\n{ind}%example \"{title}\" {{\n"));
                for l in dedent_lines(code) {
                    if l.is_empty() {
                        add.push('\n');
                    } else {
                        add.push_str(&format!("{ind}  {l}\n"));
                    }
                }
                add.push_str(&format!("{ind}}}\n"));
            }
            edits.push(Edit { file: d.file.clone(), start: ls, end: ls, expect: String::new(), text: add });
        }
        edits
    }

    // -----------------------------------------------------------------------
    // forward-binds
    // -----------------------------------------------------------------------

    fn forward_binds(&self) -> Vec<Edit> {
        let mut edits = Vec::new();
        for d in self.decls.iter().filter(|d| d.kind == Kind::Macro) {
            let Some(m) = self.macros.get(&d.name) else { continue };
            let params: HashSet<String> = form_params(m).into_iter().filter(|(n, v)| n == v).map(|(n, _)| n).collect();
            let text = self.text(d);
            let body = &text[d.start..d.end];
            let Some(b) = find_clause(body, "%binds") else { continue };
            let Some(brace) = body[b..].find('{').map(|i| i + b) else { continue };
            let Some(bend) = matching(body, brace, b'{', b'}') else { continue };
            // every call in the clause: `name(args)`
            let mut at = brace + 1;
            while let Some(open) = body[at..bend].find('(').map(|i| i + at) {
                let Some(close) = matching(body, open, b'(', b')') else { break };
                let args: Vec<String> = crate::parser::split_respecting_nesting_pub(&body[open + 1..close], ',')
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let is_forward = |a: &str| {
                    a.split_once(':').is_some_and(|(n, v)| {
                        let (n, v) = (n.trim(), v.trim());
                        v.strip_prefix('$') == Some(n) && params.contains(n)
                    })
                };
                let fwd = args.iter().filter(|a| is_forward(a)).count();
                if fwd >= 3 && !args.iter().any(|a| a == "...") {
                    let mut kept: Vec<String> = args.iter().filter(|a| !is_forward(a)).cloned().collect();
                    kept.push("...".into());
                    let s = d.start + open;
                    let e = d.start + close + 1;
                    edits.push(Edit {
                        file: d.file.clone(),
                        start: s,
                        end: e,
                        expect: text[s..e].to_string(),
                        text: format!("({})", kept.join(", ")),
                    });
                }
                at = close + 1;
            }
        }
        edits
    }
}

impl Tags {
    /// The prose plus the `@param` lines (for a wave that only moves examples).
    fn prose_with_params(&self) -> Vec<String> {
        let mut out = self.prose.clone();
        if !out.is_empty() && !(self.params.is_empty() && self.yields.is_empty()) {
            out.push(String::new());
        }
        for (n, t) in &self.params {
            out.push(format!("@param {n} - {t}"));
        }
        for (n, t) in &self.yields {
            out.push(format!("@yields ${n} - {t}"));
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

/// The named params of a macro's `%form`, as (name, capture var).
fn form_params(m: &MacroDefAst) -> Vec<(String, String)> {
    m.form
        .iter()
        .flat_map(|f| f.params.iter().chain(f.body_params.iter()))
        .filter(|p| !p.name.is_empty())
        .filter_map(|p| p.capture().map(|c| (p.name.clone(), c.var_name.clone())))
        .collect()
}

/// The contiguous `///` lines directly above `start` (the declaration's first
/// byte): their byte range, from the first line's start to the last line's end
/// (newline included).
fn doc_block(text: &str, start: usize) -> Option<(usize, usize)> {
    let decl_line = text[..start].rfind('\n').map(|n| n + 1).unwrap_or(0);
    if !text[decl_line..start].trim().is_empty() {
        return None;
    }
    // the lines above the declaration, nearest first: (start offset, trimmed text)
    let mut above: Vec<(usize, &str)> = Vec::new();
    let mut at = decl_line;
    while at > 0 {
        let s = text[..at - 1].rfind('\n').map(|n| n + 1).unwrap_or(0);
        above.push((s, text[s..at].trim()));
        at = s;
    }
    let is_doc = |l: &str| l.starts_with("///");
    let is_plain = |l: &str| l.is_empty() || (l.starts_with("//") && !is_doc(l));
    // blank lines and plain `//` comments may sit between the docs and the
    // declaration; they stay where they are
    let mut i = 0;
    while i < above.len() && is_plain(above[i].1) {
        i += 1;
    }
    let end = if i == 0 { decl_line } else { above[i - 1].0 };
    // the docs: `///` lines, and blank lines BETWEEN `///` blocks (several
    // blocks above one declaration are its doc)
    let mut top = None;
    while i < above.len() {
        if is_doc(above[i].1) {
            top = Some(above[i].0);
            i += 1;
        } else if above[i].1.is_empty() {
            let mut j = i;
            while j < above.len() && above[j].1.is_empty() {
                j += 1;
            }
            // an upper block joins only when it carries tags: a file's header
            // doc above the first declaration stays the file's
            let tagged = above[j..]
                .iter()
                .take_while(|(_, l)| is_doc(l))
                .any(|(_, l)| TAGS.iter().any(|tag| l.trim_start_matches('/').trim_start().starts_with(tag)));
            if j < above.len() && is_doc(above[j].1) && tagged {
                i = j;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    top.map(|s| (s, end))
}

const TAGS: &[&str] = &["@param", "@yields", "@example", "@returns"];

/// Split a `///` block into prose and tags.
fn parse_tags(block: &str) -> Tags {
    let lines: Vec<String> = block
        .lines()
        .map(|l| {
            let l = l.trim_start().trim_start_matches("///");
            l.strip_prefix(' ').unwrap_or(l).trim_end().to_string()
        })
        .collect();
    let mut t = Tags::default();
    let mut i = 0;
    let tag_of = |l: &str| TAGS.iter().find(|tag| l == **tag || l.starts_with(&format!("{tag} "))).copied();
    while i < lines.len() {
        let l = &lines[i];
        let Some(tag) = tag_of(l) else {
            t.prose.push(l.clone());
            i += 1;
            continue;
        };
        t.any = true;
        let rest = l[tag.len()..].trim();
        i += 1;
        match tag {
            "@example" => {
                // the code runs to the next tag; a blank line ends it only
                // where its brackets are closed (an example may have paragraphs)
                let mut code: Vec<String> = Vec::new();
                let open = |code: &[String]| {
                    let s = code.join("\n");
                    let count = |c: char| s.chars().filter(|x| *x == c).count() as i64;
                    count('{') - count('}') + count('(') - count(')') + count('[') - count(']')
                };
                while i < lines.len() && tag_of(&lines[i]).is_none() {
                    if lines[i].trim().is_empty() && open(&code) <= 0 {
                        break;
                    }
                    code.push(lines[i].clone());
                    i += 1;
                }
                while code.last().is_some_and(|l| l.trim().is_empty()) {
                    code.pop();
                }
                t.examples.push((rest.to_string(), code.join("\n")));
            }
            _ => {
                // continuation: indented lines that aren't tags
                let mut text = split_name(rest).1.to_string();
                while i < lines.len()
                    && tag_of(&lines[i]).is_none()
                    && lines[i].starts_with("  ")
                    && !lines[i].trim().is_empty()
                {
                    text.push('\n');
                    text.push_str(lines[i].trim());
                    i += 1;
                }
                let (names, _) = split_name(rest);
                match tag {
                    "@param" => {
                        // `posX/Y/Z`: one doc, on the first of the group (the
                        // others sit right below it); `distance/decay` are
                        // separate params and each keeps it
                        let all = expand_names(names);
                        let suffixed = names.split('/').skip(1).all(|p| p.len() <= 2);
                        for (k, n) in all.into_iter().enumerate() {
                            if k == 0 || !suffixed {
                                t.params.push((n, text.clone()));
                            }
                        }
                    }
                    "@yields" | "@returns" => {
                        for n in expand_names(names) {
                            t.yields.push((n.trim_start_matches('$').to_string(), text.clone()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // no blank line left hanging at the ends
    while t.prose.last().is_some_and(|l| l.is_empty()) {
        t.prose.pop();
    }
    while t.prose.first().is_some_and(|l| l.is_empty()) {
        t.prose.remove(0);
    }
    t
}

/// `posX/Y/Z - position` → ("posX/Y/Z", "position"); also `name: text`,
/// `name text`, and the glued `targetX/Y/Z- aim`.
fn split_name(rest: &str) -> (&str, &str) {
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let mut name = &rest[..end];
    let mut text = rest[end..].trim_start();
    if let Some(n) = name.strip_suffix('-').or_else(|| name.strip_suffix(':')) {
        name = n;
    }
    if let Some(t) = text.strip_prefix("- ").or_else(|| text.strip_prefix("— ")).or_else(|| text.strip_prefix(": ")) {
        text = t;
    } else if text == "-" {
        text = "";
    }
    (name, text.trim())
}

/// `posX/Y/Z` → posX, posY, posZ; `distance/decay` → distance, decay;
/// `minDistance/maxDistance` → both; `&el` → el.
fn expand_names(names: &str) -> Vec<String> {
    let parts: Vec<&str> = names.split(['/', ',']).map(str::trim).filter(|s| !s.is_empty()).collect();
    let Some(first) = parts.first() else { return Vec::new() };
    let first = first.trim_start_matches('&');
    let mut out = vec![first.to_string()];
    for p in &parts[1..] {
        if p.len() <= 2 && p.chars().all(|c| c.is_ascii_uppercase()) && first.len() > p.len() {
            // a suffix: posX/Y → posY
            let stem = &first[..first.len() - p.len()];
            out.push(format!("{stem}{p}"));
        } else {
            out.push(p.trim_start_matches('&').to_string());
        }
    }
    out
}

/// A doc without its restated default, or `None` when nothing else is left
/// (or it only repeats the param's name).
fn clean_doc(name: &str, text: &str) -> Option<String> {
    let mut t = text.to_string();
    for pat in [
        r"\s*\((?:default|defaults to)\b[^)]*\)",
        r"[;,.]?\s*\b(?:default|defaults to)\b:?\s+[^\s;,)]+\s*$",
        r"^(?:default|defaults to)\b:?\s+[^\s;,)]+[;,.]?\s*",
    ] {
        let re = regex::Regex::new(&format!("(?i){pat}")).expect("pattern");
        t = re.replace_all(&t, "").to_string();
    }
    let t = t.trim().trim_end_matches([';', ',']).trim().to_string();
    let bare: String = t.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    let name_bare: String = name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    if bare.is_empty() || bare == name_bare {
        None
    } else {
        Some(t)
    }
}

/// A doc block's new text: `kept` lines, as `///` lines at the block's indent.
/// Empty when nothing is kept.
fn render_doc_block(text: &str, start: usize, kept: &[String]) -> String {
    let ind = indent_of(&text[start..]);
    let mut lines: Vec<&String> = kept.iter().collect();
    // collapse runs of blank lines
    lines.dedup_by(|a, b| a.is_empty() && b.is_empty());
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
        .iter()
        .map(|l| if l.is_empty() { format!("{ind}///\n") } else { format!("{ind}/// {l}\n") })
        .collect()
}

fn indent_of(line: &str) -> String {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

fn dedent_lines(code: &str) -> Vec<String> {
    let n = code.lines().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    code.lines().map(|l| if l.len() >= n { l[n..].to_string() } else { l.trim().to_string() }).collect()
}

/// The param's name in a `%form` or primitive param: `type` in
/// `type: $type:string = "x"`, `el` in `&el`.
fn param_name(p: &str) -> String {
    let p = p.trim().trim_start_matches('&').trim_start_matches('$');
    p.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect()
}

/// Where a clause (`%form`, `%binds`, `%exports`) starts in a declaration's
/// text: the keyword followed by a space or a brace, outside comments.
fn find_clause(body: &str, kw: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = body[from..].find(kw) {
        let at = from + i;
        let after = body[at + kw.len()..].chars().next();
        let ls = body[..at].rfind('\n').map(|n| n + 1).unwrap_or(0);
        let in_comment = body[ls..at].contains("//");
        if matches!(after, Some(' ' | '{' | '\t' | '\n')) && !in_comment {
            return Some(at);
        }
        from = at + kw.len();
    }
    None
}

/// The `(` that opens a form's param list: the first one at the form's top
/// level that isn't a capture type's union (`$x:(` …).
fn param_list_open(body: &str, from: usize, to: usize) -> Option<usize> {
    let b = body.as_bytes();
    let mut i = from;
    while i < to {
        match b[i] {
            b'"' => i = skip_string(b, i),
            b'(' => {
                let prev = body[..i].trim_end().chars().last();
                if prev != Some(':') {
                    return Some(i);
                }
                i = matching(body, i, b'(', b')')? + 1;
                continue;
            }
            b'{' => return None,
            _ => {}
        }
        i += 1;
    }
    None
}

/// The index of the bracket closing the one at `open`, strings skipped.
fn matching(body: &str, open: usize, o: u8, c: u8) -> Option<usize> {
    let b = body.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i = skip_string(b, i);
                continue;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                i = body[i..].find('\n').map(|n| i + n).unwrap_or(b.len());
                continue;
            }
            x if x == o => depth += 1,
            x if x == c => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn skip_string(b: &[u8], at: usize) -> usize {
    let mut i = at + 1;
    while i < b.len() && b[i] != b'"' {
        if b[i] == b'\\' {
            i += 1;
        }
        i += 1;
    }
    i + 1
}

/// `word` at a word boundary in `s`.
fn find_word(s: &str, word: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = s[from..].find(word) {
        let at = from + i;
        let next = s[at + word.len()..].chars().next();
        if !next.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return Some(at);
        }
        from = at + word.len();
    }
    None
}

#[cfg(test)]
mod tests;
