//! Whole-document codec: the sibling `StFile` fields (PLAN-148 W4).
//!
//! Normative surface: `docs/edn-spacetime/SPEC.md` §9.
//!
//! # Why this wave exists
//!
//! `FormMatch` is the compile waist, but it is NOT the whole file. Measured on a
//! six-line probe: `scopes: 2`, `imports: 1`, and the CSS declaration
//! `text: $mcpPrompt` produced **zero** `FormMatch`es — CSS declarations live in
//! `StFile.scopes`, not in `matches`.
//!
//! So an EDN *document* is a map whose `:st/forms` member is only one part:
//!
//! ```clojure
//! {:st/imports ["stdlib/__mcp__"]
//!  :st/forms   [(data-inline :name $mcpPrompt :value [:st/expr "0"])]
//!  :st/scopes  [{:selector ".kit-prompt" :decls {"text" "$mcpPrompt"}}]}
//! ```
//!
//! A bare top-level sequence of forms is shorthand for `{:st/forms [...]}`.
//!
//! # What this unblocks
//!
//! `@template &card($t) { … }` parks its BODY in a sibling scope
//! (`ScopeKind::Construct("template")`, selector `@template:card`), while the
//! match keeps only a bare `ComponentBody` presence-marker. The printer refuses
//! to print that marker alone — emitting `{ }` would be a DIFFERENT program (an
//! empty template) that swallows the scopes after it. With the document in hand
//! the body can be recovered from its scope, which is why 84 corpus forms
//! (`template` 67, `editable-block` 9, `editable-mark` 8) were parked on W4
//! rather than counted as printer gaps.

use clojure_reader::edn::Edn;

use crate::parser::ast::{ScopeBlock, ScopeKind, StFile};

/// The scope that holds a construct's body, e.g. `@template:card`.
///
/// Returns `None` when the document has no such scope — the caller then knows
/// the body genuinely is not recoverable and can fail loud rather than guess.
pub fn construct_scope<'a>(
    file: &'a StFile,
    construct: &str,
    name: &str,
) -> Option<&'a ScopeBlock> {
    let wanted = format!("@{construct}:{name}");
    file.scopes.iter().find(|s| {
        matches!(&s.kind, ScopeKind::Construct(c) if c == construct) && s.selector == wanted
    })
}

/// Render a construct body scope back to `.st` body text.
///
/// Declarations use `:` for the CSS surface and `<-` for DOM injection — the
/// distinction `CssDeclaration::is_injection` records. Collapsing the two would
/// silently reroute a reactive binding between `style.setProperty` and
/// `setAttribute` (BUG-091), so it is preserved here.
pub fn scope_body_text(scope: &ScopeBlock) -> String {
    let mut parts: Vec<String> = Vec::new();

    // A body-bearing `Construct` scope carries its markup here — FEAT-119 W2's
    // `reconstruct_template_body_html` (span-subtraction) output, documented as
    // "the SOLE html source for the factory rawBody/builder". A template whose
    // body is `<li class="ad-row">…` has NO css declarations, so without this
    // the scope reads as empty and the markup would be silently deleted.
    if !scope.html.trim().is_empty() {
        parts.push(scope.html.trim().to_string());
    }

    for d in &scope.css_declarations {
        let arrow = if d.is_injection { "<-" } else { ":" };
        parts.push(format!("{} {} {};", d.property, arrow, d.value));
    }

    for n in &scope.nested_scopes {
        let mut inner: Vec<String> = Vec::new();
        for d in &n.css_declarations {
            let arrow = if d.is_injection { "<-" } else { ":" };
            inner.push(format!("{} {} {};", d.property, arrow, d.value));
        }
        parts.push(format!("{} {{ {} }}", n.selector, inner.join(" ")));
    }

    parts.join(" ")
}

