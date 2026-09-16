//! EDN text → `Vec<FormMatch>` (PLAN-148 W3).
//!
//! Normative surface: `docs/edn-spacetime/SPEC.md` §3.
//!
//! # The wire, in one line
//!
//! ```clojure
//! (<macro-name> :<capture> <value> …)
//! ```
//!
//! `<macro-name>` is the registry's own key (`RegisteredForm.macro_name`,
//! looked up with [`SyntaxRegistry::get_by_macro_name`]); `:<capture>` keys are
//! the capture names that form's `%form` declares. **Literals are never
//! written** — `inline`, `from`, `:` and `;` live in the pattern and are
//! recovered by the printer.
//!
//! This is why the reader is ~one function rather than 415: keying on the macro
//! makes the registry the authority, so a new `%macro` is writable in EDN with
//! no change here. Keying on `@directive` instead would need a hand-authored
//! kind-word → macro table for the 37 directives that carry more than one macro
//! (`@data` alone has 34) — the ad-hoc wiring this design exists to avoid.
//!
//! # Sugar (SPEC §7)
//!
//! * positional captures — `(data-inline $x "")`, order taken from the pattern
//! * `(sel "…" form…)` — `selector` is a `FormMatch` field, not a capture

use clojure_reader::edn::Edn;

use crate::edn::codec::from_edn_value;
use crate::parser::meta_ast::{FormClause, FormInlineElement};
use crate::syntax::{CapturedValue, FormMatch, SyntaxRegistry};

/// A read failure. Names the offending form and what was expected, so an EDN
/// author can fix the source from the message alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    pub message: String,
}

impl ReadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ReadError {}

type Result<T> = std::result::Result<T, ReadError>;

/// Parse EDN source into `FormMatch`es, validated against the registry.
///
/// The result feeds `pipeline::compile` exactly as a parsed `.st` file does —
/// same backend, no second pipeline.
pub fn read_forms(source: &str, registry: &SyntaxRegistry) -> Result<Vec<FormMatch>> {
    // `read_string` yields ONE form; wrap so a file of several top-level forms
    // reads as a sequence.
    let wrapped = format!("[{source}]");
    let root = clojure_reader::edn::read_string(&wrapped)
        .map_err(|e| ReadError::new(format!("EDN parse error: {e:?}")))?;

    let Edn::Vector(items) = root else {
        return Err(ReadError::new("expected a sequence of top-level forms"));
    };

    let mut out = Vec::new();
    for item in &items {
        // A DOCUMENT map (`{:st/forms […] :st/imports […] …}`) is a legal
        // top-level node, not only a bare sequence of forms — SPEC §9 defines it
        // as the whole-file shape, and `write_document` emits exactly this. A
        // generator producing a document therefore round-trips; before this it
        // hit "expected a form list", which reads as a malformed program when
        // the program was in fact the canonical whole-file form.
        //
        // Sibling members (`:st/scopes`, `:st/html`, `:st/css`) are not
        // FormMatches and are carried by `EdnDocument`; this entry point yields
        // forms, so it takes `:st/forms` and leaves the rest to `to_document`.
        if let Edn::Map(members) = item {
            // Keys are `Edn::Key("st/forms")` — the reader strips the leading
            // colon, so the lookup matches on the bare name.
            let forms = members.iter().find_map(|(k, v)| match k {
                Edn::Key(name) if *name == "st/forms" => Some(v),
                _ => None,
            });
            match forms {
                Some(Edn::Vector(fs)) | Some(Edn::List(fs)) => {
                    for f in fs {
                        read_into(f, None, registry, &mut out)?;
                    }
                    continue;
                }
                Some(other) => {
                    return Err(ReadError::new(format!(
                        ":st/forms must be a sequence of forms, found {other:?}"
                    )));
                }
                // A map with no `:st/forms` is legal ONLY when it carries some
                // other recognised document member (a page of pure styles, say).
                // An arbitrary or typo'd map must NOT be swallowed: `{:st/form
                // […]}` would otherwise compile to an empty program, and the
                // author's misspelling would surface as "nothing happened".
                None => {
                    const MEMBERS: [&str; 9] = [
                        "st/forms",
                        "st/imports",
                        "st/scopes",
                        "st/constructs",
                        "st/html",
                        "st/css",
                        "st/presets",
                        "st/patterns",
                        "st/meta-defs",
                    ];
                    let recognised: Vec<&str> = members
                        .iter()
                        .filter_map(|(k, _)| match k {
                            Edn::Key(name) if MEMBERS.contains(name) => Some(*name),
                            _ => None,
                        })
                        .collect();
                    if recognised.is_empty() {
                        let keys: Vec<String> = members
                            .iter()
                            .map(|(k, _)| match k {
                                Edn::Key(n) => format!(":{n}"),
                                other => format!("{other:?}"),
                            })
                            .collect();
                        return Err(ReadError::new(format!(
                            "not a Spacetime document: no recognised :st/… member \
                             (found {}). A document carries :st/forms and may carry \
                             :st/imports, :st/scopes, :st/constructs, :st/html, :st/css.",
                            keys.join(", ")
                        )));
                    }
                    continue;
                }
            }
        }

        read_into(item, None, registry, &mut out)?;
    }
    Ok(out)
}

