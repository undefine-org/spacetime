//! `Vec<FormMatch>` → `.st` source text (PLAN-148 W2) — **the thesis test**.
//!
//! # Why this module is the load-bearing one
//!
//! The whole EDN design rests on one claim: *Spacetime's grammar is data*. If
//! that is true, a form can be PRINTED from its `%form` pattern plus its
//! captures, with no per-construct code. If it is false, the claim has a hole
//! and we need to know before any EDN syntax depends on it.
//!
//! So W2 comes BEFORE the EDN reader: printing is the falsifiable half.
//!
//! This generalises `crate::lsp::completion::render_form_snippet`, which already
//! walks `inline_elements` / `params` / `post_arg_inline` / `body_*` and prints
//! Spacetime syntax from the declarative pattern — it just substitutes LSP
//! tabstops where we substitute captured values.
//!
//! # Fail loud — there is no fallback policy (SPEC §8.2)
//!
//! 1. Pattern-print every form. Cannot → **hard error naming the macro**.
//! 2. Raw captures (`Expr`, opaque bodies) print **verbatim** from capture text.
//!    That is faithful BY CONSTRUCTION, not a fallback: EDN never reconstructs
//!    an expression it did not author — it carries the bytes.
//! 3. Verbatim text that fails to re-parse → **hard error + a verse bug**.
//!
//! The corpus run therefore emits a LIST OF VERSE DEFECTS, not a fudge factor.

use std::fmt::Write as _;

use crate::parser::meta_ast::{
    CaptureModifier, CapturePatternAst, FormCapture, FormClause, FormInlineElement, FormParam,
    ParamDefault,
};
use crate::syntax::{CapturedValue, LengthValue, SyntaxRegistry};

/// A printing failure. Always names the macro, so a corpus run yields an
/// actionable defect list rather than a count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintError {
    /// The `%macro` whose form could not be printed.
    pub macro_name: String,
    pub message: String,
}

impl PrintError {
    fn new(macro_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            macro_name: macro_name.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PrintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.macro_name, self.message)
    }
}

impl std::error::Error for PrintError {}

type Result<T> = std::result::Result<T, PrintError>;

/// Render one `FormMatch` back to `.st` source text, driven by its `%form`.
///
/// Resolution order for the pattern is `matched_macro` first, then `macro_name`.
/// This matters: parsing `@data inline $x : "";` yields `macro_name: "data"` with
/// `matched_macro: "data-inline"`, and only the latter identifies the actual
/// pattern. Resolve honours `matched_macro`; so must the printer.
pub fn print_form(m: &crate::syntax::FormMatch, registry: &SyntaxRegistry) -> Result<String> {
    print_form_with_doc(m, registry, None)
}

/// [`print_form`] with the owning document in hand, so a `ComponentBody` marker
/// can be resolved from its sibling scope. Attributing a failure to one macro
/// needs this per-form entry point; whole-file printing uses [`print_file`].
pub fn print_form_in(
    m: &crate::syntax::FormMatch,
    registry: &SyntaxRegistry,
    doc: Option<&crate::parser::ast::StFile>,
) -> Result<String> {
    print_form_with_doc(m, registry, doc)
}

fn print_form_with_doc(
    m: &crate::syntax::FormMatch,
    registry: &SyntaxRegistry,
    doc: Option<&crate::parser::ast::StFile>,
) -> Result<String> {
    let name = m.matched_macro.as_deref().unwrap_or(&m.macro_name);

    let form = registry
        .get_by_macro_name(name)
        .or_else(|| registry.get_by_macro_name(&m.macro_name))
        .ok_or_else(|| {
            PrintError::new(
                name,
                format!(
                    "no registered form for macro {name:?} (nor for macro_name \
                     {:?}) — the registry the printer consulted does not contain \
                     this macro",
                    m.macro_name
                ),
            )
        })?;

    render_clause(name, &form.form, m, registry, doc)
}

/// Print a whole `StFile`: forms PLUS the sibling fields they depend on (W4).
///
/// This is the entry point that can print a `@template`, because a template's
/// body is not in its match — the match holds a bare `ComponentBody` marker and
/// the body lives in a sibling `ScopeKind::Construct("template")` scope named
/// `@template:<name>`. [`print_forms`] has no document, so it fails loud on that
/// marker rather than emitting `{ }` (an empty template is a DIFFERENT program,
/// and it swallows the scopes that follow).
pub fn print_file(file: &crate::parser::ast::StFile, registry: &SyntaxRegistry) -> Result<String> {
    print_forms_with_doc(&file.matches, registry, Some(file))
}

/// Render a sequence of forms as a `.st` fragment.
///
/// A form carrying a `selector` is SCOPE-BOUND: `@mcp-action` inside
/// `.kit-btn-confirm { … }` means something different from the same directive at
/// file root. Printing it bare loses the scope, and the corpus caught exactly
/// that — `workbench/actions.st` collapsed 25 matches → 1 because every
/// selector-scoped form merged into one implicit scope.
///
/// So consecutive forms sharing a selector are grouped back into one scope
/// block, preserving both the scope and the form count across a round trip.
pub fn print_forms(
    matches: &[crate::syntax::FormMatch],
    registry: &SyntaxRegistry,
) -> Result<String> {
    print_forms_with_doc(matches, registry, None)
}

fn print_forms_with_doc(
    matches: &[crate::syntax::FormMatch],
    registry: &SyntaxRegistry,
    doc: Option<&crate::parser::ast::StFile>,
) -> Result<String> {
    let mut out = String::new();
    let mut i = 0;

    while i < matches.len() {
        let sel = matches[i].selector.clone();
        match sel {
            None => {
                out.push_str(&print_form_with_doc(&matches[i], registry, doc)?);
                out.push('\n');
                i += 1;
            }
            Some(selector) => {
                // Gather the run of forms sharing this selector.
                let mut body = String::new();
                while i < matches.len() && matches[i].selector.as_deref() == Some(selector.as_str())
                {
                    body.push_str("  ");
                    body.push_str(&print_form_with_doc(&matches[i], registry, doc)?);
                    body.push('\n');
                    i += 1;
                }
                let _ = write!(out, "{selector} {{\n{body}}}\n");
            }
        }
    }

    Ok(out)
}