/// An EDN document: the forms plus every sibling `StFile` field.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EdnDocument {
    pub imports: Vec<String>,
    pub forms: Vec<crate::syntax::FormMatch>,
    pub scopes: Vec<EdnScope>,
    pub raw_css: Vec<String>,
    /// Construct BODIES: a `@template`'s markup, an `@editable-block`'s run.
    ///
    /// Kept separate from `scopes` (which are CSS regions) because they are a
    /// different thing wearing the same struct: `ScopeKind::Construct("template")`
    /// with selector `@template:<name>`. Omitting them entirely — the previous
    /// behaviour — meant an EDN document could not express a template body at
    /// all, and `print_file` refused with "its payload lives in the sibling
    /// @template:<name> scope" for a sibling the document never carried
    /// (BUG-370).
    pub constructs: Vec<EdnConstruct>,
    /// Top-level markup blocks, carried STRUCTURALLY (BUG-369).
    ///
    /// Not `Vec<String>`: the parser already splits markup into a skeleton plus
    /// an ordered hole list (`HtmlBlockAst`), and a hole is a BINDING. Flattening
    /// that back to text made markup the one plane a structural tool could not
    /// address — patching it would mean regex-on-HTML, which is exactly what the
    /// optics path exists to avoid.
    pub html: Vec<EdnHtmlBlock>,
}

/// A markup block as it appears in an EDN document.
///
/// `skeleton` keeps the parser's sentinel form (`\u{E000}<idx>\u{E001}`) in
/// memory, but is WRITTEN as `[:st/hole N]` markers so the document is readable
/// and the holes are addressable; `holes` are the ordered hole expression
/// sources, each an `[:st/expr "…"]` on the wire.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EdnHtmlBlock {
    pub skeleton: String,
    pub holes: Vec<String>,
    pub injection: crate::parser::ast::HtmlInjection,
}

/// A construct body as it appears in an EDN document.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EdnConstruct {
    /// The construct kind: "template", "editable-block", …
    pub kind: String,
    /// The owning construct's selector, e.g. `@template:card`.
    pub selector: String,
    /// The body's markup, verbatim.
    pub html: String,
    /// CSS declarations inside the body, with the `<-` arrow preserved.
    pub decls: Vec<(String, String, bool)>,
}

/// A scope as it appears in an EDN document.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EdnScope {
    pub selector: String,
    /// `(property, value, is_injection)` — the arrow is carried, not inferred.
    pub decls: Vec<(String, String, bool)>,
}

/// Project an `StFile` into the document shape (the `.st` → EDN direction).
pub fn to_document(file: &StFile) -> EdnDocument {
    EdnDocument {
        imports: file.imports.iter().map(|i| i.path.clone()).collect(),
        forms: file.matches.clone(),
        scopes: file
            .scopes
            .iter()
            // Construct scopes are a construct's BODY, not a CSS region; they are
            // reached through the owning form, not listed as scopes of their own.
            .filter(|s| !matches!(s.kind, ScopeKind::Construct(_)))
            .map(|s| EdnScope {
                selector: s.selector.clone(),
                decls: s
                    .css_declarations
                    .iter()
                    .map(|d| (d.property.clone(), d.value.clone(), d.is_injection))
                    .collect(),
            })
            .collect(),
        constructs: file
            .scopes
            .iter()
            .filter_map(|s| match &s.kind {
                ScopeKind::Construct(kind) => Some(EdnConstruct {
                    kind: kind.clone(),
                    selector: s.selector.clone(),
                    html: s.html.clone(),
                    decls: s
                        .css_declarations
                        .iter()
                        .map(|d| (d.property.clone(), d.value.clone(), d.is_injection))
                        .collect(),
                }),
                _ => None,
            })
            .collect(),
        raw_css: file
            .raw_css_blocks
            .iter()
            .map(|b| b.source.clone())
            .collect(),
        // BUG-369: carry the skeleton/holes split the parser already produced.
        // `to_document` previously set this to `Vec::new()`, so every top-level
        // markup block was silently dropped from the document.
        html: file
            .html_blocks
            .iter()
            .map(|b| EdnHtmlBlock {
                skeleton: b.skeleton.clone(),
                holes: b.holes.clone(),
                injection: b.injection,
            })
            .collect(),
    }
}