/// Read one node, appending the forms it denotes. `selector` threads the
/// enclosing `(sel "…" …)` scope down to its members.
fn read_into(
    node: &Edn<'_>,
    selector: Option<&str>,
    registry: &SyntaxRegistry,
    out: &mut Vec<FormMatch>,
) -> Result<()> {
    let Edn::List(items) = node else {
        return Err(ReadError::new(format!(
            "expected a form list `(macro-name …)`, found {node:?}"
        )));
    };

    let head = match items.first() {
        Some(Edn::Symbol(s)) => *s,
        Some(other) => {
            return Err(ReadError::new(format!(
                "expected a macro name symbol at the head of a form, found {other:?}"
            )));
        }
        None => return Err(ReadError::new("empty form `()`")),
    };

    // `(sel "…" form…)` — a scope wrapper, not a macro. `selector` is a
    // FormMatch FIELD, so it factors out of a group of forms exactly as a `.st`
    // scope block does.
    if head == "sel" {
        let sel = match items.get(1) {
            Some(Edn::Str(s)) => *s,
            Some(other) => {
                return Err(ReadError::new(format!(
                    "(sel \"selector\" …): expected a selector string, found {other:?} \
                     — selectors are strings because `#hero` is not readable as a \
                     bare EDN symbol"
                )));
            }
            None => return Err(ReadError::new("(sel …): missing selector")),
        };
        for member in &items[2..] {
            read_into(member, Some(sel), registry, out)?;
        }
        return Ok(());
    }

    out.push(read_form(head, &items[1..], selector, registry)?);
    Ok(())
}