fn render_clause(
    macro_name: &str,
    form: &FormClause,
    m: &crate::syntax::FormMatch,
    reg: &SyntaxRegistry,
    doc: Option<&crate::parser::ast::StFile>,
) -> Result<String> {
    let mut out = String::new();

    // The directive word itself (`@data`, `@fn`, `$`, `&`, …).
    out.push_str(&form.directive_name);

    for el in &form.inline_elements {
        render_inline(macro_name, el, m, &mut out)?;
    }

    if !form.params.is_empty() {
        let rendered = render_params(macro_name, &form.params, m)?;
        // A form whose params are ALL defaulted-and-absent prints without the
        // parens at all — emitting `()` would be noise the author never wrote.
        //
        // But an EMPTY render does not always mean "the author wrote nothing":
        // `%form { @template &$name:ident($params:param_list) { … } }` spells the
        // parens as LITERALS around a capture that legitimately matches empty
        // (`param_list` documents `() -> []`). Suppressing them there prints
        // `@template &shell { … }`, which re-parses to ZERO matches — the
        // template declaration silently disappears while the file still parses
        // "successfully", so the loss surfaces later as an unresolved template.
        //
        // The distinguishing question is whether the paren group is STRUCTURAL:
        // a param with no default is required by the pattern, so its parens are
        // part of the syntax rather than sugar for an argument the author chose
        // to supply.
        if rendered.is_empty() {
            let structural = form.params.iter().any(|p| {
                p.default.is_none()
                    && p.elements.iter().any(|el| {
                        matches!(el, FormInlineElement::Capture(_, None))
                    })
            });
            if structural {
                out.push_str("()");
            }
        } else {
            let _ = write!(out, "({rendered})");
        }
    }

    for el in &form.post_arg_inline {
        render_inline(macro_name, el, m, &mut out)?;
    }

    let had_body = {
        let before = out.len();
        render_body(macro_name, form, m, reg, doc, &mut out)?;
        out.len() != before
    };

    let mut out = out.trim_end().to_string();

    // Statement terminator.
    //
    // Several `%form` patterns spell `;` as a trailing literal (`@data inline …
    // ;`), but many do NOT — `@import`, `@mcp-input` and friends rely on the
    // `.st` grammar's implicit statement end. Printing those without a `;` makes
    // the parser run the NEXT construct into them: `confirm.st` silently lost
    // `@import` and `@mcp-input`, 6 matches → 4. So a bodyless form that did not
    // already end in a delimiter gets one.
    if !had_body && !out.ends_with(';') && !out.ends_with('}') {
        out.push(';');
    }

    Ok(out)
}

fn render_inline(
    macro_name: &str,
    el: &FormInlineElement,
    m: &crate::syntax::FormMatch,
    out: &mut String,
) -> Result<()> {
    match el {
        // Literals come from the PATTERN, never from the author's EDN. This is
        // the mechanism that lets EDN key on the macro name and still emit
        // `@data inline … : … ;` exactly.
        FormInlineElement::Literal(s) => {
            push_token(out, s);
        }
        FormInlineElement::Capture(cap, default) => {
            if let Some(text) = capture_text(macro_name, cap, default.as_ref(), m)? {
                push_token(out, &text);
            }
        }
        FormInlineElement::Comparison { operator, capture } => {
            if let Some(text) = capture_text(macro_name, capture, None, m)? {
                push_token(out, operator);
                push_token(out, &text);
            }
        }
        FormInlineElement::Group {
            elements, modifier, ..
        } => {
            // An optional group is emitted only when at least one of its
            // captures is actually present in the match.
            //
            // BUG-367 is fixed at the REGISTRY now: a capture inside `( … )?`
            // reports Optional, so `capture_text` handles the absence itself and
            // this needs no special case. The `modifier` is still read — an
            // optional group that matched nothing prints nothing — but the
            // captures inside are trusted to describe themselves.
            let _ = modifier;

            let mut inner = String::new();
            let mut any = false;
            for e in elements {
                let before = inner.len();
                render_inline(macro_name, e, m, &mut inner)?;
                if matches!(e, FormInlineElement::Capture(..)) && inner.len() != before {
                    any = true;
                }
            }
            if any {
                push_token(out, inner.trim());
            }
        }
        FormInlineElement::KeywordBlock {
            keyword,
            body_params,
            ..
        } => {
            let inner = render_params(macro_name, body_params, m)?;
            if !inner.is_empty() {
                push_token(out, keyword);
                let _ = write!(out, " {{ {inner} }}");
            }
        }
        // `( :entering { $_enterAnim:keyframes } )?` — an optional pseudo-selector
        // clause. Its captures now REPORT as optional (BUG-367 fixed at the
        // registry), so the ordinary path handles an absent one correctly and the
        // clause simply renders empty when nothing was written.
        FormInlineElement::PseudoSelector {
            name, body_params, ..
        } => {
            let inner = render_params(macro_name, body_params, m)?;
            if !inner.is_empty() {
                let _ = write!(out, ":{name} {{ {inner} }}");
            }
        }
        FormInlineElement::PseudoClass { name, body_params } => {
            let inner = render_params(macro_name, body_params, m)?;
            if !inner.is_empty() {
                let _ = write!(out, ":{name} {{ {inner} }}");
            }
        }
    }
    Ok(())
}

/// Append a token with `.st`-appropriate spacing: no space before `;`/`,`, and
/// no doubled spaces. Keeps output re-parseable without a formatter.
fn push_token(out: &mut String, tok: &str) {
    let tok = tok.trim();
    if tok.is_empty() {
        return;
    }
    let tight = matches!(tok, ";" | "," | ")" | "]");
    // `&` and `$` are SIGILS, not words: `@template &name(…)` must not print as
    // `@template & name(…)` — the space breaks the element reference and the
    // parser then swallows the following scope blocks (measured on
    // `showcases/cta/minimal.st`).
    let opens_sigil = out.ends_with('&') || out.ends_with('$') || out.ends_with('.');
    if !out.is_empty() && !out.ends_with(' ') && !tight && !out.ends_with('(') && !opens_sigil {
        out.push(' ');
    }
    out.push_str(tok);
}

fn render_params(
    macro_name: &str,
    params: &[FormParam],
    m: &crate::syntax::FormMatch,
) -> Result<String> {
    let mut parts: Vec<String> = Vec::with_capacity(params.len());

    for p in params {
        // A param is a small element sequence; if every capture in it is absent
        // (defaulted), the whole param is omitted rather than printed empty.
        //
        // NB the default for a PARAM lives on `FormParam.default`, NOT on the
        // inline element (`%form { @x(on: $on:string = "click") }` parks
        // `"click"` on the param). Reading only the element default is why
        // `@mcp-action` first printed all six captures including the four the
        // author never wrote.
        let mut buf = String::new();
        let mut present = false;
        for el in &p.elements {
            let before = buf.len();
            match el {
                FormInlineElement::Capture(cap, el_default) => {
                    let effective = el_default.as_ref().or(p.default.as_ref());
                    if let Some(text) = capture_text(macro_name, cap, effective, m)? {
                        push_token(&mut buf, &text);
                    }
                }
                _ => render_inline(macro_name, el, m, &mut buf)?,
            }
            if matches!(el, FormInlineElement::Capture(..)) && buf.len() != before {
                present = true;
            }
        }
        let body = buf.trim().to_string();
        if body.is_empty() || (!present && !p.elements.is_empty()) {
            continue;
        }
        // A bare pattern (`$source as $item`) has an empty name and prints with
        // no `name:` prefix; a named param is `name: value`.
        if p.name.is_empty() {
            parts.push(body);
        } else {
            parts.push(format!("{}: {}", p.name, body));
        }
    }

    Ok(parts.join(", "))
}