/// Render a document to EDN text.
pub fn write_document(doc: &EdnDocument, registry: &crate::syntax::SyntaxRegistry) -> String {
    let mut out = String::from("{");

    if !doc.imports.is_empty() {
        out.push_str(":st/imports [");
        for (i, p) in doc.imports.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&format!("{p:?}"));
        }
        out.push_str("]\n ");
    }

    // BUG-374: a form's SELECTOR is part of the form.
    //
    // `read.rs` has understood `(sel "…" form…)` since the beginning — it is how
    // an EDN document says "these forms live inside this scope". Egress never
    // emitted one, so every scoped macro was written at file level and its
    // selector silently dropped:
    //
    //     .slot { @each($status as $s) { &t(); } }
    //       → edn → st →
    //     @each($status as $s) { &t(); }
    //
    // The behaviour is not attached to anything. `:st/scopes` looks like it
    // should carry this, but it only holds `:decls` — CSS properties — so the
    // scope survived as an empty shell while the form that gave it meaning
    // floated free. Both halves round-tripped; only their RELATIONSHIP was lost,
    // which is why the corpus gate never saw it: each piece is still there.
    //
    // Forms are grouped by selector, preserving first-appearance order, so a
    // scope with several forms emits ONE wrapper rather than one per form.
    out.push_str(":st/forms [");
    let mut idx = 0usize;
    let mut i = 0usize;
    while i < doc.forms.len() {
        let sel = doc.forms[i].selector.clone();
        // Take the run of consecutive forms sharing this selector.
        let mut j = i;
        while j < doc.forms.len() && doc.forms[j].selector == sel {
            j += 1;
        }
        let group = &doc.forms[i..j];

        if idx > 0 {
            out.push('\n');
            out.push_str("            ");
        }
        match &sel {
            Some(s) => {
                out.push_str(&format!("(sel {s:?}"));
                for f in group {
                    out.push(' ');
                    out.push_str(&write_form(f, registry));
                }
                out.push(')');
            }
            None => {
                for (k, f) in group.iter().enumerate() {
                    if k > 0 {
                        out.push('\n');
                        out.push_str("            ");
                    }
                    out.push_str(&write_form(f, registry));
                }
            }
        }
        idx += 1;
        i = j;
    }
    out.push_str("]");

    if !doc.scopes.is_empty() {
        out.push_str("\n :st/scopes [");
        for (i, s) in doc.scopes.iter().enumerate() {
            if i > 0 {
                out.push('\n');
                out.push_str("             ");
            }
            out.push_str(&format!("{{:selector {:?} :decls {{", s.selector));
            for (j, (p, v, inj)) in s.decls.iter().enumerate() {
                if j > 0 {
                    out.push(' ');
                }
                // The arrow is DATA: a `<-` injection is not a CSS property.
                if *inj {
                    out.push_str(&format!("{p:?} [:st/inject {v:?}]"));
                } else {
                    out.push_str(&format!("{p:?} {v:?}"));
                }
            }
            out.push_str("}}");
        }
        out.push(']');
    }

    // Construct bodies (BUG-370): a template's markup, carried so the document
    // is self-contained. Without it `print_file` cannot reconstruct the body and
    // an EDN-authored template is unrenderable.
    if !doc.constructs.is_empty() {
        out.push_str("\n :st/constructs [");
        for (i, c) in doc.constructs.iter().enumerate() {
            if i > 0 {
                out.push('\n');
                out.push_str("                 ");
            }
            out.push_str(&format!(
                "{{:kind {:?} :selector {:?} :html {:?} :decls {{",
                c.kind, c.selector, c.html
            ));
            for (j, (p, v, inj)) in c.decls.iter().enumerate() {
                if j > 0 {
                    out.push(' ');
                }
                if *inj {
                    out.push_str(&format!("{p:?} [:st/inject {v:?}]"));
                } else {
                    out.push_str(&format!("{p:?} {v:?}"));
                }
            }
            out.push_str("}}");
        }
        out.push(']');
    }

    // BUG-369: markup is a PLANE, not a payload. Each block writes its skeleton
    // with `[:st/hole N]` in place of the parser's private-use sentinels, plus
    // the ordered hole expressions, so a reader can address a hole (and the
    // binding it carries) without re-parsing HTML.
    if !doc.html.is_empty() {
        out.push_str("\n :st/html [");
        for (i, b) in doc.html.iter().enumerate() {
            if i > 0 {
                out.push('\n');
                out.push_str("           ");
            }
            match b.injection {
                crate::parser::ast::HtmlInjection::Raw => {
                    out.push_str(&format!("[:st/raw {:?}]", b.skeleton));
                    continue;
                }
                crate::parser::ast::HtmlInjection::Reactive => {
                    out.push_str(&format!("[:st/html {:?}]", b.skeleton));
                    continue;
                }
                crate::parser::ast::HtmlInjection::Markdown => {
                    out.push_str(&format!("[:st/md {:?}]", b.skeleton));
                    continue;
                }
                crate::parser::ast::HtmlInjection::Parsed => {}
            }
            out.push_str(&format!(
                "{{:skeleton {:?} :holes [",
                render_hole_markers(&b.skeleton)
            ));
            for (j, h) in b.holes.iter().enumerate() {
                if j > 0 {
                    out.push(' ');
                }
                out.push_str(&format!("[:st/expr {h:?}]"));
            }
            out.push_str("]}");
        }
        out.push(']');
    }

    if !doc.raw_css.is_empty() {
        out.push_str("\n :st/css [");
        for (i, c) in doc.raw_css.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&format!("[:st/raw {c:?}]"));
        }
        out.push(']');
    }

    out.push('}');
    out
}