/// Build one `FormMatch` from a macro name and its arguments.
fn read_form(
    macro_name: &str,
    args: &[Edn<'_>],
    selector: Option<&str>,
    registry: &SyntaxRegistry,
) -> Result<FormMatch> {
    let form = registry.get_by_macro_name(macro_name).ok_or_else(|| {
        ReadError::new(format!(
            "unknown macro {macro_name:?} — no `%macro` of that name is registered. \
             EDN can only name forms the registry already defines; it cannot \
             introduce syntax."
        ))
    })?;

    // BUG-370: keyword validation must also accept the captures that live
    // INSIDE a `%capture_type`, because that is exactly what the printer emits.
    // `@data signal`'s pattern declares `$send:send_clause`, but the printer
    // writes the clause's own `$verb`/`$target` flattened to top level — so the
    // printer's own output could not be read back (`:arms` is not a capture of
    // this form). Positional sugar still uses the PATTERN's captures only:
    // a flattened sub-capture has no slot of its own.
    let declared = declared_captures_deep(&form.form, registry);
    let required = declared_captures(&form.form, true);
    let mut captures: std::collections::HashMap<String, CapturedValue> = Default::default();

    // Keyword args: `:name value` pairs. Positional args (sugar) are matched
    // against the pattern's capture order.
    let mut i = 0;
    let mut positional = 0usize;
    while i < args.len() {
        match &args[i] {
            Edn::Key(k) => {
                let value = args.get(i + 1).ok_or_else(|| {
                    ReadError::new(format!("{macro_name}: `:{k}` has no value"))
                })?;
                if !declared.iter().any(|d| d == k) {
                    return Err(ReadError::new(format!(
                        "{macro_name}: `:{k}` is not a capture of this form — \
                         declared captures are {declared:?}"
                    )));
                }
                captures.insert((*k).to_string(), decode(macro_name, k, value)?);
                i += 2;
            }
            value => {
                // Positional sugar: the pattern declares capture ORDER, so this
                // mapping is mechanical rather than a per-macro table.
                //
                // OPTIONAL captures are skipped. `@data inline $name:binding
                // $type:typeref? : $value:expr ;` has `type` sitting between the
                // two an author actually writes, so counting every declared
                // capture put the value in the `type` slot and emitted different
                // JS — caught by the parity test. Positional sugar addresses the
                // REQUIRED captures; anything optional needs its keyword.
                let name = required.get(positional).ok_or_else(|| {
                    ReadError::new(format!(
                        "{macro_name}: too many positional arguments — the form \
                         declares {} required capture(s): {required:?} (optional \
                         captures must be given by keyword)",
                        required.len()
                    ))
                })?;
                captures.insert(name.clone(), decode(macro_name, name, value)?);
                positional += 1;
                i += 1;
            }
        }
    }

    Ok(FormMatch {
        macro_name: form.form.directive_name.trim_start_matches('@').to_string(),
        matched_macro: Some(macro_name.to_string()),
        captures,
        selector: selector.map(str::to_string),
        ..Default::default()
    })
}

fn decode(macro_name: &str, capture: &str, value: &Edn<'_>) -> Result<CapturedValue> {
    from_edn_value(value).map_err(|e| {
        ReadError::new(format!("{macro_name}: capture `:{capture}`: {}", e.message))
    })
}

/// Every capture name a form ACCEPTS as a keyword, including the captures
/// declared inside the `%capture_type`s its pattern references (BUG-370).
///
/// The printer flattens sub-captures: `$send:send_clause` prints as the clause's
/// own `:verb`/`:target`/`:ephemeral`, not as a nested `:send`. Validation that
/// walked only the pattern therefore rejected the printer's OWN output, so
/// `spacetime edn F` followed by `spacetime st` failed on any form using a
/// capture type with inner captures. The corpus gate never saw it: it measures
/// `.st → EDN → .st`, never EDN back through the reader.
///
/// Depth is bounded by `seen`: capture types reference each other freely
/// (`score_line` → `score_step` → `score_subject` → …), and a cycle would
/// otherwise recurse forever.
fn declared_captures_deep(form: &FormClause, registry: &SyntaxRegistry) -> Vec<String> {
    let mut out = declared_captures(form, false);
    let mut seen: std::collections::HashSet<String> = Default::default();
    let mut queue: Vec<String> = form_capture_type_names(form);

    while let Some(ty) = queue.pop() {
        if !seen.insert(ty.clone()) {
            continue;
        }
        let Some(def) = registry.capture_types().find(|d| d.name == ty) else {
            continue;
        };
        collect_pattern_captures(&def.pattern, &mut out, &mut queue);
    }

    out
}

/// Capture names and nested capture-type references inside a `%capture_type`
/// pattern. Names are appended to `out` (deduped); referenced type names are
/// pushed onto `queue` for the caller's worklist.
fn collect_pattern_captures(
    pat: &crate::parser::meta_ast::CapturePatternAst,
    out: &mut Vec<String>,
    queue: &mut Vec<String>,
) {
    use crate::parser::meta_ast::{CapturePatternAst as P, CaptureType};

    match pat {
        P::Capture {
            var_name,
            capture_type,
            ..
        } => {
            if !out.contains(var_name) {
                out.push(var_name.clone());
            }
            if let CaptureType::Custom(name) = capture_type {
                queue.push(name.clone());
            }
        }
        P::Group { pattern, .. } => collect_pattern_captures(pattern, out, queue),
        P::Sequence(items) | P::Choice(items) => {
            for i in items {
                collect_pattern_captures(i, out, queue);
            }
        }
        _ => {}
    }
}

/// The `%capture_type` names a form's own pattern references.
fn form_capture_type_names(form: &FormClause) -> Vec<String> {
    use crate::parser::meta_ast::CaptureType;
    let mut out = Vec::new();

    fn walk(el: &FormInlineElement, out: &mut Vec<String>) {
        match el {
            FormInlineElement::Capture(c, _) => {
                if let CaptureType::Custom(name) = &c.capture_type {
                    out.push(name.clone());
                }
                if let Some(alias) = &c.alias_capture {
                    if let CaptureType::Custom(name) = &alias.capture_type {
                        out.push(name.clone());
                    }
                }
            }
            FormInlineElement::Group { elements, .. } => {
                for e in elements {
                    walk(e, out);
                }
            }
            FormInlineElement::KeywordBlock { body_params, .. }
            | FormInlineElement::PseudoSelector { body_params, .. }
            | FormInlineElement::PseudoClass { body_params, .. } => {
                for p in body_params {
                    for e in &p.elements {
                        walk(e, out);
                    }
                }
            }
            _ => {}
        }
    }

    for el in &form.inline_elements {
        walk(el, &mut out);
    }
    for p in &form.params {
        for e in &p.elements {
            walk(e, &mut out);
        }
    }
    for el in &form.post_arg_inline {
        walk(el, &mut out);
    }
    for p in &form.body_params {
        for e in &p.elements {
            walk(e, &mut out);
        }
    }
    // `body_groups` holds a braced body written as GROUPS, e.g.
    //   `{ ( $receive:receive_block ) }`
    // which lands in neither `inline_elements` nor `body_params`. Missing it is
    // what left `:arms` unrecognised: the whole receive vocabulary of every
    // @data signal/stream form lives behind exactly this field.
    for pat in &form.body_groups {
        let mut names = Vec::new();
        collect_pattern_captures(pat, &mut names, &mut out);
    }

    // A body capture like `{ $body:on_motion_body }` names its type in text.
    if let Some(pat) = &form.body_capture {
        if let Some(ty) = pat.split(':').nth(1) {
            let name: String = ty
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                out.push(name);
            }
        }
    }
    out
}