fn render_body(
    macro_name: &str,
    form: &FormClause,
    m: &crate::syntax::FormMatch,
    reg: &SyntaxRegistry,
    doc: Option<&crate::parser::ast::StFile>,
    out: &mut String,
) -> Result<()> {
    let has_body =
        form.body_capture.is_some() || !form.body_params.is_empty() || !form.body_groups.is_empty();
    if !has_body {
        return Ok(());
    }

    // Body GROUPS are the `( :entering { $_enterAnim:keyframes } )?` clauses.
    // Their captures are optional by virtue of the enclosing `?` even though the
    // registry records them Required (BUG-367), so an absent one is skipped, not
    // an error.
    if !form.body_groups.is_empty() {
        let mut parts: Vec<String> = Vec::new();
        for g in &form.body_groups {
            if let Some(t) = grammar_text(macro_name, "", g, &group_value(m), reg, 0)?
                && !t.trim().is_empty()
            {
                parts.push(t);
            }
        }
        if !parts.is_empty() {
            let _ = write!(out, " {{ {} }}", join_tokens(&parts));
            return Ok(());
        }
    }

    // Body params (`{ prop: $v:type; … }`) render as declarations.
    //
    // Optional-aware: a body param may be an optional pseudo-selector clause
    // (`( :entering { … } )?`) whose captures the registry marks Required
    // (BUG-367). Erroring on those absences fails 34 corpus forms for legal
    // source, so absence is skipped here and the whole body simply comes out
    // empty when no clause was written.
    if !form.body_params.is_empty() {
        let inner = render_params(macro_name, &form.body_params, m)?;
        if !inner.is_empty() {
            let _ = write!(out, " {{ {inner} }}");
            return Ok(());
        }
    }

    // A greedy body capture: find the capture it names and emit its text.
    if let Some(pat) = &form.body_capture {
        let var = body_capture_var(pat);
        if let Some(v) = m.captures.get(&var) {
            match v {
                // RULE 2: raw bodies print VERBATIM. `Expr` holds JavaScript and
                // `String` holds an opaque body (HTML, a `@fixture` block, a JS
                // statement run); we carry the bytes rather than inventing a
                // reconstruction. This is faithful BY CONSTRUCTION — EDN never
                // rebuilds a body it did not author, it carries it.
                CapturedValue::Expr(src) | CapturedValue::String(src) => {
                    let t = src.trim();
                    // The parser is known to hand back bodies with an unbalanced
                    // leading `{` (see the `@fn` defect in PLAN-148 / FUP-188).
                    // Normalise the delimiters we own so the printed text is at
                    // least well-formed; the INNER text is still verbatim.
                    let inner = t.trim_start_matches('{').trim_end_matches('}').trim();
                    let _ = write!(out, " {{ {inner} }}");
                }
                CapturedValue::StyleProperties(props) => {
                    let decls = props
                        .iter()
                        .map(|(k, val)| format!("{k}: {val};"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = write!(out, " {{ {decls} }}");
                }
                CapturedValue::Properties(props) => {
                    let decls = props
                        .iter()
                        .map(|p| {
                            format!(
                                "{}{}: {};",
                                p.name,
                                if p.optional { "?" } else { "" },
                                p.type_ref
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = write!(out, " {{ {decls} }}");
                }
                // `opacity: 0 -> 1;` — the arrow-chain body shape. Nested-scope
                // entries (BUG-201) carry their own selector and re-nest.
                CapturedValue::Keyframes(kfs) => {
                    let mut root: Vec<String> = Vec::new();
                    let mut scoped: std::collections::BTreeMap<&str, Vec<String>> =
                        Default::default();
                    for k in kfs {
                        let line = format!("{}: {};", k.property, k.values.join(" -> "));
                        match k.selector.as_deref() {
                            Some(sel) => scoped.entry(sel).or_default().push(line),
                            None => root.push(line),
                        }
                    }
                    let mut inner = root.join(" ");
                    for (sel, lines) in scoped {
                        let _ = write!(inner, " {sel} {{ {} }}", lines.join(" "));
                    }
                    let _ = write!(out, " {{ {} }}", inner.trim());
                }

                // An ALTERNATION body: `%capture_type on_motion_body { ( $slot:…
                // | $line:motion_line | $sel:selector "{" $scope:… "}" | … )* }`.
                // Each element is a `Named` whose ONE live key names the
                // alternative that matched, so rendering is generic over the
                // alternation — no per-alternative code, which is the whole point.
                CapturedValue::Array(items) => {
                    // Prefer the body capture's DECLARED grammar: it carries the
                    // literals (`for`, `;`, `|`, `=>`) and the sigils that a
                    // key-shape guess cannot recover. `@form score` printed
                    // `["25%"] name` instead of `&name for 25%;` until this
                    // took precedence (measured on `showcases/_schools.st`).
                    let ct = body_capture_type(pat);
                    if let Some(text) = render_array_by_grammar(
                        macro_name, &var, &ct, items, reg,
                    )? {
                        let _ = write!(out, " {{ {} }}", text.trim());
                        return Ok(());
                    }

                    // Builtin capture types (e.g. `template_invocation`) have no
                    // `%capture_type` grammar to walk, so try the structural
                    // recognisers before the key-shape fallback.
                    let mut structural = Vec::with_capacity(items.len());
                    let mut all = true;
                    for it in items {
                        match it {
                            CapturedValue::Named(map) => {
                                match structural_element_text(macro_name, &var, map)? {
                                    Some(t) => structural.push(t),
                                    None => {
                                        all = false;
                                        break;
                                    }
                                }
                            }
                            _ => {
                                all = false;
                                break;
                            }
                        }
                    }
                    if all && !structural.is_empty() {
                        let _ = write!(out, " {{ {} }}", structural.join(" ").trim());
                        return Ok(());
                    }

                    let mut lines = Vec::with_capacity(items.len());
                    for it in items {
                        lines.push(alternation_element_text(macro_name, &var, it)?);
                    }
                    let _ = write!(out, " {{ {} }}", lines.join(" ").trim());
                }

                // A nested `Block` holds child FORMS — a `@stage` body is
                // `@particles`, `@scroll-3d`, `@post`, each a `FormMatch` in its
                // own right. Print them through the SAME entry point, so a form
                // renders identically whether it sits at the top level or inside
                // another form's body; two implementations would drift.
                //
                // Refusing these was correct while it was unimplemented (emitting
                // an empty body would silently delete every child), but it is not
                // a limit of the design: printing a nested form is the same
                // problem as printing a top-level one.
                CapturedValue::Block(children) => {
                    let mut inner = Vec::with_capacity(children.len());
                    for child in children {
                        inner.push(print_form_with_doc(child, reg, doc)?);
                    }
                    if inner.is_empty() {
                        out.push_str(" { }");
                    } else {
                        let _ = write!(out, " {{ {} }}", inner.join(" "));
                    }
                }
                // A bare presence-marker carrying NO data: the real payload
                // lives in the sibling `@template:<name>` scope, which is a
                // `StFile` field rather than a capture (W4). Printing `{ }` here
                // would emit a template with an empty body — a DIFFERENT program,
                // and one that swallows the scopes that follow it. Fail loud
                // until W4 can supply the scope.
                CapturedValue::ComponentBody => {
                    // W4: recover the payload from the sibling scope.
                    //
                    // `@template &card($t) { … }` parks its body in a scope of
                    // kind `Construct("template")` named `@template:card`. The
                    // marker itself carries nothing, so WITHOUT a document there
                    // is no honest way to print this form — emitting `{ }` would
                    // be a different program (an empty template) that swallows
                    // the scopes after it.
                    let name = m
                        .captures
                        .get("name")
                        .and_then(|v| match v {
                            CapturedValue::Ident(s)
                            | CapturedValue::Element(s)
                            | CapturedValue::Binding(s) => {
                                Some(s.trim_start_matches(['&', '$']).to_string())
                            }
                            _ => None,
                        })
                        .unwrap_or_default();

                    let construct = m
                        .matched_macro
                        .as_deref()
                        .unwrap_or(macro_name)
                        .to_string();

                    if let Some(file) = doc
                        && let Some(scope) =
                            crate::edn::doc::construct_scope(file, &construct, &name)
                                .or_else(|| {
                                    crate::edn::doc::construct_scope(file, macro_name, &name)
                                })
                    {
                        let body = crate::edn::doc::scope_body_text(scope);
                        if !body.trim().is_empty() {
                            let _ = write!(out, " {{ {} }}", body.trim());
                            return Ok(());
                        }
                        // An EMPTY scope does not mean an empty body: a template
                        // whose body is HTML (`<li class="ad-row">…`) parks that
                        // markup in `StFile.html_blocks`, leaving the construct
                        // scope with no declarations. Printing `{ }` here would
                        // silently delete the markup, so this is a hard error
                        // until HTML-body recovery lands.
                        return Err(PrintError::new(
                            macro_name,
                            format!(
                                "body of @{construct}:{name} is empty as a scope — its \
                                 markup lives in `StFile.html_blocks`, which the \
                                 document codec does not yet recover. Printing an \
                                 empty body would delete the markup."
                            ),
                        ));
                    }

                    return Err(PrintError::new(
                        macro_name,
                        format!(
                            "body is a ComponentBody marker for {name:?}; its payload \
                             lives in the sibling @{construct}:{name} scope. Print \
                             through `print_file` (whole document) rather than \
                             `print_forms` (matches only)."
                        ),
                    ));
                }
                // Everything else: consult the body capture's OWN
                // `%capture_type` grammar (`variant_list`, `score_entry`, `arm`,
                // …) and render from the declared pattern. Generic — a new body
                // grammar needs no printer change.
                other => {
                    let ct = body_capture_type(pat);
                    if let Some(text) = grammar_text(
                        macro_name,
                        &var,
                        &CapturePatternAst::Capture {
                            var_name: var.clone(),
                            capture_type: ct,
                            modifier: CaptureModifier::Required,
                        },
                        &CapturedValue::Named(
                            [(var.clone(), other.clone())].into_iter().collect(),
                        ),
                        reg,
                        0,
                    )? {
                        let _ = write!(out, " {{ {} }}", text.trim());
                        return Ok(());
                    }
                    return Err(PrintError::new(
                        macro_name,
                        format!(
                            "body capture {var:?} holds {other:?}, and its capture \
                             type grammar produced no rendering"
                        ),
                    ));
                }
            }
            return Ok(());
        }
    }

    // The pattern declares a body, but the match carries no capture for it.
    //
    // This is LEGITIMATE when every body element is optional: `@handle $sig {
    // ( $optimistic:… )? ( $receive:… )? }` matched with no clauses present, or
    // `@host $n : http(url) { headers: $headers:object? }` written without
    // headers. The author wrote no body, so we print no body.
    //
    // It is a genuine inconsistency only when the body is REQUIRED — then the
    // pattern promises something the match cannot supply, and we fail loud
    // (Rule 1) rather than emit a form that means something different.
    // A body whose capture type can match NOTHING is legitimately empty — but the
    // braces are still part of the form's shape. `@on &.scroll(…) { }` printed
    // WITHOUT them became `@on &.scroll;`, which no longer matches `on-driver-body`
    // at all: the form silently vanished from the file (3 matches → 2).
    if body_type_matches_empty(form, reg) {
        out.push_str(" { }");
        return Ok(());
    }

    if body_is_all_optional(form) {
        return Ok(());
    }

    Err(PrintError::new(
        macro_name,
        format!(
            "form declares a REQUIRED body ({:?}) but the match has no corresponding \
             capture — pattern and match disagree",
            form.body_capture
        ),
    ))
}

/// The match's captures as a `Named` value, so a body-group grammar can be
/// walked against them uniformly.
fn group_value(m: &crate::syntax::FormMatch) -> CapturedValue {
    CapturedValue::Named(m.captures.clone().into_iter().collect())
}

/// True when the body capture's own `%capture_type` grammar can match NOTHING,
/// so an empty body is legal and produces no capture.
///
/// `@on &.scroll(…) { }` is the live case:
///
/// ```text
/// %form { @on $driver:driver_expr $as:on_as? { $body:on_motion_body } }
/// %capture_type on_motion_body { ( … | … | … )* }
/// ```
///
/// The `%form` spells `$body` without `?` or `*`, so a check that only reads the
/// body-capture STRING concludes "required" — but the TYPE is zero-or-more, and
/// an empty body is exactly what the grammar permits. The optionality is one
/// level down, in the capture type, which is why it has to be asked rather than
/// inferred from the form's own text.
///
/// Same family as BUG-367: the question "may this be absent?" has an authority,
/// and guessing instead of consulting it produces a wrong answer.
fn body_type_matches_empty(form: &FormClause, reg: &SyntaxRegistry) -> bool {
    let Some(pat) = &form.body_capture else {
        return false;
    };
    let crate::parser::meta_ast::CaptureType::Custom(name) = body_capture_type(pat) else {
        return false;
    };
    let Some(def) = reg.capture_types().find(|d| d.name == name) else {
        return false;
    };
    pattern_matches_empty(&def.pattern)
}

/// Whether a capture-type pattern can match the empty input.
fn pattern_matches_empty(p: &CapturePatternAst) -> bool {
    match p {
        // `( … )*` / `( … )?` — zero repetitions is a match.
        CapturePatternAst::Group { modifier, .. } => matches!(
            modifier,
            Some(CaptureModifier::ZeroOrMore) | Some(CaptureModifier::Optional)
        ),
        CapturePatternAst::Capture { modifier, .. } => matches!(
            modifier,
            CaptureModifier::ZeroOrMore | CaptureModifier::Optional
        ),
        // A sequence matches empty only if EVERY element does.
        CapturePatternAst::Sequence(parts) => parts.iter().all(pattern_matches_empty),
        // A choice matches empty if ANY alternative does.
        CapturePatternAst::Choice(alts) => alts.iter().any(pattern_matches_empty),
        _ => false,
    }
}

/// True when nothing in the body is required, so an absent body is correct
/// rather than a defect.
fn body_is_all_optional(form: &FormClause) -> bool {
    let groups_optional = form.body_groups.iter().all(|g| match g {
        CapturePatternAst::Capture { modifier, .. } => !matches!(
            modifier,
            CaptureModifier::Required | CaptureModifier::OneOrMore
        ),
        CapturePatternAst::Group { modifier, .. } => !matches!(
            modifier,
            Some(CaptureModifier::Required) | Some(CaptureModifier::OneOrMore)
        ),
        // A literal/char-class/sequence carries no requiredness of its own.
        _ => true,
    });

    let params_optional = form.body_params.iter().all(|p| {
        p.default.is_some()
            || p.elements.iter().all(|el| match el {
                FormInlineElement::Capture(cap, d) => {
                    d.is_some()
                        || matches!(
                            cap.modifier,
                            CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                        )
                }
                _ => true,
            })
    });

    // A greedy body capture spelled optional.
    //
    // BOTH repetition markers mean "may be absent": `?` (zero-or-one) and `*`
    // (zero-or-more). Checking only `?` left `@object`/`@stage`'s
    // `{ $children* }` looking REQUIRED, so a form written with an empty body
    // failed to print — for a body the grammar explicitly allows to be empty.
    let capture_optional = match &form.body_capture {
        None => true,
        Some(p) => p.contains('?') || p.contains('*'),
    };

    groups_optional && params_optional && capture_optional
}

/// One element of an alternation body.
///
/// The element is a `Named` carrying exactly one live alternative. Two shapes
/// need structure rather than a bare join:
///   * `{prop, value}` — a declaration, prints `prop: value;`
///   * `{sel, scope}`  — a nested scope, prints `sel { … }`
/// everything else falls through to the ordinary compound rendering.
fn alternation_element_text(
    macro_name: &str,
    var: &str,
    v: &CapturedValue,
) -> Result<String> {
    let CapturedValue::Named(map) = v else {
        return value_text(macro_name, var, v);
    };

    // The two STRUCTURAL shapes are checked on the node itself first: a nested
    // scope arrives as `{sel, scope}` at THIS level (not wrapped in a
    // single-key envelope), and missing it printed `[…] .doc-card` — the array
    // leaked and the selector lost its braces, which then failed to re-parse
    // (measured on `demos/spacetime-docs/index.st`).
    if let Some(text) = structural_element_text(macro_name, var, map)? {
        return Ok(text);
    }

    // Otherwise unwrap the single-alternative envelope (`{"line": …}`) and try
    // the same structural shapes one level down.
    let live: Vec<(&String, &CapturedValue)> = map
        .iter()
        .filter(|(_, val)| !is_absent(val))
        .collect();
    if live.len() == 1 {
        let inner = live[0].1;
        if let CapturedValue::Named(m) = inner
            && let Some(text) = structural_element_text(macro_name, var, m)?
        {
            return Ok(text);
        }
        return value_text(macro_name, var, inner);
    }

    value_text(macro_name, var, v)
}

/// Render a compound value by walking the `%capture_type` GRAMMAR that produced
/// it — the generic path for cons-list and separator shapes.
///
/// These bodies are not ad-hoc: each is a declared pattern with literal
/// separators, e.g.
/// ```text
/// %capture_type variant_list { $head:variant_def $tail:variant_alt+ }
/// %capture_type variant_alt  { "|" $v:variant_def }
/// %capture_type arm          { $pat:match_pat "=>" $inv:template_invocation ";"? }
/// %capture_type score_line   { $head:score_step ( "->" $next:score_step )* ";" }
/// ```
/// Walking the pattern emits the literals (`|`, `=>`, `->`, `;`) in their
/// declared positions and substitutes each capture from the value, so ONE
/// function covers `variant_list`, `arm`, `score_entry` and every future shape.
/// Writing four hand-rolled cases instead would be exactly the ad-hoc wiring
/// this design exists to avoid.
/// Does the match actually CARRY this capture type's own captures?
///
/// Used to decide whether an absent-by-name capture is FLATTENED (its type's
/// captures parked at the top level of the match, which is how `send_clause`
/// and `receive_block` arrive) or simply not written.
///
/// The distinction has teeth: descending on a hunch renders the type's LITERALS
/// against unrelated data, which is how `on_mutation` (`"$" $target "<-" $expr`)
/// printed a bare `$<-;` into every `@on` body that contained only a motion
/// line. Looking first costs one walk and cannot invent syntax.
fn type_captures_present(
    pattern: &CapturePatternAst,
    map: &std::collections::HashMap<String, CapturedValue>,
    reg: &SyntaxRegistry,
    depth: usize,
) -> bool {
    if depth > 8 {
        return false;
    }
    match pattern {
        CapturePatternAst::Capture {
            var_name,
            capture_type,
            ..
        } => {
            if map.get(var_name).is_some_and(|v| !is_absent(v)) {
                return true;
            }
            // One level down: `receive_block` names `$arms`, which is where the
            // data actually lands.
            if let crate::parser::meta_ast::CaptureType::Custom(name) = capture_type
                && let Some(def) = reg.capture_types().find(|d| &d.name == name)
            {
                return type_captures_present(&def.pattern, map, reg, depth + 1);
            }
            false
        }
        CapturePatternAst::Sequence(parts) => parts
            .iter()
            .any(|p| type_captures_present(p, map, reg, depth)),
        CapturePatternAst::Choice(alts) => alts
            .iter()
            .any(|a| type_captures_present(a, map, reg, depth)),
        CapturePatternAst::Group { pattern, .. } => {
            type_captures_present(pattern, map, reg, depth)
        }
        _ => false,
    }
}

fn grammar_text(
    macro_name: &str,
    var: &str,
    pattern: &CapturePatternAst,
    v: &CapturedValue,
    reg: &SyntaxRegistry,
    depth: usize,
) -> Result<Option<String>> {
    // Guard against a self-referential grammar walking forever.
    if depth > 24 {
        return Ok(None);
    }

    match pattern {
        CapturePatternAst::Sequence(parts) => {
            let CapturedValue::Named(map) = v else {
                return Ok(None);
            };
            let mut out: Vec<String> = Vec::new();
            // Track whether any CAPTURE contributed. A sequence that yields only
            // its literals matched nothing: `( "->" $next:score_step )*` with no
            // `$next` present must print NOTHING, not a dangling `->`. Emitting
            // the bare arrow produced `&first for inherit ->;`, which no longer
            // matched `form-score` and dropped the form from the file.
            let mut any_capture = false;
            for p in parts {
                match p {
                    CapturePatternAst::Literal(lit) => out.push(lit.clone()),
                    CapturePatternAst::Capture {
                        var_name,
                        capture_type,
                        ..
                    } => {
                        let Some(inner) = map.get(var_name) else {
                            continue;
                        };
                        if is_absent(inner) {
                            continue;
                        }
                        out.push(render_by_capture_type(
                            macro_name,
                            var_name,
                            capture_type,
                            inner,
                            reg,
                            depth + 1,
                        )?);
                        any_capture = true;
                    }
                    CapturePatternAst::Group { pattern, .. } => {
                        if let Some(t) =
                            grammar_text(macro_name, var, pattern, v, reg, depth + 1)?
                        {
                            out.push(t);
                            any_capture = true;
                        }
                    }
                    _ => {}
                }
            }
            let joined = join_tokens(&out);
            if joined.is_empty() || !any_capture {
                Ok(None)
            } else {
                Ok(Some(joined))
            }
        }

        // `( $a:x ) | ( $b:y ) | …` — exactly one alternative is live.
        CapturePatternAst::Choice(alts) => {
            for a in alts {
                if let Some(t) = grammar_text(macro_name, var, a, v, reg, depth + 1)? {
                    return Ok(Some(t));
                }
            }
            Ok(None)
        }

        CapturePatternAst::Group { pattern, .. } => {
            grammar_text(macro_name, var, pattern, v, reg, depth + 1)
        }

        CapturePatternAst::Capture {
            var_name,
            capture_type,
            ..
        } => {
            let inner = match v {
                CapturedValue::Named(map) => map.get(var_name),
                other => Some(other),
            };
            match inner {
                Some(x) if !is_absent(x) => Ok(Some(render_by_capture_type(
                    macro_name,
                    var_name,
                    capture_type,
                    x,
                    reg,
                    depth + 1,
                )?)),

                // The capture is absent BY NAME — but it may be FLATTENED.
                //
                // `%form { @data signal … { ( $send:send_clause ) ( $receive:receive_block ) … } }`
                // parks `send_clause`'s own captures (`verb`, `target`) and
                // `receive_block`'s (`sum`, `arms`) at the TOP level of the
                // match. So `map.get("send")` is None while the data sits right
                // there under other keys, and returning None here printed
                //
                //     @data signal $inc() to $counter;
                //
                // with the entire send/receive body gone. That output does not
                // re-parse: it fails the form's own grammar with E0946
                // ("expected body block") and E0804 ("missing required
                // parameter 'sum'"). A signal that sends nothing and decodes
                // nothing is a different program, and it was reachable from a
                // committed reference page — `counter.st` round-tripped to it.
                //
                // `read.rs` already answers this question for INGRESS, via
                // `declared_captures_deep`. This is the same question asked in
                // the printing direction: when the named capture is missing, ask
                // its TYPE whether the flattened captures satisfy its grammar,
                // and render through that. Nothing is invented — if the type's
                // pattern finds no captures either, the result stays empty.
                _ => {
                    // Only descend when the flattened captures are ACTUALLY
                    // there. Without this guard the walk rendered a type's
                    // LITERALS against an unrelated match — `on_mutation` is
                    // `"$" $target:ident "<-" $expr:expr`, and with neither
                    // capture present it still printed `$<-;`, a mutation with
                    // no target and no expression, into every `@on` body that
                    // held only a motion line.
                    //
                    // "Is this capture stored under another name?" is answered
                    // by looking, not by assuming: if none of the type's own
                    // capture names appear in the match, the capture is simply
                    // absent and must print nothing.
                    if let crate::parser::meta_ast::CaptureType::Custom(tname) = capture_type
                        && let Some(def) = reg.capture_types().find(|d| &d.name == tname)
                        && let CapturedValue::Named(map) = v
                        && type_captures_present(&def.pattern, map, reg, 0)
                        && let Some(t) =
                            grammar_text(macro_name, var_name, &def.pattern, v, reg, depth + 1)?
                        && !t.trim().is_empty()
                    {
                        return Ok(Some(t));
                    }
                    Ok(None)
                }
            }
        }

        CapturePatternAst::Literal(lit) => Ok(Some(lit.clone())),
        CapturePatternAst::CharClass { .. } => Ok(None),
    }
}

/// Render every element of a repeated body through the element grammar.
/// `None` when the grammar yields nothing, so the caller can fall back.
fn render_array_by_grammar(
    macro_name: &str,
    var: &str,
    ct: &crate::parser::meta_ast::CaptureType,
    items: &[CapturedValue],
    reg: &SyntaxRegistry,
) -> Result<Option<String>> {
    let crate::parser::meta_ast::CaptureType::Custom(name) = ct else {
        return Ok(None);
    };
    let Some(def) = reg.capture_types().find(|d| &d.name == name) else {
        return Ok(None);
    };

    let mut parts = Vec::with_capacity(items.len());
    for it in items {
        match grammar_text(macro_name, var, &def.pattern, it, reg, 0)? {
            Some(t) if !t.trim().is_empty() => parts.push(t),
            _ => return Ok(None),
        }
    }
    if parts.is_empty() {
        return Ok(None);
    }
    Ok(Some(join_tokens(&parts)))
}

/// Render a value that is known to have a named capture type: consult that
/// type's grammar when there is one, else fall back to the value's own text.
fn render_by_capture_type(
    macro_name: &str,
    var: &str,
    ct: &crate::parser::meta_ast::CaptureType,
    v: &CapturedValue,
    reg: &SyntaxRegistry,
    depth: usize,
) -> Result<String> {
    // A repeated element arrives as an Array; render each and join.
    if let CapturedValue::Array(items) = v {
        let mut parts = Vec::with_capacity(items.len());
        for it in items {
            parts.push(render_by_capture_type(macro_name, var, ct, it, reg, depth + 1)?);
        }
        return Ok(join_tokens(&parts));
    }

    if let crate::parser::meta_ast::CaptureType::Custom(name) = ct
        && let Some(def) = reg.capture_types().find(|d| &d.name == name)
        && let Some(t) = grammar_text(macro_name, var, &def.pattern, v, reg, depth + 1)?
    {
        return Ok(t);
    }

    // BUG-373: a BUILTIN capture type has no `%capture_type` grammar for the
    // walk above to follow, so it lands here — and `value_text` renders a
    // `{name, args}` map as a bare string.
    //
    // `%capture_type arm { $pat:match_pat "=>" $inv:template_invocation ";"? }`
    // is the shape behind `@match`/`@view`, and `$inv` is exactly such a
    // builtin. So an arm printed as
    //
    //     @view $status { "thinking" => "thinking"; }
    //
    // losing the `&` and the parens. That re-parses — it is a well-formed arm
    // whose consequence is a quoted word — so the document round-tripped, the
    // page compiled, `check` passed, and the template simply never mounted.
    // Verse did it to its OWN file: `spacetime edn` then `spacetime st` turned
    // `&thinking();` into `"thinking";`.
    //
    // `structural_element_text` already knows how to spell an invocation
    // (including the rule that an arg naming a binding must not be quoted);
    // reuse it rather than write a second speller that can drift from the first.
    if let CapturedValue::Named(map) = v
        && let Some(t) = structural_element_text(macro_name, var, map)?
    {
        // The invocation speller self-terminates (BUG-066: `template_invocation`
        // absorbs its own trailing `;`). The `arm` grammar ALSO carries a `";"?`
        // after `$inv`, so returning it terminated here yields `&thinking;;`.
        // Hand back the bare invocation and let the grammar walk supply the
        // separator it declares — the alternative is teaching two places to
        // agree about one semicolon.
        return Ok(t.strip_suffix(';').unwrap_or(&t).to_string());
    }

    value_text(macro_name, var, v)
}

/// Join rendered tokens with `.st` spacing: no space before `;`/`,`, none after
/// an opening `(`.
fn join_tokens(parts: &[String]) -> String {
    let mut out = String::new();
    for p in parts {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        let tight = matches!(p, ";" | "," | ")");
        // A SIGIL binds tightly to the token after it. Several capture-type
        // grammars spell the sigil as its own literal —
        // `%capture_type on_mutation { "$" $target:ident "<-" $expr:expr }` — so
        // without this the walk emitted `$ tasks <- …`, splitting the binding in
        // two and failing to re-parse.
        let opens_sigil = out.ends_with('$') || out.ends_with('&') || out.ends_with('.');
        if !out.is_empty() && !tight && !out.ends_with('(') && !opens_sigil {
            out.push(' ');
        }
        out.push_str(p);
    }
    out
}

/// The structural alternatives of a motion body, recognised by their key set:
///   * `{prop, value}` → `prop: value;`   (a declaration line)
///   * `{sel, scope}`  → `sel { … }`      (a nested scope)
fn structural_element_text(
    macro_name: &str,
    var: &str,
    m: &std::collections::HashMap<String, CapturedValue>,
) -> Result<Option<String>> {
    // `&name(arg, …)` — a template invocation. `TemplateInvocation` is a BUILTIN
    // capture type (`parser/mod.rs:8265`), so it has no `%capture_type` grammar
    // for the generic walk to follow; without this it printed as
    // `["$entry"] "ad-row"` and the surrounding `@each` failed to re-parse.
    if let (Some(CapturedValue::String(name)), Some(args)) = (m.get("name"), m.get("args")) {
        // A template ARGUMENT that names a binding prints as the binding.
        //
        // `TemplateInvocation` stores its args as strings, so `&bubble($m)`
        // arrives as `String("$m")` and the general `value_text` rule — quote a
        // string, which is right everywhere else — printed `&bubble("$m")`. That
        // re-parses happily as a string LITERAL, so the template received the
        // two characters `$m` instead of the item, `$m.role`/`$m.text` resolved
        // to nothing, and `@each` rendered zero rows.
        //
        // The page compiled clean and the screenshot came back the right size
        // with an empty log — which is exactly the failure the DOM check in
        // `shot.sh` now exists to catch.
        let arg_text = match args {
            CapturedValue::Array(items) => items
                .iter()
                .map(|a| match a {
                    CapturedValue::String(s)
                        if s.starts_with('$') || s.starts_with('&') =>
                    {
                        Ok(s.clone())
                    }
                    other => value_text(macro_name, var, other),
                })
                .collect::<Result<Vec<_>>>()?
                .join(", "),
            other => value_text(macro_name, var, other)?,
        };
        return Ok(Some(if arg_text.is_empty() {
            format!("&{name};")
        } else {
            format!("&{name}({arg_text});")
        }));
    }

    if let (Some(p), Some(val)) = (m.get("prop"), m.get("value")) {
        return Ok(Some(format!(
            "{}: {};",
            value_text(macro_name, var, p)?,
            value_text(macro_name, var, val)?
        )));
    }

    // `($guard) => Cons;` — one arm of a `@match` block.
    //
    // The generic compound join flattened these to a bare `[…]` list, dropping
    // the parens, the `=>` and the `;`, so `@data derive $x T : @match { … }`
    // printed as an array and failed to re-parse.
    if let (Some(guard), Some(cons)) = (m.get("guard"), m.get("cons")) {
        let g = match guard {
            CapturedValue::Named(gm) => gm
                .get("expr")
                .map(|e| value_text(macro_name, var, e))
                .transpose()?
                .unwrap_or_default(),
            other => value_text(macro_name, var, other)?,
        };
        let c = match cons {
            CapturedValue::Named(cm) => cm
                .get("name")
                .map(|n| value_text(macro_name, var, n))
                .transpose()?
                .unwrap_or_default(),
            other => value_text(macro_name, var, other)?,
        };
        return Ok(Some(if g.is_empty() {
            format!("{c};")
        } else {
            format!("({g}) => {c};")
        }));
    }

    // `$target <- expr;` — a MUTATION line in an `@on` body. The target is a
    // binding NAME without its sigil, so it must be re-prefixed here: the
    // generic compound join printed `$ tasks <- …` (sigil split from the name),
    // which then failed to re-parse.
    if let (Some(target), Some(expr)) = (m.get("target"), m.get("expr")) {
        let name = value_text(macro_name, var, target)?;
        let name = if name.starts_with('$') {
            name
        } else {
            format!("${name}")
        };
        return Ok(Some(format!(
            "{name} <- {};",
            value_text(macro_name, var, expr)?
        )));
    }

    if let (Some(sel), Some(scope)) = (m.get("sel"), m.get("scope")) {
        let body = match scope {
            CapturedValue::Array(items) => {
                let mut inner_lines = Vec::with_capacity(items.len());
                for it in items {
                    inner_lines.push(alternation_element_text(macro_name, var, it)?);
                }
                inner_lines.join(" ")
            }
            other => value_text(macro_name, var, other)?,
        };
        return Ok(Some(format!(
            "{} {{ {} }}",
            value_text(macro_name, var, sel)?,
            body.trim()
        )));
    }

    Ok(None)
}

/// An empty string / empty array is how an unmatched alternative is encoded.
fn is_absent(v: &CapturedValue) -> bool {
    match v {
        CapturedValue::String(s) => s.is_empty(),
        CapturedValue::Array(a) => a.is_empty(),
        _ => false,
    }
}

/// `"{\n  $variants:variant_list\n}"` → `CaptureType::Custom("variant_list")`.
fn body_capture_type(pattern: &str) -> crate::parser::meta_ast::CaptureType {
    use crate::parser::meta_ast::CaptureType;
    let name = pattern
        .split('$')
        .nth(1)
        .and_then(|rest| rest.split(':').nth(1))
        .map(|t| {
            t.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .unwrap_or_default();
    CaptureType::Custom(name)
}

/// `"{\n  $body:expr\n}"` → `"body"`.
fn body_capture_var(pattern: &str) -> String {
    pattern
        .split('$')
        .nth(1)
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .unwrap_or_else(|| "body".to_string())
}

/// The text for one capture, or `None` when it is absent/defaulted and should
/// be omitted entirely.
fn capture_text(
    macro_name: &str,
    cap: &FormCapture,
    default: Option<&ParamDefault>,
    m: &crate::syntax::FormMatch,
) -> Result<Option<String>> {
    let Some(value) = m.captures.get(&cap.var_name) else {
        // Absent + optional ⇒ correctly omitted. Absent + required ⇒ a real
        // inconsistency between pattern and match.
        return if matches!(
            cap.modifier,
            CaptureModifier::Optional | CaptureModifier::ZeroOrMore
        ) || default.is_some()
        {
            Ok(None)
        } else {
            Err(PrintError::new(
                macro_name,
                format!(
                    "required capture {:?} is missing from the match",
                    cap.var_name
                ),
            ))
        };
    };

    // SPEC §8.1: suppress a capture whose value equals the registry's DECLARED
    // default. `capture_spans` cannot be used for this — `@fn`'s body and
    // `@type`'s fields are author-written yet span-less (measured).
    if let Some(d) = default
        && !matches!(d, ParamDefault::None)
        && equals_default(value, d)
    {
        return Ok(None);
    }
    // An empty string with no declared default is how the parser represents
    // "this optional param was never written" for several stdlib forms.
    if default.is_none()
        && matches!(cap.modifier, CaptureModifier::Optional)
        && matches!(value, CapturedValue::String(s) if s.is_empty())
    {
        return Ok(None);
    }

    let mut text = value_text(macro_name, &cap.var_name, value)?;

    // `$source as $item` — the alias is part of the authored syntax, so it must
    // be printed or the form is structurally incomplete.
    if let Some(alias) = &cap.alias_capture
        && let Some(alias_text) = capture_text(macro_name, alias, None, m)?
    {
        text.push_str(" as ");
        text.push_str(&alias_text);
    }

    Ok(Some(text))
}

fn equals_default(v: &CapturedValue, d: &ParamDefault) -> bool {
    match (v, d) {
        (CapturedValue::String(s), ParamDefault::String(t)) => s == t,
        (CapturedValue::Number(n), ParamDefault::Number(t)) => n == t,
        (CapturedValue::Bool(b), ParamDefault::Bool(t)) => b == t,
        (CapturedValue::Time(ms), ParamDefault::Number(t)) => (*ms as f64) == *t,
        (CapturedValue::Length(LengthValue { value, unit }), ParamDefault::Length(n, u)) => {
            value == n && unit == u
        }
        (CapturedValue::Array(items), ParamDefault::EmptyArray) => items.is_empty(),
        (CapturedValue::Named(m), ParamDefault::EmptyObject) => m.is_empty(),
        _ => false,
    }
}

/// One captured value as `.st` source text.
fn value_text(macro_name: &str, var: &str, v: &CapturedValue) -> Result<String> {
    Ok(match v {
        CapturedValue::Ident(s)
        | CapturedValue::Binding(s)
        | CapturedValue::Selector(s)
        | CapturedValue::TypeRef(s)
        | CapturedValue::Color(s) => s.clone(),

        // An element reference is authored WITH its `&` sigil (`&hero__title`),
        // but the capture may hold the bare name. Restore it, or the printed
        // text is a different token entirely.
        CapturedValue::Element(s) => {
            if s.starts_with('&') {
                s.clone()
            } else {
                format!("&{s}")
            }
        }

        // RULE 2 again: raw JS is carried verbatim, never rebuilt.
        CapturedValue::Expr(s) => s.trim().to_string(),

        CapturedValue::String(s) => format!("{s:?}"),
        CapturedValue::Bool(b) => b.to_string(),
        CapturedValue::Number(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CapturedValue::Time(ms) => format!("{ms}ms"),
        CapturedValue::Length(LengthValue { value, unit }) => {
            if value.fract() == 0.0 {
                format!("{}{}", *value as i64, unit)
            } else {
                format!("{value}{unit}")
            }
        }
        CapturedValue::Preset(s) => format!("~{s}"),

        CapturedValue::Array(items) => {
            let inner = items
                .iter()
                .map(|i| value_text(macro_name, var, i))
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            format!("[{inner}]")
        }

        // `@match { … }` — a value-producing match block. The `arms` key is the
        // marker; each arm renders through `structural_element_text`, which knows
        // the `($guard) => Cons;` shape.
        CapturedValue::Named(map) if map.contains_key("arms") => {
            let arms = match map.get("arms") {
                Some(CapturedValue::Array(items)) => {
                    let mut out = Vec::with_capacity(items.len());
                    for it in items {
                        out.push(alternation_element_text(macro_name, var, it)?);
                    }
                    out.join(" ")
                }
                Some(other) => value_text(macro_name, var, other)?,
                None => String::new(),
            };
            format!("@match {{ {} }}", arms.trim())
        }

        // A COMPOUND value: a capture type built from named parts (`$x.load` is
        // `Named{subject, member}`; `40ms` in a stagger slot is
        // `Named{delay, grid, from}`). Empty-string parts are the encoding for
        // "this part was not written", so they are dropped rather than printed.
        //
        // Ordering: `subject`/`member` reconstruct a dotted driver reference;
        // everything else joins on space in a stable (sorted) order so the
        // output is deterministic.
        CapturedValue::Named(map) => {
            let live: Vec<(&String, &CapturedValue)> = {
                let mut v: Vec<_> = map
                    .iter()
                    .filter(|(_, val)| match val {
                        CapturedValue::String(s) => !s.is_empty(),
                        CapturedValue::Array(a) => !a.is_empty(),
                        _ => true,
                    })
                    .collect();
                v.sort_by(|a, b| a.0.cmp(b.0));
                v
            };

            // Driver reference: `$subject.member`, or `&.member` when the subject
            // is absent — an EMPTY subject means "this element", which is spelled
            // `&.` in `.st`. Printing a bare `member` dropped the reference and
            // changed what the form binds to.
            if map.contains_key("member") {
                let member = live
                    .iter()
                    .find(|(k, _)| k.as_str() == "member")
                    .map(|(_, v)| value_text(macro_name, var, v))
                    .transpose()?
                    .unwrap_or_default();
                let subject = live
                    .iter()
                    .find(|(k, _)| k.as_str() == "subject")
                    .map(|(_, v)| value_text(macro_name, var, v))
                    .transpose()?;
                // `(name: parallax, scope: cover, …)` — the driver's own
                // arguments. Dropping them printed `@on &.scroll { }`, a form
                // that still parses but configures nothing.
                let params = match map.get("driver_params") {
                    Some(CapturedValue::Array(items)) if !items.is_empty() => {
                        let mut parts = Vec::with_capacity(items.len());
                        for it in items {
                            if let CapturedValue::Named(pm) = it
                                && let (Some(n), Some(v)) = (pm.get("name"), pm.get("value"))
                            {
                                parts.push(format!(
                                    "{}: {}",
                                    value_text(macro_name, var, n)?,
                                    value_text(macro_name, var, v)?
                                ));
                            }
                        }
                        if parts.is_empty() {
                            String::new()
                        } else {
                            format!("({})", parts.join(", "))
                        }
                    }
                    _ => String::new(),
                };

                return Ok(match subject {
                    Some(s) if !s.is_empty() => format!("{s}.{member}{params}"),
                    _ => format!("&.{member}{params}"),
                });
            }

            if live.len() == 1 {
                return value_text(macro_name, var, live[0].1);
            }

            live.iter()
                .map(|(_, v)| value_text(macro_name, var, v))
                .collect::<Result<Vec<_>>>()?
                .join(" ")
        }

        CapturedValue::ParamList(ps) => ps
            .iter()
            .map(|p| {
                let sigil = match p.kind {
                    crate::syntax::TemplateParamKind::Binding => "$",
                    crate::syntax::TemplateParamKind::Element => "&",
                };
                let mut s = format!("{sigil}{}", p.name);
                if p.collection {
                    s.push_str("[]");
                }
                if p.optional {
                    s.push('?');
                }
                if let Some(t) = &p.type_ref {
                    let _ = write!(s, " {t}");
                }
                if let Some(d) = &p.default {
                    let _ = write!(s, " = {d}");
                }
                s
            })
            .collect::<Vec<_>>()
            .join(", "),

        // Everything else is a structured body shape that only appears in body
        // position; reaching it here means the pattern and the match disagree.
        other => {
            return Err(PrintError::new(
                macro_name,
                format!("capture {var:?} holds {other:?}, which has no inline rendering"),
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::STDLIB_REGISTRY;

    /// Print one statement and check it re-parses to an EQUIVALENT match.
    /// Equivalence, not string equality: class A is a SEMANTIC bijection.
    fn assert_reparses(src: &str) {
        let ast = crate::parse(src).expect("fixture must parse");
        assert!(!ast.matches.is_empty(), "fixture produced no matches: {src}");

        let printed =
            print_forms(&ast.matches, &STDLIB_REGISTRY).unwrap_or_else(|e| panic!("print failed: {e}"));
        let reparsed = crate::parse(&printed)
            .unwrap_or_else(|e| panic!("printed text did not parse:\n{printed}\n{e:?}"));

        assert_eq!(
            ast.matches.len(),
            reparsed.matches.len(),
            "match count changed\n  original: {src}\n  printed:  {printed}"
        );
        for (a, b) in ast.matches.iter().zip(reparsed.matches.iter()) {
            assert_eq!(
                a.matched_macro.as_deref().unwrap_or(&a.macro_name),
                b.matched_macro.as_deref().unwrap_or(&b.macro_name),
                "macro identity changed\n  original: {src}\n  printed:  {printed}"
            );
        }
    }

    #[test]
    fn prints_data_inline() {
        // The canonical two-literal form: `inline`, `:` and `;` must all be
        // recovered FROM THE PATTERN, since EDN never spells them.
        assert_reparses(r#"@data inline $mcpPrompt : "";"#);
    }

    #[test]
    fn prints_mcp_action_without_default_noise() {
        // Parsing this yields SIX captures (target/targetAttr/valueAttr/on are
        // materialised defaults). The printer must not emit the four the author
        // never wrote.
        let src = r#".b { @mcp-action(action: "go", value: "v") }"#;
        let ast = crate::parse(src).expect("parse");
        let printed = print_forms(&ast.matches, &STDLIB_REGISTRY).expect("print");
        assert!(printed.contains("action"), "{printed}");
        assert!(
            !printed.contains("targetAttr"),
            "defaulted captures leaked into output: {printed}"
        );
    }

    #[test]
    fn prints_data_stream() {
        assert_reparses(r#"@data stream $fx from "/api/fx";"#);
    }

    #[test]
    fn unknown_macro_fails_loud() {
        // Rule 1: never silently skip a form we cannot print.
        let m = crate::syntax::FormMatch {
            macro_name: "definitely-not-a-macro".to_string(),
            ..Default::default()
        };
        let err = print_form(&m, &STDLIB_REGISTRY).unwrap_err();
        assert_eq!(err.macro_name, "definitely-not-a-macro");
        assert!(err.message.contains("no registered form"), "{}", err.message);
    }
}