/// Rewrite the parser's private-use hole sentinels into readable `[:st/hole N]`
/// markers for the wire (BUG-369).
///
/// `HtmlBlockAst.skeleton` marks holes with `\u{E000}<idx>\u{E001}` — invisible
/// characters chosen precisely because they cannot occur in real markup. That is
/// right for an in-memory skeleton and wrong for a document a human reads and a
/// program patches, so the wire spells the marker out.
fn render_hole_markers(skeleton: &str) -> String {
    use crate::syntax::cst::{HOLE_CLOSE, HOLE_OPEN};
    let mut out = String::with_capacity(skeleton.len());
    let mut rest = skeleton;
    while let Some(start) = rest.find(HOLE_OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + HOLE_OPEN.len_utf8()..];
        match after.find(HOLE_CLOSE) {
            Some(end) => {
                out.push_str(&format!("[:st/hole {}]", &after[..end]));
                rest = &after[end + HOLE_CLOSE.len_utf8()..];
            }
            // An unterminated sentinel means the skeleton is malformed. Emit the
            // remainder verbatim rather than silently truncating it — the reader
            // then fails loud on text it cannot map back.
            None => {
                out.push_str(after);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Inverse of [`render_hole_markers`]: `[:st/hole N]` → the parser's sentinels,
/// so a document read back yields the exact skeleton the parser produced.
pub fn parse_hole_markers(wire: &str) -> String {
    use crate::syntax::cst::{HOLE_CLOSE, HOLE_OPEN};
    let mut out = String::with_capacity(wire.len());
    let mut rest = wire;
    while let Some(start) = rest.find("[:st/hole ") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "[:st/hole ".len()..];
        match after.find(']') {
            Some(end) => {
                out.push(HOLE_OPEN);
                out.push_str(after[..end].trim());
                out.push(HOLE_CLOSE);
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// One `FormMatch` as an EDN form: `(macro-name :capture value …)`.
///
/// Capture ORDER comes from the pattern, so the output is deterministic and
/// matches what the reader expects positionally.
fn write_form(m: &crate::syntax::FormMatch, registry: &crate::syntax::SyntaxRegistry) -> String {
    let name = m.matched_macro.as_deref().unwrap_or(&m.macro_name);
    let mut out = format!("({name}");

    let ordered: Vec<String> = registry
        .get_by_macro_name(name)
        .map(|f| pattern_capture_order(&f.form))
        .unwrap_or_default();

    let mut seen = std::collections::HashSet::new();
    for k in ordered {
        if let Some(v) = m.captures.get(&k) {
            out.push_str(&format!(" :{k} {}", edn_text(&crate::edn::to_edn_value(v))));
            seen.insert(k);
        }
    }
    // Anything the pattern did not name (defensive: keeps the projection total).
    let mut rest: Vec<&String> = m.captures.keys().filter(|k| !seen.contains(*k)).collect();
    rest.sort();
    for k in rest {
        out.push_str(&format!(
            " :{k} {}",
            edn_text(&crate::edn::to_edn_value(&m.captures[k]))
        ));
    }

    out.push(')');
    out
}

fn pattern_capture_order(form: &crate::parser::meta_ast::FormClause) -> Vec<String> {
    use crate::parser::meta_ast::FormInlineElement as F;
    let mut out = Vec::new();

    fn walk(el: &F, out: &mut Vec<String>) {
        match el {
            F::Capture(c, _) => out.push(c.var_name.clone()),
            F::Comparison { capture, .. } => out.push(capture.var_name.clone()),
            F::Group { elements, .. } => elements.iter().for_each(|e| walk(e, out)),
            F::KeywordBlock { body_params, .. }
            | F::PseudoSelector { body_params, .. }
            | F::PseudoClass { body_params, .. } => {
                for p in body_params {
                    p.elements.iter().for_each(|e| walk(e, out));
                }
            }
            F::Literal(_) => {}
        }
    }

    form.inline_elements.iter().for_each(|e| walk(e, &mut out));
    for p in &form.params {
        p.elements.iter().for_each(|e| walk(e, &mut out));
    }
    form.post_arg_inline.iter().for_each(|e| walk(e, &mut out));
    for p in &form.body_params {
        p.elements.iter().for_each(|e| walk(e, &mut out));
    }
    out
}

/// Serialise an `Edn` value back to text.
fn edn_text(e: &Edn<'_>) -> String {
    match e {
        Edn::Nil => "nil".into(),
        Edn::Bool(b) => b.to_string(),
        Edn::Int(i) => i.to_string(),
        Edn::Double(d) => f64::from(*d).to_string(),
        Edn::Str(s) => format!("{s:?}"),
        Edn::Symbol(s) => (*s).to_string(),
        Edn::Key(k) => format!(":{k}"),
        Edn::Vector(items) | Edn::List(items) => {
            let inner: Vec<String> = items.iter().map(edn_text).collect();
            format!("[{}]", inner.join(" "))
        }
        Edn::Map(pairs) => {
            let inner: Vec<String> = pairs
                .iter()
                .map(|(k, v)| format!("{} {}", edn_text(k), edn_text(v)))
                .collect();
            format!("{{{}}}", inner.join(" "))
        }
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::STDLIB_REGISTRY;

    #[test]
    fn template_body_lives_in_a_sibling_scope() {
        // The fact that parks 84 corpus forms on this wave: the match keeps only
        // a ComponentBody marker; the body is a scope named `@template:<name>`.
        let ast = spacetime_parse("@template &card($title) {\n  .card { color: red; }\n}\n");
        let scope =
            construct_scope(&ast, "template", "card").expect("the template body scope must exist");
        assert!(matches!(&scope.kind, ScopeKind::Construct(c) if c == "template"));
        assert_eq!(scope.selector, "@template:card");

        let body = scope_body_text(scope);
        assert!(body.contains(".card"), "{body}");
        assert!(body.contains("color : red;"), "{body}");
    }

    #[test]
    fn document_captures_what_matches_alone_cannot() {
        // A CSS declaration produces ZERO FormMatches — it lives in `scopes`.
        let ast =
            spacetime_parse("@import \"stdlib/__mcp__\";\n.kit-prompt { text: $mcpPrompt; }\n");
        let doc = to_document(&ast);

        assert_eq!(doc.imports, vec!["stdlib/__mcp__".to_string()]);
        assert!(
            doc.scopes.iter().any(|s| s.selector == ".kit-prompt"),
            "the scope carrying `text:` must survive into the document: {:?}",
            doc.scopes
        );
    }

    #[test]
    fn construct_scopes_are_not_listed_as_css_scopes() {
        // A template body is reached THROUGH its form, not as a scope of its own;
        // listing it twice would emit the body at file root as well.
        let ast = spacetime_parse("@template &card($title) {\n  .card { color: red; }\n}\n");
        let doc = to_document(&ast);
        assert!(
            !doc.scopes
                .iter()
                .any(|s| s.selector.starts_with("@template:")),
            "construct scopes must not appear as CSS scopes: {:?}",
            doc.scopes
        );
    }

    #[test]
    fn document_writes_readable_edn() {
        let ast =
            spacetime_parse("@import \"stdlib/__mcp__\";\n.kit-prompt { text: $mcpPrompt; }\n");
        let doc = to_document(&ast);
        let text = write_document(&doc, &STDLIB_REGISTRY);

        assert!(text.contains(":st/imports"), "{text}");
        assert!(text.contains(":st/scopes"), "{text}");
        // It must be READABLE EDN, not merely printable text.
        clojure_reader::edn::read_string(&text)
            .unwrap_or_else(|e| panic!("document is not valid EDN: {e:?}\n{text}"));
    }

    #[test]
    fn injection_arrow_survives_the_projection() {
        // `<-` (DOM injection) and `:` (CSS property) route differently
        // (BUG-091); collapsing them would silently change behaviour.
        let ast = spacetime_parse(".a { text <- $x; }\n");
        let doc = to_document(&ast);
        let injected = doc
            .scopes
            .iter()
            .flat_map(|s| s.decls.iter())
            .any(|(_, _, inj)| *inj);
        assert!(injected, "the `<-` arrow must be carried: {:?}", doc.scopes);
    }

    #[test]
    fn markup_is_carried_structurally_with_addressable_holes() {
        // BUG-369: `to_document` used to set `html: Vec::new()`, so top-level
        // markup vanished from the document entirely. It must arrive as a
        // skeleton + ordered holes — the split the parser already made.
        let ast =
            spacetime_parse("<article class=\"card\" data-id=\"`$c.id`\">`$c.title`</article>\n");
        assert!(
            !ast.html_blocks.is_empty(),
            "fixture must produce a top-level html block"
        );

        let doc = to_document(&ast);
        assert_eq!(doc.html.len(), 1, "the block must reach the document");

        let block = &doc.html[0];
        assert_eq!(
            block.holes.len(),
            2,
            "both backtick holes are carried: {:?}",
            block.holes
        );

        // A hole is a BINDING, and the whole point of carrying it structurally is
        // that the document can answer "what does this markup read?" WITHOUT
        // re-parsing HTML.
        assert!(
            block.holes.iter().any(|h| h.contains("$c.id")),
            "hole expressions are recoverable: {:?}",
            block.holes
        );

        // The skeleton keeps the markup MINUS the holes, so the two planes are
        // separable rather than interleaved in one blob.
        assert!(
            block.skeleton.contains("article") && !block.skeleton.contains("$c.id"),
            "skeleton is markup with holes lifted out: {:?}",
            block.skeleton
        );
    }

    #[test]
    fn hole_markers_round_trip_through_the_wire_spelling() {
        // On the wire a hole is `[:st/hole N]`, not the parser's invisible
        // private-use sentinels: a document is read and PATCHED by programs and
        // people, and neither can address a character they cannot see.
        use crate::syntax::cst::{HOLE_CLOSE, HOLE_OPEN};
        let skeleton = format!("<p>{HOLE_OPEN}0{HOLE_CLOSE} and {HOLE_OPEN}1{HOLE_CLOSE}</p>");

        let wire = render_hole_markers(&skeleton);
        assert_eq!(wire, "<p>[:st/hole 0] and [:st/hole 1]</p>");

        assert_eq!(
            parse_hole_markers(&wire),
            skeleton,
            "the wire spelling must map back to the exact parser skeleton"
        );
    }

    #[test]
    fn document_writes_the_html_plane() {
        let ast = spacetime_parse("<p class=\"x\">`$msg`</p>\n");
        let doc = to_document(&ast);
        let registry = crate::syntax::SyntaxRegistry::default();
        let text = write_document(&doc, &registry);

        assert!(text.contains(":st/html"), "html plane is written: {text}");
        assert!(
            text.contains("[:st/hole 0]"),
            "holes are spelled on the wire: {text}"
        );
        assert!(
            text.contains("[:st/expr \"$msg\"]"),
            "hole expressions ride as :st/expr: {text}"
        );
    }

    fn spacetime_parse(src: &str) -> crate::parser::ast::StFile {
        crate::parse(src).expect("fixture must parse")
    }
}