/// The capture names a form declares, in pattern order — the authority for both
/// keyword validation and positional sugar.
///
/// `required_only` drops captures the pattern marks optional (or that carry a
/// default). Keyword validation wants EVERY name; positional sugar wants only
/// the ones an author must write, else an optional capture silently eats a slot.
fn declared_captures(form: &FormClause, required_only: bool) -> Vec<String> {
    use crate::parser::meta_ast::CaptureModifier;

    let mut out = Vec::new();

    fn walk(el: &FormInlineElement, req: bool, optional_ctx: bool, out: &mut Vec<String>) {
        match el {
            FormInlineElement::Capture(c, default) => {
                let optional = optional_ctx
                    || default.is_some()
                    || matches!(
                        c.modifier,
                        CaptureModifier::Optional | CaptureModifier::ZeroOrMore
                    );
                if !(req && optional) {
                    out.push(c.var_name.clone());
                }
                if let Some(alias) = &c.alias_capture {
                    out.push(alias.var_name.clone());
                }
            }
            FormInlineElement::Comparison { capture, .. } => out.push(capture.var_name.clone()),
            FormInlineElement::Group { elements, modifier } => {
                let opt = optional_ctx
                    || !matches!(
                        modifier,
                        CaptureModifier::Required | CaptureModifier::OneOrMore
                    );
                for e in elements {
                    walk(e, req, opt, out);
                }
            }
            FormInlineElement::KeywordBlock { body_params, .. }
            | FormInlineElement::PseudoSelector { body_params, .. }
            | FormInlineElement::PseudoClass { body_params, .. } => {
                for p in body_params {
                    for e in &p.elements {
                        walk(e, req, optional_ctx, out);
                    }
                }
            }
            FormInlineElement::Literal(_) => {}
        }
    }

    for el in &form.inline_elements {
        walk(el, required_only, false, &mut out);
    }
    for p in &form.params {
        for e in &p.elements {
            walk(e, required_only, p.default.is_some(), &mut out);
        }
    }
    for el in &form.post_arg_inline {
        walk(el, required_only, false, &mut out);
    }
    for p in &form.body_params {
        for e in &p.elements {
            walk(e, required_only, p.default.is_some(), &mut out);
        }
    }
    // A braced body written as GROUPS (`{ refresh: $r:duration?  initial: $i… }`)
    // lands in `body_groups`, which nothing here walked — so `:initial` read as
    // "not a capture of this form" even though the pattern declares it.
    for pat in &form.body_groups {
        let mut queue = Vec::new();
        collect_pattern_captures(pat, &mut out, &mut queue);
    }
    if let Some(pat) = &form.body_capture {
        // EVERY `$var` in the body text, not just the first. A braced body is
        // stored unparsed, so a multi-capture body like
        //   `{ refresh: $refresh:duration?  initial: $initial:balanced(';')? }`
        // declares two captures — `.nth(1)` saw only `refresh`, and `:initial`
        // was rejected as "not a capture of this form" though the pattern
        // plainly declares it (BUG-370).
        for seg in pat.split('$').skip(1) {
            let var: String = seg
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !var.is_empty() && !out.contains(&var) {
                out.push(var);
            }
        }
    }

    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::STDLIB_REGISTRY;

    #[test]
    fn reads_keyword_form() {
        let forms = read_forms(
            r#"(data-inline :name $mcpPrompt :value "")"#,
            &STDLIB_REGISTRY,
        )
        .expect("read");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].matched_macro.as_deref(), Some("data-inline"));
        assert_eq!(
            forms[0].captures.get("name"),
            Some(&CapturedValue::Binding("$mcpPrompt".into()))
        );
    }

    #[test]
    fn positional_sugar_matches_pattern_order() {
        // The sugar is DERIVED from the pattern's capture order, not a table.
        let kw = read_forms(
            r#"(data-inline :name $x :value "")"#,
            &STDLIB_REGISTRY,
        )
        .unwrap();
        let pos = read_forms(r#"(data-inline $x "")"#, &STDLIB_REGISTRY).unwrap();
        assert_eq!(kw[0].captures.get("name"), pos[0].captures.get("name"));
    }

    #[test]
    fn sel_threads_the_selector() {
        let forms = read_forms(
            r#"(sel ".kit-btn-confirm" (mcp-action :action "kit-confirm"))"#,
            &STDLIB_REGISTRY,
        )
        .expect("read");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].selector.as_deref(), Some(".kit-btn-confirm"));
    }

    #[test]
    fn unknown_macro_is_rejected_by_the_registry() {
        // EDN cannot introduce syntax: the registry is the ceiling.
        let err = read_forms("(no-such-macro :a 1)", &STDLIB_REGISTRY).unwrap_err();
        assert!(err.message.contains("unknown macro"), "{}", err.message);
    }

    #[test]
    fn unknown_capture_is_rejected() {
        let err = read_forms(
            r#"(data-inline :nope 1)"#,
            &STDLIB_REGISTRY,
        )
        .unwrap_err();
        assert!(
            err.message.contains("not a capture"),
            "{}",
            err.message
        );
    }

    #[test]
    fn deref_is_not_a_directive() {
        // `@` is free precisely BECAUSE forms are keyed by macro name, so a
        // Lisp `@count` deref can never be mistaken for a directive.
        let err = read_forms("(text @count)", &STDLIB_REGISTRY).unwrap_err();
        assert!(err.message.contains("unknown macro"), "{}", err.message);
    }

    /// SPEC §2 obligation P3, in miniature: EDN and `.st` reach the SAME
    /// `FormMatch`, so they compile through the identical backend.
    #[test]
    fn edn_and_st_agree_on_the_same_program() {
        let st = crate::parse(r#"@data inline $mcpPrompt : "";"#).expect("parse .st");
        let edn = read_forms(
            r#"(data-inline :name $mcpPrompt :value "\"\"")"#,
            &STDLIB_REGISTRY,
        )
        .expect("read .edn");

        assert_eq!(st.matches.len(), 1);
        assert_eq!(edn.len(), 1);
        assert_eq!(
            st.matches[0].matched_macro.as_deref(),
            edn[0].matched_macro.as_deref()
        );
        assert_eq!(st.matches[0].macro_name, edn[0].macro_name);
        assert_eq!(
            st.matches[0].captures.get("name"),
            edn[0].captures.get("name")
        );
    }
}
