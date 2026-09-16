//! PLAN-124 W3.2 — the `drive` bind expansion: `@on`'s driver-projection slot.
//!
//! The new `@on` head (`stdlib/macros/on.st`) binds ONE pseudo-primitive:
//!
//! ```text
//! %binds { drive(&self, driver: $driver, form: $form) }
//! ```
//!
//! This module expands it, between Evaluate and Resolve, into the REAL binds
//! the match needs — the driver's primitive from the REGISTRY (the entry's
//! `primitive` field is the dispatch table; no Rust arm per driver) plus an
//! `apply-animations` consuming the resolved motion form. Errors are
//! diagnostics, never silence:
//!
//! - E0948 — the member names no registered driver / the driver is `planned:`
//! - E0949 — the projection does not apply to this kind of ref, or the form
//!   in the form slot is not a motion form
//!
//! Out of scope (later waves, recorded in the implementation report):
//! positional driver params (`.key(" ")`), the `as &name` timeline naming,
//! `@on <driver> { … }` body lowering (on_actions — W3.3), and static
//! subject-kind checking beyond the subject-less (elem) case — typed refs
//! are W4.

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::metasystem::MetaRegistry;
use crate::parser::meta_ast::{BindArg, BindDecl, BindOutput, BindValue, RegisterValue};
use crate::syntax::{CapturedValue, FormMatch, KeyframeDef, STDLIB_FORM_DECLARATIONS, replace_param_token};

use super::evaluate::EvaluatedMatch;

/// Expand every `drive(...)` bind in the evaluated matches, in place. Returns
/// the diagnostics produced (E0948/E0949); a `drive` bind that errored is
/// REMOVED so resolution never sees the pseudo-primitive.
/// `drive-legacy-space(&self, driver:, form:)` — the D28 error sibling: the
/// space form matched, so the consequence is missing its `:`. Refuse loudly
/// with the exact rewrite (never a silent drop, never a wrong dispatcher).
fn expand_legacy_space_error(
    form_match: &FormMatch,
    span: crate::parser::SourceSpan,
) -> Diagnostic {
    let member = capture_named(form_match, "driver")
        .and_then(|m| m.get("member"))
        .and_then(|v| text_of(Some(v)))
        .unwrap_or_default();
    drive_error(
        DiagnosticCode::E0946,
        span,
        format!(
            "`@on &.{member} --form;` — body-less `@on` statements carry `:` before the consequence (D28)"
        ),
        format!(
            "write `@on &.{member}: --form;` — `:` introduces the value, `{{ }}` is choreography"
        ),
    )
}

/// `drive-as-error(&self, driver:, as:)` — the BUG-261 error sibling for an
/// `as` clause that does not name a signal: `as &reveal` or `as reveal`.
///
/// `on_as` is optional and captures only `$name:binding`. Without explicit
/// siblings, either wrong spelling failed the capture, vanished with the
/// whole clause, and published under `__drive_visible_1`. Both spellings
/// violate the same rule, so one E0953 and one diagnostic implementation name
/// their shared repair: a driver's `as` clause publishes a progress SIGNAL.
fn expand_as_error(form_match: &FormMatch, span: crate::parser::SourceSpan) -> Diagnostic {
    let member = capture_named(form_match, "driver")
        .and_then(|m| m.get("member"))
        .and_then(|v| text_of(Some(v)))
        .unwrap_or_default();
    // `element` deliberately stays outside `text_of`; `ident` (the bare
    // spelling) is already one of text_of's accepted scalar captures.
    let name = match capture_named(form_match, "as").and_then(|m| m.get("name")) {
        Some(CapturedValue::Element(e)) => format!("&{}", e.trim_start_matches('&')),
        other => text_of(other).unwrap_or_default(),
    };
    let bare = name.trim_start_matches('&').trim_start_matches('$');
    drive_error(
        DiagnosticCode::E0953,
        span,
        format!(
            "`as {name}` does not name a progress SIGNAL, but a driver's `as` clause \
             publishes one"
        ),
        format!(
            "write `@on &.{member} as ${bare}` — `$` is data that flows (the progress \
             value), `&` is identity (which thing)"
        ),
    )
}

/// `optional-clause-error(message:, hint:)` is the shared sink for data-defined
/// error siblings. New clauses contribute only registry/stdlib data, never a
/// directive-specific Rust branch.
fn expand_optional_clause_error(
    decl: &BindDecl,
    form_match: &FormMatch,
    span: crate::parser::SourceSpan,
) -> Option<Diagnostic> {
    let text_arg = |name: &str| {
        decl.args.iter().find_map(|arg| match arg {
            BindArg::Named {
                name: arg_name,
                value: BindValue::String(value) | BindValue::Ident(value),
            } if arg_name == name => Some(value.clone()),
            BindArg::Named {
                name: arg_name,
                value: BindValue::Number(value),
            } if arg_name == name => Some(value.to_string()),
            BindArg::Named {
                name: arg_name,
                value: BindValue::Variable(capture),
            } if arg_name == name => match form_match.captures.get(capture) {
                Some(
                    CapturedValue::Expr(value)
                    | CapturedValue::String(value)
                    | CapturedValue::Ident(value),
                ) => Some(value.clone()),
                Some(CapturedValue::Number(value)) => Some(value.to_string()),
                _ => None,
            },
            _ => None,
        })
    };
    // The `rule`/`value` data supplies a discrimination knob: a sibling may
    // opt to report only a specific near-miss shape. There is no value guard
    // here — BUG-264's widening of the strict `:duration` capture rejects ALL
    // near-misses (`5`, `5px`, `foo`, `5 ms`), and every one must be refused
    // loudly so the wrong-kind refresh is never silently dropped.
    Some(drive_error(
        DiagnosticCode::E0955,
        span,
        text_arg("message").unwrap_or_else(|| "invalid optional directive clause".to_string()),
        text_arg("hint").unwrap_or_else(|| {
            "remove the invalid clause or use its documented spelling".to_string()
        }),
    ))
}

/// Type evidence for facet splitting (review B, showcase §5): a facet on a
/// dotted arms head is only meaningful when the subject's type is KNOWN —
/// `.done` needs an `as`-bound progress publication, `.prev` needs a
/// declared signal or an `as` binding. Untyped ⇒ the head stays a plain
/// dotted path (status quo) — but `.done` specifically must ERROR
/// (uninferrable), never silently watch the head.
#[derive(Default)]
struct FacetEvidence {
    as_names: std::collections::HashSet<String>,
    declared_signals: std::collections::HashSet<String>,
}

fn collect_facet_evidence(matches: &[FormMatch]) -> FacetEvidence {
    let mut ev = FacetEvidence::default();
    for m in matches {
        if let Some(as_cap) = m.captures.get("as") {
            let name = match as_cap {
                CapturedValue::Named(mm) => mm.get("name").cloned(),
                other => Some(other.clone()),
            };
            if let Some(n) = name.and_then(|v| text_of(Some(&v))) {
                let bare = n.trim_start_matches('$').to_string();
                if !bare.is_empty() {
                    ev.as_names.insert(bare);
                }
            }
        }
        if m.macro_name == "local-state" || m.macro_name == "local-state-uninitialized" {
            if let Some(n) = text_of(m.captures.get("name")) {
                ev.declared_signals
                    .insert(n.trim_start_matches('$').to_string());
            }
        }
    }
    ev
}

pub fn expand_drive_binds(
    evaluated: &mut [EvaluatedMatch],
    matches: &[FormMatch],
    registry: &MetaRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut counter: usize = 0;
    let evidence = collect_facet_evidence(matches);

    for ev in evaluated.iter_mut() {
        if !ev.bind_decls.iter().any(|b| {
            matches!(
                b.primitive.as_str(),
                "drive"
                    | "drive-arms"
                    | "drive-body"
                    | "drive-legacy-space"
                    | "drive-as-error"
                    | "optional-clause-error"
            )
        }) {
            continue;
        }
        let span = ev.form_match.span;
        let mut expanded: Vec<BindDecl> = Vec::new();
        for decl in std::mem::take(&mut ev.bind_decls) {
            counter += 1;
            let result = match decl.primitive.as_str() {
                "drive" => expand_one(&decl, &mut ev.form_match, matches, registry, counter, span),
                "drive-arms" => expand_arms(
                    &decl,
                    &mut ev.form_match,
                    matches,
                    registry,
                    counter,
                    span,
                    &evidence,
                ),
                "drive-body" => {
                    expand_body(&decl, &mut ev.form_match, matches, registry, counter, span)
                }
                "drive-as-error" => {
                    // BUG-261 error sibling: no expansion, just the loud
                    // diagnostic — the shape `drive-legacy-space` established.
                    diagnostics.push(expand_as_error(&ev.form_match, span));
                    continue;
                }
                "optional-clause-error" => {
                    if let Some(diagnostic) =
                        expand_optional_clause_error(&decl, &ev.form_match, span)
                    {
                        diagnostics.push(diagnostic);
                    }
                    continue;
                }
                "drive-legacy-space" => {
                    // The error sibling: no expansion, just the loud diagnostic.
                    diagnostics.push(expand_legacy_space_error(&ev.form_match, span));
                    continue;
                }
                _ => {
                    expanded.push(decl);
                    continue;
                }
            };
            match result {
                Ok(mut decls) => expanded.append(&mut decls),
                Err(diag) => {
                    diagnostics.push(diag);
                    // The failed decl is dropped — if it was the match's
                    // ONLY bind, `bind_decls` is now empty and resolve would
                    // RESURRECT it from the macro definition, surfacing an
                    // E0956 that masks the diagnostic explaining the page
                    // (BUG-297; the same trap the E0951 score path sets
                    // `binds_consumed` for). The error above is the build's
                    // verdict; the successful siblings' output is moot.
                    ev.binds_consumed = true;
                }
            }
        }
        ev.bind_decls = expanded;
    }

    diagnostics
}

fn expand_one(
    decl: &BindDecl,
    form_match: &mut FormMatch,
    matches: &[FormMatch],
    registry: &MetaRegistry,
    counter: usize,
    span: crate::parser::SourceSpan,
) -> Result<Vec<BindDecl>, Diagnostic> {
    // --- The driver expression: member, optional subject, optional params.
    // The captures may be nested under `driver` (a Named map) or flattened
    // into the match's capture map — accept both, prefer the nested map. All
    // reads are cloned OUT here so the capture map can be mutated later.
    let nested = capture_named(form_match, "driver").cloned();
    // Field read: the driver_expr capture nests as a Named map, but
    // form_application — a SINGLE inner capture — flattens to a bare value at
    // match level. Read nested first, fall back to top-level. (Verified
    // shapes: driver = Named{subject, member}, form = Ident("--rise").)
    let read = |name: &str| -> Option<CapturedValue> {
        nested
            .as_ref()
            .and_then(|m| m.get(name))
            .cloned()
            .or_else(|| form_match.captures.get(name).cloned())
    };
    let member = text_of(read("member").as_ref()).ok_or_else(|| {
        drive_error(
            DiagnosticCode::E0948,
            span,
            "`drive` bind without a driver member — the `@on` head grammar \
             should always capture one; this is a compiler bug, please report it",
            "expected captures from `driver_expr`",
        )
    })?;
    // (subject is read again below with the empty-string filter — the kind
    // check owns the canonical read.)
    let driver_params = read("driver_params");

    // --- Registry lookup: the member names a registered driver.
    let entries = registry.entries_of("driver");
    let clause = entries
        .iter()
        .find(|(_, c)| registry.entry_key(c) == Some(member.as_str()))
        .map(|(_, c)| *c)
        .ok_or_else(|| {
            let mut known: Vec<&str> = entries
                .iter()
                .filter_map(|(_, c)| registry.entry_key(c))
                .collect();
            known.sort_unstable();
            drive_error(
                DiagnosticCode::E0948,
                span,
                format!("unknown driver projection `.{member}` — no `%registers driver({member})` entry exists"),
                format!(
                    "registered drivers: {} · a new driver is a `%registers driver(<name>)` clause in stdlib or your own macros — data, not syntax",
                    known.join(", ")
                ),
            )
        })?;

    // --- Value facets (PLAN-126 D26): `lowers_to` / `facet_primitive` /
    // `expr_lowering` entries are typed READS, not drivers. `$reveal.done`
    // lowers to a derived signal; `$price.prev` to the prev-signal primitive.
    // Read BEFORE the primitive requirement: a value facet HAS no primitive,
    // and the statement desugars to arms without one.
    let lowers_to = match registry.entry_field(clause, "lowers_to") {
        Some(RegisterValue::String(t)) => Some(t.clone()),
        _ => None,
    };
    let facet_primitive = match registry.entry_field(clause, "facet_primitive") {
        Some(RegisterValue::Ident(p)) => Some(p.clone()),
        _ => None,
    };
    let is_value_facet = lowers_to.is_some() || facet_primitive.is_some();
    let primitive = match registry.entry_field(clause, "primitive") {
        Some(RegisterValue::Ident(p)) => p.clone(),
        _ if is_value_facet => String::new(),
        _ => {
            return Err(drive_error(
                DiagnosticCode::E0948,
                span,
                format!(
                    "driver `{member}` has no `primitive` field — its registry entry is malformed"
                ),
                "every driver entry in stdlib/macros/drivers.st declares `primitive:`",
            ));
        }
    };
    if let Some(planned) = primitive.strip_prefix("planned:") {
        return Err(drive_error(
            DiagnosticCode::E0948,
            span,
            format!(
                "driver `.{member}` is registered but its primitive `{planned}` is PLANNED, not yet bound"
            ),
            "the word is known so the diagnostic can be precise; the primitive lands with its wave",
        ));
    }

    // --- Kind check: the subject's SIGIL selects the rail (D26) —
    // `&` elem (optional: the host) vs `$` signal (required).
    let on_kinds = match registry.entry_field(clause, "on_kinds") {
        Some(RegisterValue::Ident(k)) => k.clone(),
        _ => "elem".to_string(),
    };
    let subject = text_of(read("subject").as_ref()).filter(|s| !s.is_empty());
    let ssubject = text_of(read("ssubject").as_ref()).filter(|s| !s.is_empty());
    if ssubject.is_some() && on_kinds != "signal" {
        return Err(drive_error(
            DiagnosticCode::E0949,
            span,
            format!(
                "projection `.{member}` applies to `{on_kinds}` refs, but `${}` is a signal",
                ssubject.as_deref().unwrap_or("")
            ),
            format!("signal projections: {}", {
                let mut v: Vec<&str> = entries
                    .iter()
                    .filter(|(_, c)| {
                        matches!(registry.entry_field(c, "on_kinds"), Some(RegisterValue::Ident(k)) if k == "signal")
                    })
                    .filter_map(|(_, c)| registry.entry_key(c))
                    .collect();
                v.sort_unstable();
                v.iter()
                    .map(|k| format!("`$.{k}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
        ));
    }
    if ssubject.is_none() && subject.is_none() && on_kinds != "elem" {
        return Err(drive_error(
            DiagnosticCode::E0949,
            span,
            format!(
                "projection `.{member}` applies to `{on_kinds}` refs, but `&.{member}` (no subject) is the element host"
            ),
            format!("give the projection a `{on_kinds}` subject, e.g. `&<name>.{member}`"),
        ));
    }
    // An elem SUBJECT on a signal-only projection (`&hero.change`).
    if ssubject.is_none() && subject.is_some() && on_kinds == "signal" {
        return Err(drive_error(
            DiagnosticCode::E0949,
            span,
            format!(
                "projection `.{member}` applies to `signal` refs, but `&{}` is an element",
                subject.as_deref().unwrap_or("")
            ),
            format!("give the projection a signal subject, e.g. `$<name>.{member}`"),
        ));
    }

    // --- The `as` clause (D24/D30): names the progress publication.
    let as_name = read("as")
        .and_then(|v| match v {
            CapturedValue::Named(m) => m.get("name").cloned(),
            other => Some(other),
        })
        .and_then(|v| text_of(Some(&v)))
        .map(|n| n.trim_start_matches('$').to_string())
        .filter(|n| !n.is_empty());

    // --- The consequence (D28/D30): a form application OR a mutation.
    // on_consequence's choice wraps the branch: {form: <form_application>} or
    // {mut: <on_mutation>}. The transitional space sibling passes the bare
    // form_application (the pre-D28 flat shape).
    // The colon form's %binds passes `form: $consequence`; the transitional
    // space sibling passes `form: $form`. Read the CAPTURE names, not the
    // bind arg name.
    let consequence = read("consequence").or_else(|| read("form"));
    if let Some(CapturedValue::Named(cmap)) = &consequence
        && let Some(mutv) = cmap.get("mut")
    {
        // MUTATION CONSEQUENCE: `@on &.click: $open <- !$open;`. Mutations
        // attach to DOM EVENT listeners (on-mutation-handler) — signal-change
        // is not a DOM event, and a mutation publishes nothing.
        if ssubject.is_some() {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!(
                    "`@on ${ssubject}.{member}: $… <- …` — mutations attach to DOM event listeners, not signal changes",
                    ssubject = ssubject.as_deref().unwrap_or("")
                ),
                "to mutate on a signal change, use an arm: `@on $sig { _ => --form; }` (arms take form consequences)",
            ));
        }
        if as_name.is_some() {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!(
                    "`as` names a driver's progress publication — `@on &.{member}: $… <- …` has no driver"
                ),
                "drop the `as` clause, or use a body: `@on &.<driver> { … }`",
            ));
        }
        let value_type = match registry.entry_field(clause, "value_type") {
            Some(RegisterValue::Ident(v)) => v.clone(),
            _ => String::new(),
        };
        if value_type != "event" {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!(
                    "`@on &.{member}: $… <- …` — `.{member}` is a `{value_type}` driver; mutations need an EVENT driver"
                ),
                "event drivers: click, hover, focus, submit, key",
            ));
        }
        let (target, expr) = match mutv {
            CapturedValue::Named(m) => (
                text_of(m.get("target")).unwrap_or_default(),
                match m.get("expr") {
                    Some(CapturedValue::Expr(e)) => e.clone(),
                    other => text_of(other).unwrap_or_default(),
                },
            ),
            _ => (String::new(), String::new()),
        };
        let mutation_el = match &subject {
            Some(s) => BindArg::Element {
                name: s.clone(),
                child_selector: None,
            },
            None => BindArg::Element {
                name: "self".to_string(),
                child_selector: None,
            },
        };
        check_mutation_driver_params(
            driver_params.as_ref(),
            registry,
            clause,
            "on-mutation-handler",
            span,
        )?;
        let mut mutation_args = vec![
            mutation_el,
            BindArg::Named {
                name: "event".to_string(),
                value: BindValue::Ident(member.clone()),
            },
            BindArg::Named {
                name: "actions".to_string(),
                value: BindValue::String(format!("${target} <- {expr}")),
            },
        ];
        // BUG-375: forward what the row declares the author may write — a key
        // filter today. Without this the param validated and then vanished.
        mutation_args.extend(mutation_driver_param_args(
            driver_params.as_ref(),
            registry,
            clause,
        ));
        return Ok(vec![BindDecl {
            primitive: "on-mutation-handler".to_string(),
            args: mutation_args,
            outputs: Vec::new(),
            span: decl.span,
        }]);
    }

    // --- VALUE FACET in statement position (D30 bool sugar):
    // `@on $reveal.done: --form;` ≡ `@on $reveal.done { true => --form; }` —
    // a COMPILE-TIME desugar onto the arms path (one mechanism, no second
    // dispatcher). The facet lowers to a derived/prev signal the arms watch.
    if is_value_facet {
        // Only BOOL-typed facets (done) get the true-arm sugar; other value
        // facets (prev) are expression/derive/guard reads — a statement on
        // them is a type error (review D, showcase §3.9).
        let facet_value_type = match registry.entry_field(clause, "value_type") {
            Some(RegisterValue::Ident(v)) => v.clone(),
            _ => String::new(),
        };
        if facet_value_type != "bool" {
            return Err(drive_error(
                DiagnosticCode::E0950,
                span,
                format!(
                    "`@on $….{member}: --form;` — `.{member}` is a `{facet_value_type}` facet; only `done` takes the statement sugar"
                ),
                "read it in an expression (`color: $x > $x.prev ? …`), a derive, or a guard (`where $new > $x.prev`)",
            ));
        }
        let Some(sig) = &ssubject else {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!(
                    "`.{member}` is a value facet of SIGNALS — `&.{member}` (element) has nothing to read"
                ),
                format!("read it on a signal: `@on $<name>.{member} {{ true => --form; }}`"),
            ));
        };
        if as_name.is_some() {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!(
                    "`as` names a DRIVER's publication — `.{member}` is a value facet, not a driver"
                ),
                "drop the `as` clause — the subject signal is already the publication",
            ));
        }
        let form_map = match &consequence {
            Some(CapturedValue::Named(cmap)) => match cmap.get("form") {
                Some(CapturedValue::Named(inner)) => inner.clone(),
                Some(other) => {
                    let mut m = std::collections::HashMap::new();
                    m.insert("form".to_string(), other.clone());
                    if let Some(args) = cmap.get("args") {
                        m.insert("args".to_string(), args.clone());
                    }
                    m
                }
                None => {
                    return Err(drive_error(
                        DiagnosticCode::E0950,
                        span,
                        format!(
                            "`@on ${sig}.{member}: <consequence>` — the sugar takes a FORM consequence"
                        ),
                        "mutations need an event driver (`@on &.click: $x <- …`); bodies need braces",
                    ));
                }
            },
            _ => {
                return Err(drive_error(
                    DiagnosticCode::E0948,
                    span,
                    format!(
                        "`drive` bind without a form — the statement shape is `@on <driver>: --form;`"
                    ),
                    "write `@on &.<member>: --<form>;` with a declared motion form",
                ));
            }
        };
        let lowered = lower_signal_facet(
            form_match,
            sig,
            &member,
            lowers_to.as_deref(),
            facet_primitive.as_deref(),
            counter,
            span,
        );
        let mut binds = lowered.prelude;
        binds.extend(expand_true_arm(
            form_match,
            matches,
            registry,
            &form_map,
            &lowered.signal_name,
            counter,
            span,
            decl.span,
        )?);
        return Ok(binds);
    }

    // --- The motion form (form slot): resolve its declared keyframes.
    // W3 review P2: form_application captures call args under `args` since
    // W3.3 — the earlier `form_params` read silently dropped statement-site
    // args (`--rise(distance: 12px);` emitted the default). One resolution
    // path for all three slots (statement, arm, body splice).
    let form_map: std::collections::HashMap<String, CapturedValue> = match &consequence {
        Some(CapturedValue::Named(cmap)) => {
            // on_consequence wraps the form_application once more:
            // {form: {form, args?}} — unwrap to the form map.
            match cmap.get("form") {
                Some(CapturedValue::Named(inner)) if inner.contains_key("form") => inner.clone(),
                _ => cmap.clone(),
            }
        }
        Some(other) => {
            // Flat shape (the transitional space sibling): the form name
            // captured directly at match level; the args capture sits beside.
            let mut map = std::collections::HashMap::new();
            map.insert("form".to_string(), other.clone());
            if let Some(args) = read("args") {
                map.insert("args".to_string(), args);
            }
            map
        }
        None => {
            return Err(drive_error(
                DiagnosticCode::E0948,
                span,
                "`drive` bind without a form — the statement shape is \
                 `@on <driver>: --form;`",
                "write `@on &.<member>: --<form>;` with a declared motion form",
            ));
        }
    };
    let (animations, _declared) = resolve_form_with_args(matches, registry, &form_map, span)?;

    // --- Synthesize the real binds.
    // A `name` driver param IS the timeline name (the cutover maps the old
    // directives' `$name` onto it — `@after` chains resolve by name, so the
    // synthetic name must not replace it).
    // Publication name (D24/D30): the `as` clause names the driver's
    // progress signal — a progress-TYPED binding (`$reveal.done` reads it).
    // The legacy `name:` param (the timelines registry) dies in W5.
    // W5a/D1: a `domain: inherited` driver (`.clip`) has NO CLOCK of its own —
    // its progress is the window an enclosing score already assigned to this
    // element. It must therefore address that window BY ELEMENT, through the
    // same rule the score used when it published it. A counter here would name
    // a signal nobody publishes, which compiles clean and never moves.
    let inherited = crate::pipeline::score::driver_inherits_progress(registry, &member)
        .then(|| crate::pipeline::score::inherited_progress_name(form_match.selector.as_deref()))
        .flatten();
    let drive_name = as_name
        .clone()
        .or_else(|| named_driver_param(driver_params.as_ref(), "name"))
        .or(inherited)
        .unwrap_or_else(|| format!("__drive_{member}_{counter}"));
    let anim_key = format!("__drive_anim_{counter}");
    let name_key = format!("__drive_name_{counter}");
    form_match
        .captures
        .insert(anim_key.clone(), CapturedValue::Keyframes(animations));
    form_match
        .captures
        .insert(name_key.clone(), CapturedValue::Ident(drive_name.clone()));

    // Driver primitive bind: `primitive(&self, <named scalar driver params>) -> { $<drive_name> }`
    // A SUBJECT (`@on &hero.visible …`) is the OBSERVED element — resolved
    // by name through the established `&name` rail (emit: ST.ref("hero")),
    // NOT a CSS descendant selector. The ANIMATED element stays &self (the
    // scope owner): hero's visibility drives the card's animation.
    let driver_el = match &subject {
        Some(s) => BindArg::Element {
            name: s.clone(),
            child_selector: None,
        },
        None => BindArg::Element {
            name: "self".to_string(),
            child_selector: None,
        },
    };
    let mut driver_args = vec![driver_el];
    // PUBLICATION ALIGNMENT (review A): the `%yield progress -> $%name`
    // drivers publish under the name PARAM — which the drive expansion never
    // passed, so nameless and `as`-named drivers published to `""` while
    // consumers watched the real drive_name (silently dead since W3; the
    // capsule's always-`name:` masked it). When the primitive HAS a `name`
    // param and the head didn't supply one, inject it: publication and
    // consumption share drive_name by construction.
    let primitive_has_name_param = registry
        .get_primitive(&primitive)
        .map(|p| {
            p.params.iter().any(|param| match param {
                crate::parser::meta_ast::PrimitiveParam::Typed { name, .. } => name == "name",
                crate::parser::meta_ast::PrimitiveParam::TypedData { name, .. } => name == "name",
                _ => false,
            })
        })
        .unwrap_or(false);
    let head_supplied_name = named_driver_param(driver_params.as_ref(), "name").is_some();
    if primitive_has_name_param && !head_supplied_name {
        driver_args.push(BindArg::Named {
            name: "name".to_string(),
            value: BindValue::String(drive_name.clone()),
        });
    }
    // A SIGNAL subject rides as the driver's `signal` arg (change-driver's
    // signature): the element stays &self — the scope node the signal
    // resolves through.
    if let Some(sig) = &ssubject {
        driver_args.push(BindArg::Named {
            name: "signal".to_string(),
            value: BindValue::String(sig.clone()),
        });
    }
    driver_args.extend(
        driver_param_args(
            driver_params.as_ref(),
            registry,
            &primitive,
            clause,
            Some(span),
        )
        .map_err(|(_, d)| d)?,
    );
    let interactive = matches!(
        registry.entry_field(clause, "interactive"),
        Some(RegisterValue::Bool(true))
    );
    // Fixed-yield primitives (no `name` param — their yield names a fixed
    // export like `$progress`) publish under that RAW name; the alias
    // channel (resolve's BUG-154 remap) renames it to the publication name.
    let progress_export = match registry.entry_field(clause, "progress_export") {
        Some(RegisterValue::Ident(e)) => e.clone(),
        _ => "progress".to_string(),
    };
    let driver_outputs = if primitive_has_name_param {
        vec![BindOutput {
            name: drive_name,
            alias: None,
        }]
    } else {
        vec![BindOutput {
            name: progress_export,
            alias: Some(drive_name),
        }]
    };
    let driver_bind = BindDecl {
        primitive,
        args: driver_args,
        outputs: driver_outputs,
        span: decl.span,
    };

    // apply-animations bind: the resolved form body under the synthetic name.
    // `interactive` forwards the driver registry's flag (event drivers:
    // hover/click/focus/key) — the retired binds passed `interactive: true`
    // for these, and apply-animations' runtime gates on it.
    let mut apply_args = vec![
        // The ANIMATED element is always &self (the scope owner) — only the
        // DRIVER follows the subject.
        BindArg::Element {
            name: "self".to_string(),
            child_selector: None,
        },
        BindArg::Named {
            name: "driver".to_string(),
            value: BindValue::Variable(name_key),
        },
        BindArg::Named {
            name: "animations".to_string(),
            value: BindValue::Variable(anim_key),
        },
    ];
    if interactive {
        apply_args.push(BindArg::Named {
            name: "interactive".to_string(),
            value: BindValue::Ident("true".to_string()),
        });
    }
    let apply_bind = BindDecl {
        primitive: "apply-animations".to_string(),
        args: apply_args,
        outputs: Vec::new(),
        span: decl.span,
    };

    Ok(vec![driver_bind, apply_bind])
}

/// Expand one `drive-arms(&self, signal: $signal, arms: $arms)` bind into a
/// `signal-arms` primitive call whose arms array is RESOLVED at compile time:
/// each arm's form looked up in the form registry, params substituted (call
/// args by name, else positional in declaration order, else defaults), the
/// duration/easing pulled out for the WAAPI play.
fn expand_arms(
    decl: &BindDecl,
    form_match: &mut FormMatch,
    matches: &[FormMatch],
    registry: &MetaRegistry,
    counter: usize,
    span: crate::parser::SourceSpan,
    evidence: &FacetEvidence,
) -> Result<Vec<BindDecl>, Diagnostic> {
    let arms_value = form_match.captures.get("arms").cloned().ok_or_else(|| {
        drive_error(
            DiagnosticCode::E0948,
            span,
            "`drive-arms` bind without arms (compiler bug — please report)",
            "expected an `$arms:on_arm+` capture",
        )
    })?;
    let CapturedValue::Array(arm_items) = arms_value else {
        return Err(drive_error(
            DiagnosticCode::E0948,
            span,
            "the arms capture is not a list (compiler bug — please report)",
            "expected `$arms:on_arm+` to yield an Array",
        ));
    };

    // --- Facet heads (PLAN-126 D26): `@on $reveal.done { … }`. The Binding
    // capture is GREEDY (dotted paths are one binding — the `$data.items`
    // convention), so the facet is recovered by splitting the LAST segment,
    // but only when that segment is a REGISTERED value facet (the collision
    // rule, showcase §5: known facet => facet; otherwise a dotted signal
    // path, status quo).
    let mut prelude: Vec<BindDecl> = Vec::new();
    let facet_signal_key = format!("__arms_signal_{counter}");
    let raw_signal = form_match.captures.get("signal").cloned();
    let signal_text = match &raw_signal {
        Some(CapturedValue::Binding(t)) => Some(t.clone()),
        Some(CapturedValue::Named(m)) => match m.get("base") {
            Some(CapturedValue::Binding(t)) => Some(t.clone()),
            other => text_of(other),
        },
        other => text_of(other.as_ref()),
    };
    if let Some(signal_text) = signal_text {
        let bare = signal_text.trim_start_matches('$').to_string();
        if let Some((base, last)) = bare.rsplit_once('.') {
            let entries = registry.entries_of("driver");
            if let Some((_, fclause)) = entries
                .iter()
                .find(|(_, c)| registry.entry_key(c) == Some(last))
            {
                let f_lowers = match registry.entry_field(fclause, "lowers_to") {
                    Some(RegisterValue::String(t)) => Some(t.clone()),
                    _ => None,
                };
                let f_prim = match registry.entry_field(fclause, "facet_primitive") {
                    Some(RegisterValue::Ident(p)) => Some(p.clone()),
                    _ => None,
                };
                // A PLANNED facet refuses loudly at the arms head too —
                // falling through to the dotted-path default would silently
                // watch the head signal instead (D2: known word, loud no).
                if let Some(RegisterValue::Ident(p)) = registry.entry_field(fclause, "primitive")
                    && let Some(planned) = p.strip_prefix("planned:")
                {
                    return Err(drive_error(
                        DiagnosticCode::E0948,
                        span,
                        format!(
                            "facet `.{last}` is registered but its primitive `{planned}` is PLANNED, not yet bound"
                        ),
                        "the word is known so the diagnostic can be precise; the primitive lands with its wave",
                    ));
                }
                if f_lowers.is_some() || f_prim.is_some() {
                    // §5 type evidence (review B): `.done` requires an
                    // `as`-bound progress publication; `.prev` requires a
                    // declared signal or an `as` binding. Untyped subjects
                    // ERROR — never silently watch the head.
                    let has_evidence = if last == "done" {
                        evidence.as_names.contains(base)
                    } else {
                        evidence.as_names.contains(base) || evidence.declared_signals.contains(base)
                    };
                    if !has_evidence {
                        return Err(drive_error(
                            DiagnosticCode::E0950,
                            span,
                            format!(
                                "`@on ${base}.{last} {{ … }}` — `.{last}` on an untyped subject is uninferrable"
                            ),
                            if last == "done" {
                                "publish the timeline first: `@on &.<driver>(…) as $reveal { … }` — then `@on $reveal.done { … }`"
                            } else {
                                "declare the signal (`$price number: 0;`) or name a driver publication with `as`"
                            },
                        ));
                    }
                    let lowered = lower_signal_facet(
                        form_match,
                        base,
                        last,
                        f_lowers.as_deref(),
                        f_prim.as_deref(),
                        counter,
                        span,
                    );
                    prelude = lowered.prelude;
                    form_match.captures.insert(
                        facet_signal_key.clone(),
                        CapturedValue::Binding(format!("${}", lowered.signal_name)),
                    );
                }
            }
        }
        // NORMALIZE the non-facet path. `signal_path` captures as a Named map
        // ({base, facet?}); splicing that map into the primitive's `signal`
        // param lowered to the JS object literal `{ base: "x" }`, so the
        // primitive's `String(%signal)` read `"[object Object]"` and every
        // plain `@on $sig { … }` watched a signal name that can never fire —
        // silently, with correct-looking arms beside it. The facet branch above
        // already re-keys to a flat Binding; this gives the ordinary path the
        // same shape, so exactly ONE value kind reaches the primitive.
        if !form_match.captures.contains_key(&facet_signal_key) {
            form_match.captures.insert(
                facet_signal_key.clone(),
                CapturedValue::Binding(format!("${bare}")),
            );
        }
    }
    let mut resolved_arms: Vec<CapturedValue> = Vec::new();
    for item in &arm_items {
        let CapturedValue::Named(arm) = item else {
            continue;
        };
        let match_spec = arm_match_spec(arm.get("pat"));
        let Some(CapturedValue::Named(form_map)) = arm.get("form") else {
            continue;
        };
        let form_name = text_of(form_map.get("form")).ok_or_else(|| {
            drive_error(
                DiagnosticCode::E0948,
                span,
                "an arm's consequence did not capture a form name (compiler bug — please report)",
                "expected a `dashed_ident` capture",
            )
        })?;

        let (mut animations, declared_params) =
            resolve_motion_form(matches, registry, &form_name, span)?;

        // Call args: named by name; positionals in declaration order.
        let call_args: Vec<(Option<String>, String)> = match form_map.get("args") {
            Some(CapturedValue::Array(items)) => items
                .iter()
                .filter_map(|i| {
                    if let CapturedValue::Named(a) = i {
                        let name = text_of(a.get("name"));
                        let value = match a.get("value") {
                            Some(CapturedValue::Expr(s)) => Some(s.clone()),
                            Some(CapturedValue::String(s)) | Some(CapturedValue::Ident(s)) => {
                                Some(s.clone())
                            }
                            _ => None,
                        };
                        value.map(|v| (name, v))
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        };
        let mut positional = call_args.iter().filter(|(n, _)| n.is_none());
        let overrides: std::collections::HashMap<String, Option<String>> = declared_params
            .iter()
            .map(|p| {
                let bare = p.name.trim_start_matches('$').to_string();
                let by_name = call_args
                    .iter()
                    .find(|(n, _)| n.as_deref() == Some(bare.as_str()))
                    .map(|(_, v)| v.clone());
                let value = by_name.or_else(|| positional.next().map(|(_, v)| v.clone()));
                (p.name.clone(), value)
            })
            .collect();
        substitute_params(
            &mut animations,
            Some(&CapturedValue::ParamList(declared_params.clone())),
            Some(&CapturedValue::Params(
                declared_params
                    .iter()
                    .map(|p| crate::syntax::ParamDef {
                        name: p.name.clone(),
                        type_ref: overrides
                            .get(&p.name)
                            .cloned()
                            .flatten()
                            .unwrap_or_default(),
                        default: None,
                    })
                    .collect(),
            )),
        );

        // Duration/easing for the WAAPI play: the resolved param of that name,
        // else the declared default, else the primitive's own default.
        let resolved = |want: &str| -> Option<String> {
            declared_params.iter().find_map(|p| {
                if p.name.trim_start_matches('$') == want {
                    overrides
                        .get(&p.name)
                        .cloned()
                        .flatten()
                        .filter(|v| !v.is_empty())
                        .or_else(|| p.default.clone())
                } else {
                    None
                }
            })
        };
        let duration = resolved("duration").map(|d| parse_duration_ms(&d));
        let easing = resolved("easing");

        let mut record = std::collections::HashMap::new();
        record.insert("match".to_string(), match_spec);
        // MATCH-TIME guard (D31-iii): `where <expr>` — transpiled with the
        // pattern's binding names mapping to the runtime's __old/__new
        // (coerced prev / new values); other $refs stay signal reads.
        if let Some(guard) = arm.get("guard") {
            let gtext = match guard {
                CapturedValue::Expr(e) => Some(e.clone()),
                other => text_of(Some(other)),
            };
            if let Some(gtext) = gtext.filter(|g| !g.trim().is_empty()) {
                let (from, to) = guard_binding_names(arm.get("pat"));
                record.insert(
                    "guard".to_string(),
                    CapturedValue::String(transpile_guard(&gtext, from.as_deref(), to.as_deref())),
                );
            }
        }
        record.insert(
            "animations".to_string(),
            CapturedValue::Keyframes(animations),
        );
        if let Some(d) = duration {
            record.insert("duration".to_string(), CapturedValue::Number(d));
        }
        if let Some(e) = easing {
            record.insert("easing".to_string(), CapturedValue::String(e));
        }
        resolved_arms.push(CapturedValue::Named(record));
    }

    let arms_key = format!("__arms_{counter}");
    form_match
        .captures
        .insert(arms_key.clone(), CapturedValue::Array(resolved_arms));

    // Re-emit the primitive with the resolved arms: `signal-arms(&self,
    // signal: <same binding>, arms: $<arms_key>)`. The signal arg passes
    // through as the original variable reference.
    let signal_arg = if form_match.captures.contains_key(&facet_signal_key) {
        BindValue::Variable(facet_signal_key.clone())
    } else {
        decl.args
            .iter()
            .find_map(|a| match a {
                BindArg::Named { name, value } if name == "signal" => Some(value.clone()),
                _ => None,
            })
            .unwrap_or(BindValue::Variable("signal".to_string()))
    };

    let mut out = prelude;
    out.push(BindDecl {
        primitive: "signal-arms".to_string(),
        args: vec![
            BindArg::Element {
                name: "self".to_string(),
                child_selector: None,
            },
            BindArg::Named {
                name: "signal".to_string(),
                value: signal_arg,
            },
            BindArg::Named {
                name: "arms".to_string(),
                value: BindValue::Variable(arms_key),
            },
        ],
        outputs: Vec::new(),
        span: decl.span,
    });
    Ok(out)
}

/// The result of lowering a signal facet (`done`/`prev`) into a watchable
/// signal: prelude binds that PUBLISH the lowered value, plus the signal
/// name the arms/statements watch.
struct LoweredFacet {
    prelude: Vec<BindDecl>,
    signal_name: String,
}

/// Lower a value facet on a signal subject (PLAN-126 D26): `lowers_to`
/// entries go through `derived-signal` (a signal-expression template with
/// `%subject` substituted); `facet_primitive` entries emit their primitive
/// with the subject as the `signal` arg. Data-driven: a new value facet is
/// a registry row, never a Rust arm.
fn lower_signal_facet(
    form_match: &mut FormMatch,
    subject: &str,
    member: &str,
    lowers_to: Option<&str>,
    facet_primitive: Option<&str>,
    counter: usize,
    span: crate::parser::SourceSpan,
) -> LoweredFacet {
    let signal_name = format!("__facet_{member}_{counter}");
    if let Some(template) = lowers_to {
        let expr = template.replace("%subject", subject);
        let expr_key = format!("__facet_expr_{counter}");
        form_match
            .captures
            .insert(expr_key.clone(), CapturedValue::String(expr));
        return LoweredFacet {
            prelude: vec![BindDecl {
                primitive: "derived-signal".to_string(),
                args: vec![
                    BindArg::Named {
                        name: "name".to_string(),
                        value: BindValue::Ident(signal_name.clone()),
                    },
                    BindArg::Named {
                        name: "expr".to_string(),
                        value: BindValue::Variable(expr_key),
                    },
                    BindArg::Named {
                        name: "fold".to_string(),
                        value: BindValue::Ident("false".to_string()),
                    },
                ],
                outputs: vec![BindOutput {
                    name: "value".to_string(),
                    alias: Some(signal_name.clone()),
                }],
                span,
            }],
            signal_name,
        };
    }
    let prim = facet_primitive.expect("a value facet carries lowers_to or facet_primitive");
    LoweredFacet {
        prelude: vec![BindDecl {
            primitive: prim.to_string(),
            args: vec![
                BindArg::Element {
                    name: "self".to_string(),
                    child_selector: None,
                },
                BindArg::Named {
                    name: "signal".to_string(),
                    value: BindValue::String(subject.to_string()),
                },
            ],
            // prev-signal's export is `$prev`; the alias channel renames it
            // to the facet's signal name (review A's remap mechanism).
            outputs: vec![BindOutput {
                name: "prev".to_string(),
                alias: Some(signal_name.clone()),
            }],
            span,
        }],
        signal_name,
    }
}

/// The D30 bool sugar's other half: `@on $sig.<facet>: --form;` desugars to
/// ONE `true` arm on the lowered signal — resolved exactly like a written
/// arm (form lookup, params, duration/easing), so the sugar and the braced
/// shape share every mechanism.
fn expand_true_arm(
    form_match: &mut FormMatch,
    matches: &[FormMatch],
    registry: &MetaRegistry,
    form_map: &std::collections::HashMap<String, CapturedValue>,
    signal_name: &str,
    counter: usize,
    span: crate::parser::SourceSpan,
    decl_span: crate::parser::SourceSpan,
) -> Result<Vec<BindDecl>, Diagnostic> {
    let (animations, declared) = resolve_form_with_args(matches, registry, form_map, span)?;
    let mut record = std::collections::HashMap::new();
    record.insert(
        "match".to_string(),
        CapturedValue::String("true".to_string()),
    );
    record.insert(
        "animations".to_string(),
        CapturedValue::Keyframes(animations),
    );
    // Duration/easing from the form's declared params (review C: the sugar
    // must honor form metadata exactly like an explicit arm). Call args are
    // already substituted into the keyframes by resolve_form_with_args; the
    // play's transport comes from the declaration defaults.
    for p in &declared {
        let bare = p.name.trim_start_matches('$');
        let value = p.default.clone().or_else(|| text_of(form_map.get(bare)));
        match (bare, value) {
            ("duration", Some(d)) => {
                record.insert(
                    "duration".to_string(),
                    CapturedValue::Number(parse_duration_ms(&d)),
                );
            }
            ("easing", Some(e)) => {
                record.insert("easing".to_string(), CapturedValue::String(e));
            }
            _ => {}
        }
    }
    let arms_key = format!("__arms_{counter}");
    form_match.captures.insert(
        arms_key.clone(),
        CapturedValue::Array(vec![CapturedValue::Named(record)]),
    );
    let signal_key = format!("__arms_signal_{counter}");
    form_match.captures.insert(
        signal_key.clone(),
        CapturedValue::Binding(format!("${signal_name}")),
    );
    Ok(vec![BindDecl {
        primitive: "signal-arms".to_string(),
        args: vec![
            BindArg::Element {
                name: "self".to_string(),
                child_selector: None,
            },
            BindArg::Named {
                name: "signal".to_string(),
                value: BindValue::Variable(signal_key),
            },
            BindArg::Named {
                name: "arms".to_string(),
                value: BindValue::Variable(arms_key),
            },
        ],
        outputs: Vec::new(),
        span: decl_span,
    }])
}

/// The binding names a BINDING pattern declares (`$old -> $new` →
/// ("old", "new")) — the guard's `$old`/`$new` map to the runtime's
/// __old/__new through THESE names (a guard referencing an undeclared name
/// reads a SIGNAL, never the match values).
fn guard_binding_names(pat: Option<&CapturedValue>) -> (Option<String>, Option<String>) {
    let Some(CapturedValue::Named(pat)) = pat else {
        return (None, None);
    };
    let Some(CapturedValue::Named(bt)) = pat.get("bind_transition") else {
        return (None, None);
    };
    let name = |k: &str| text_of(bt.get(k)).map(|t| t.trim_start_matches('$').to_string());
    (name("from"), name("to"))
}

/// Transpile a guard expression (D31-iii): the pattern's binding names map
/// to __old/__new (the runtime's coerced prev/new); every other `$ref` is a
/// scoped signal read. Dotted paths stay member access (the
/// transpile_signal_expr_with convention).
fn transpile_guard(expr: &str, from: Option<&str>, to: Option<&str>) -> String {
    let bytes = expr.as_bytes();
    let mut out = String::with_capacity(expr.len() + 16);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' || c == '\'' || c == '`' {
            // String literals pass through untouched (no $ substitution inside).
            let quote = c;
            out.push(c);
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                out.push(ch);
                i += 1;
                if ch == '\\' && i < bytes.len() {
                    out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
                if ch == quote {
                    break;
                }
            }
            continue;
        }
        if c == '$' {
            let mut j = i + 1;
            while j < bytes.len()
                && ((bytes[j] as char).is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            let name = &expr[i + 1..j];
            if Some(name) == from {
                out.push_str("__old");
            } else if Some(name) == to {
                out.push_str("__new");
            } else {
                out.push_str(&crate::syntax::signal_read(
                    crate::syntax::SignalScope::Scoped,
                    name,
                ));
            }
            // Dotted path stays raw member access.
            i = j;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// The match spec for one arm's pattern: a leaf value (`"true"`, `"grid"`),
/// the wildcard (`_` → `"*"`), or a transition (`a -> b` → `[from, to]`).
fn arm_match_spec(pat: Option<&CapturedValue>) -> CapturedValue {
    let Some(CapturedValue::Named(pat)) = pat else {
        return CapturedValue::String("*".to_string());
    };
    // A BINDING pattern (`$old -> $new`, D31-iii) matches ANY transition —
    // the guard does the filtering; the bindings ride to the guard as
    // __old/__new.
    if pat.contains_key("bind_transition") {
        return CapturedValue::String("*".to_string());
    }
    if let Some(CapturedValue::Named(leaf)) = pat.get("leaf") {
        if let Some(v) = text_of(leaf.get("lit")) {
            return CapturedValue::String(v);
        }
        return CapturedValue::String("*".to_string());
    }
    if let Some(CapturedValue::Named(transition)) = pat.get("transition") {
        let from = leaf_text(transition.get("from"));
        let to = leaf_text(transition.get("to"));
        return CapturedValue::Array(vec![CapturedValue::String(from), CapturedValue::String(to)]);
    }
    CapturedValue::String("*".to_string())
}

fn leaf_text(leaf: Option<&CapturedValue>) -> String {
    if let Some(CapturedValue::Named(leaf)) = leaf {
        text_of(leaf.get("lit")).unwrap_or_else(|| "*".to_string())
    } else {
        "*".to_string()
    }
}

/// `280ms` → 280, `1.2s` → 1200, `6f` → 100 (frames at 60fps — the score's
/// frame unit, see the launch article's `--dissolve(6f)`).
fn parse_duration_ms(raw: &str) -> f64 {
    // ONE duration parser (G3 / PLAN-141 W4). This used to be a second
    // implementation that knew `ms`, `s` and `f` and nothing else, so `2m` meant
    // 120000ms canonically and 300ms here — one literal, two durations, and
    // nothing anywhere failed. That is GH-11's defect (three filter tables that
    // had drifted) in a different costume.
    //
    // The frame unit `f` moved INTO the canonical parser rather than justifying
    // a fork: a score duration authored in frames is still a duration, and the
    // reason to keep a private parser evaporates once the shared one knows the
    // unit.
    //
    // A bare number keeps meaning milliseconds, and an unparseable value keeps
    // falling back to 300ms — both preserved so this is a unification, not a
    // behavior change for anything that already worked.
    let raw = raw.trim();
    if let Some((ms, _unit)) = crate::syntax::conversions::parse_duration_with_unit(raw) {
        return ms as f64;
    }
    raw.parse().unwrap_or(300.0)
}

/// Resolve a motion form by name: registry check (kind must be motion), its
/// keyframes body, and its declared params. Shared by the `drive` and
/// `drive-arms` expansions.
fn resolve_motion_form(
    matches: &[FormMatch],
    registry: &MetaRegistry,
    form_name: &str,
    span: crate::parser::SourceSpan,
) -> Result<(Vec<KeyframeDef>, Vec<crate::syntax::TemplateParamDef>), Diagnostic> {
    let declaration = matches.iter().find(|m| {
        let is_form_macro = m
            .matched_macro
            .as_deref()
            .map(|mm| {
                registry
                    .get_macro(mm)
                    .and_then(|def| def.registers.as_ref())
                    .is_some_and(|r| r.name == "form")
            })
            .unwrap_or(false);
        is_form_macro && text_of(m.captures.get("name")).as_deref() == Some(form_name)
    });

    let Some(declaration) = declaration else {
        return Err(drive_error(
            DiagnosticCode::E0948,
            span,
            format!("unknown motion form `{form_name}` — no `@form` declaration registers it"),
            format!("declare it first: `@form motion {form_name} {{ … }}` — or check the spelling"),
        ));
    };

    let kind = text_of(declaration.captures.get("kind")).unwrap_or_default();
    // A CAMERA form (PLAN-150 W3) is MOTION with a reserved axis vocabulary.
    // It rides the same `@on` slot; the only difference is that its axes are
    // rewritten to `--cam-*` custom-property tracks (below), which the ordinary
    // keyframe engine animates and a `camera-rig` reader turns into a transform
    // — no CSS property named `position`/`fov` is ever written.
    if kind != "motion" && kind != "camera" {
        return Err(drive_error(
            DiagnosticCode::E0949,
            span,
            format!("`{form_name}` is a `{kind}` form — `@on`'s form slot takes a MOTION or CAMERA form"),
            "motion/camera forms drive; style forms splice into scopes, value forms into values",
        ));
    }

    let mut keyframes = match declaration.captures.get("body") {
        Some(CapturedValue::Keyframes(k)) => k.clone(),
        other => {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!("motion form `{form_name}` has no keyframes body captured ({other:?})"),
                "a motion form's body is `prop: from -> to;` lines",
            ));
        }
    };

    if kind == "camera" {
        keyframes = rewrite_camera_axes(keyframes, form_name, span)?;
    }

    let params = match declaration.captures.get("params") {
        Some(CapturedValue::ParamList(p)) => p.clone(),
        _ => Vec::new(),
    };

    Ok((keyframes, params))
}

/// Rewrite a CAMERA form's reserved axes into `--cam-*` custom-property tracks
/// (PLAN-150 W3). `position: x y z` decomposes into `--cam-x` / `--cam-y` /
/// `--cam-z` (each stop's matching component); `target: x y z` into
/// `--cam-tx/ty/tz`; scalar `fov` -> `--cam-fov`, `roll` -> `--cam-roll`.
/// Everything downstream then treats them as ordinary custom-property
/// keyframes (the W1 engine animates them; the `camera-rig` reader composes a
/// transform from them). An unknown axis is a hard error — the camera
/// vocabulary is closed.
fn rewrite_camera_axes(
    keyframes: Vec<KeyframeDef>,
    form_name: &str,
    span: crate::parser::SourceSpan,
) -> Result<Vec<KeyframeDef>, Diagnostic> {
    let mut out: Vec<KeyframeDef> = Vec::new();
    for kf in keyframes {
        let axes: &[(&str, usize)] = match kf.property.as_str() {
            "position" => &[("--cam-x", 0), ("--cam-y", 1), ("--cam-z", 2)],
            "target" => &[("--cam-tx", 0), ("--cam-ty", 1), ("--cam-tz", 2)],
            "fov" => {
                out.push(KeyframeDef { property: "--cam-fov".into(), values: kf.values, selector: kf.selector });
                continue;
            }
            "roll" => {
                out.push(KeyframeDef { property: "--cam-roll".into(), values: kf.values, selector: kf.selector });
                continue;
            }
            other => {
                return Err(drive_error(
                    DiagnosticCode::E0949,
                    span,
                    format!("`{form_name}`: `{other}` is not a camera axis"),
                    "a camera form's axes are `position` / `target` (x y z vectors), `fov`, `roll`",
                ));
            }
        };
        // decompose each vector stop into its component per axis
        for (prop, idx) in axes {
            let values: Vec<String> = kf
                .values
                .iter()
                .map(|v| {
                    v.split_whitespace()
                        .nth(*idx)
                        .unwrap_or("0")
                        .to_string()
                })
                .collect();
            out.push(KeyframeDef { property: (*prop).into(), values, selector: kf.selector.clone() });
        }
    }

    // The DOM rig: a STATIC transform on the host that reads the animated
    // `--cam-*` custom properties, so moving the camera moves the host's
    // contents (the inverse: camera +z pulls the scene away, so the scene
    // scales down; camera x/y pans, roll rotates). CSS re-evaluates the
    // `calc(var(…))` whenever a track updates — the same mechanism the @post
    // lens uses. This is the DOM APPROXIMATION; on a `@stage` canvas the axes
    // drive the real camera instead (film.st.md §4 fidelity note).
    //
    // `--cam-z` is the dolly: larger z = further back = smaller scale.
    // A perspective from `--cam-fov` gives the parallax feel.
    let static_transform = "perspective(calc((100 - var(--cam-fov, 38)) * 24px)) \
        translate3d(calc(var(--cam-x, 0) * -1px), calc(var(--cam-y, 0) * 1px), calc(var(--cam-z, 10) * 8px)) \
        rotateZ(calc(var(--cam-roll, 0) * 1deg))";
    out.push(KeyframeDef {
        property: "transform".into(),
        values: vec![static_transform.to_string()],
        selector: None,
    });
    out.push(KeyframeDef {
        property: "transform-origin".into(),
        values: vec!["center center".to_string()],
        selector: None,
    });
    Ok(out)
}

/// Expand an `entering-exiting` two-slot body (text-change): slots become
/// the primitive's enterKeyframes/exitKeyframes (ordinary motion lines, the
/// same arrow split as the main body loop); the driver params carry
/// duration/charStagger/staggerFrom (renamed via the entry's `param_map`).
/// No apply-animations — the primitive choreographs characters internally.
fn expand_two_slot_body(
    form_match: &mut FormMatch,
    registry: &MetaRegistry,
    primitive: &str,
    clause: &crate::parser::meta_ast::RegistersClause,
    items: &[CapturedValue],
    slot_of: fn(
        &std::collections::HashMap<String, CapturedValue>,
    ) -> Option<&std::collections::HashMap<String, CapturedValue>>,
    driver_params: Option<&CapturedValue>,
    subject: Option<&str>,
    span: crate::parser::SourceSpan,
    decl_span: crate::parser::SourceSpan,
) -> Result<Vec<BindDecl>, Diagnostic> {
    let mut slots: Vec<(String, Vec<KeyframeDef>)> = Vec::new();
    for item in items {
        let CapturedValue::Named(entry) = item else {
            continue;
        };
        let Some(slot) = slot_of(entry) else {
            return Err(drive_error(
                DiagnosticCode::E0950,
                span,
                "a `body: entering-exiting` driver takes ONLY `:entering { … }` / `:exiting { … }` slots",
                "plain keyframe lines belong to ordinary drivers (`&.visible`, `$sig.change`)",
            ));
        };
        let name = text_of(slot.get("slot")).unwrap_or_default();
        if name != "entering" && name != "exiting" {
            return Err(drive_error(
                DiagnosticCode::E0950,
                span,
                format!("`:{name} {{ … }}` — the slots are `entering` and `exiting`"),
                "the two-slot body kind is fixed: `:entering { … } :exiting { … }`",
            ));
        }
        let body_items = match slot.get("slot_body") {
            Some(CapturedValue::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        let mut keyframes: Vec<KeyframeDef> = Vec::new();
        for sub in &body_items {
            let CapturedValue::Named(sub) = sub else {
                continue;
            };
            // `$slot_body:motion_line*` repeats the type DIRECTLY — items are
            // Named{prop, value}, not the alternation's {line: …} wrapper.
            let line = match sub.get("line") {
                Some(CapturedValue::Named(l)) => l,
                _ => sub,
            };
            {
                let prop = text_of(line.get("prop")).unwrap_or_default();
                let value = match line.get("value") {
                    Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => s.clone(),
                    _ => continue,
                };
                // `easing: &x;` lines carry no arrow — they are the slot's
                // easing, not a keyframe (the zeystudios slots declare one;
                // build_keyframes_js_object lifts it to the IR's easing
                // field). Dropping them silently would regress the
                // @value-change migration's behavior-exactness.
                if prop == "easing" {
                    keyframes.push(KeyframeDef {
                        property: prop,
                        values: vec![value],
                        selector: None,
                    });
                    continue;
                }
                let values =
                    crate::syntax::events::extractors::custom::split_on_top_level_arrow(&value);
                if values.is_empty() {
                    continue;
                }
                keyframes.push(KeyframeDef {
                    property: prop,
                    values,
                    selector: None,
                });
            }
        }
        slots.push((name, keyframes));
    }
    if slots.is_empty() {
        return Err(drive_error(
            DiagnosticCode::E0950,
            span,
            "an entering-exiting body with no slots is empty — write `:entering { … }` or drop the directive",
            "the driver-only shape (`@on &.text-change { }`) animates nothing",
        ));
    }

    // E: the driver observes the SUBJECT when given (`@on &hero.text-change`
    // watches hero's text), exactly like the main body path.
    let mut args = vec![BindArg::Element {
        name: subject.unwrap_or("self").to_string(),
        child_selector: None,
    }];
    args.extend(
        driver_param_args(driver_params, registry, primitive, clause, Some(span))
            .map_err(|(_, d)| d)?,
    );
    // G: a duplicated slot is a grammar error, never first-wins-silently.
    let mut seen = std::collections::HashSet::new();
    for (name, _) in &slots {
        if !seen.insert(name.clone()) {
            return Err(drive_error(
                DiagnosticCode::E0950,
                span,
                format!("`:{name} {{ … }}` appears twice — each slot is declared once"),
                "merge the two blocks into one `:{name}` slot",
            ));
        }
    }
    for (name, keyframes) in slots {
        // H: an EMPTY slot emits NO arg — the runtime's default keyframes
        // fire (the retired form's optional-slot behavior, behavior-exact).
        // An empty payload is truthy in JS and would bypass the defaults.
        if keyframes.is_empty() {
            continue;
        }
        let param = if name == "entering" {
            "enterKeyframes"
        } else {
            "exitKeyframes"
        };
        let key = format!("__slot_{name}_{}", form_match.captures.len());
        form_match
            .captures
            .insert(key.clone(), CapturedValue::Keyframes(keyframes));
        args.push(BindArg::Named {
            name: param.to_string(),
            value: BindValue::Variable(key),
        });
    }
    Ok(vec![BindDecl {
        primitive: primitive.to_string(),
        args,
        outputs: Vec::new(),
        span: decl_span,
    }])
}

/// Reconstruct the ARG RUN from structured `call_arg*` captures so a call
/// statement survives runMutations' `splitArgs` on the other side: positional
/// values joined `, `, a `name: value` arg keeping its label.
///
/// Shared by both call forms in a driver body — the bare `$ping()` and the
/// member `$rows.insert(7)`. One reconstruction, so the two spellings cannot
/// drift in how their arguments survive the round trip.
fn reconstruct_arg_run(args: Option<&CapturedValue>) -> String {
    match args {
        Some(CapturedValue::Array(items)) => items
            .iter()
            .filter_map(|i| {
                let CapturedValue::Named(a) = i else {
                    return None;
                };
                let val = match a.get("value") {
                    Some(CapturedValue::Expr(s))
                    | Some(CapturedValue::String(s))
                    | Some(CapturedValue::Ident(s)) => s.clone(),
                    _ => return None,
                };
                Some(match a.get("name") {
                    Some(CapturedValue::Ident(n)) => format!("{n}: {val}"),
                    _ => val,
                })
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

/// Expand one `drive-body(&self, driver: $driver, body: $body)` bind: the
/// driver resolves through the registry (same as `drive`), and the body —
/// splices of declared motion forms interleaved with keyframe/setting lines —
/// lowers into ONE merged animations value. Source order is preserved.
fn expand_body(
    decl: &BindDecl,
    form_match: &mut FormMatch,
    matches: &[FormMatch],
    registry: &MetaRegistry,
    counter: usize,
    span: crate::parser::SourceSpan,
) -> Result<Vec<BindDecl>, Diagnostic> {
    // --- Driver half (same registry dispatch as `drive`).
    let nested = capture_named(form_match, "driver").cloned();
    let member = nested
        .as_ref()
        .and_then(|m| m.get("member"))
        .and_then(|v| text_of(Some(v)))
        .ok_or_else(|| {
            drive_error(
                DiagnosticCode::E0948,
                span,
                "`drive-body` bind without a driver member (compiler bug — please report)",
                "expected captures from `driver_expr`",
            )
        })?;
    let subject = nested
        .as_ref()
        .and_then(|m| m.get("subject"))
        .and_then(|v| text_of(Some(v)))
        .filter(|s| !s.is_empty());
    // PLAN-126: signal subjects ride the `$` rail; the `as` clause names the
    // publication (read from the match captures, beside `driver`).
    let ssubject = nested
        .as_ref()
        .and_then(|m| m.get("ssubject"))
        .and_then(|v| text_of(Some(v)))
        .filter(|s| !s.is_empty());
    let as_name = form_match
        .captures
        .get("as")
        .and_then(|v| match v {
            CapturedValue::Named(m) => m.get("name").cloned(),
            other => Some(other.clone()),
        })
        .and_then(|v| text_of(Some(&v)))
        .map(|n| n.trim_start_matches('$').to_string())
        .filter(|n| !n.is_empty());

    let entries = registry.entries_of("driver");
    let clause = entries
        .iter()
        .find(|(_, c)| registry.entry_key(c) == Some(member.as_str()))
        .map(|(_, c)| *c)
        .ok_or_else(|| {
            let mut known: Vec<&str> = entries
                .iter()
                .filter_map(|(_, c)| registry.entry_key(c))
                .collect();
            known.sort_unstable();
            drive_error(
                DiagnosticCode::E0948,
                span,
                format!("unknown driver projection `.{member}` — no `%registers driver({member})` entry exists"),
                format!(
                    "registered drivers: {} · a new driver is a `%registers driver(<name>)` clause — data, not syntax",
                    known.join(", ")
                ),
            )
        })?;
    // A VALUE FACET in body position is a type error, not a malformed
    // driver: bodies consume DRIVERS (arms consume values).
    let is_value_facet = registry.entry_field(clause, "lowers_to").is_some()
        || registry.entry_field(clause, "facet_primitive").is_some();
    let primitive = match registry.entry_field(clause, "primitive") {
        Some(RegisterValue::Ident(p)) => p.clone(),
        _ if is_value_facet => {
            return Err(drive_error(
                DiagnosticCode::E0950,
                span,
                format!(
                    "`@on $….{member} {{ … }}` — `.{member}` is a value facet; bodies consume DRIVERS"
                ),
                "arms consume values (`@on $sig.done { true => --form; }`); bodies need a driver (`&visible`, `$sig.change`)",
            ));
        }
        _ => {
            return Err(drive_error(
                DiagnosticCode::E0948,
                span,
                format!(
                    "driver `{member}` has no `primitive` field — its registry entry is malformed"
                ),
                "every driver entry in stdlib/macros/drivers.st declares `primitive:`",
            ));
        }
    };
    if let Some(planned) = primitive.strip_prefix("planned:") {
        return Err(drive_error(
            DiagnosticCode::E0948,
            span,
            format!(
                "driver `.{member}` is registered but its primitive `{planned}` is PLANNED, not yet bound"
            ),
            "the word is known so the diagnostic can be precise; the primitive lands with its wave",
        ));
    }
    let on_kinds = match registry.entry_field(clause, "on_kinds") {
        Some(RegisterValue::Ident(k)) => k.clone(),
        _ => "elem".to_string(),
    };
    if ssubject.is_some() && on_kinds != "signal" {
        return Err(drive_error(
            DiagnosticCode::E0949,
            span,
            format!(
                "projection `.{member}` applies to `{on_kinds}` refs, but `${}` is a signal",
                ssubject.as_deref().unwrap_or("")
            ),
            format!("give the body a `{on_kinds}` subject, e.g. `&<name>.{member}`"),
        ));
    }
    // BUG-265: the element-subject rejection for a SIGNAL-rail driver is
    // DEFERRED to after body classification — `&.change` / `&el.change` with a
    // MUTATION body is the DOM `change` event, a real form/select/input event.
    // The subject type decides the rail; only a non-mutation body on an element
    // subject is refused (see the check below the mutations loop).
    let interactive = matches!(
        registry.entry_field(clause, "interactive"),
        Some(RegisterValue::Bool(true))
    );

    // --- Body half: splices and lines merge into one animations value.
    // A genuinely EMPTY body (`@on &.scroll { }`) binds NO body capture at
    // all (the zero-item match records nothing) — that is the driver-only
    // case, not a compiler bug: default to an empty item list. (A body of
    // only comments binds, which is why this path was late to surface.)
    let body_value = form_match
        .captures
        .get("body")
        .cloned()
        .unwrap_or_else(|| CapturedValue::Array(Vec::new()));
    let CapturedValue::Array(items) = body_value else {
        return Err(drive_error(
            DiagnosticCode::E0948,
            span,
            "the motion body capture is not a list (compiler bug — please report)",
            "expected `$body:on_motion_body` to yield an Array",
        ));
    };

    // --- Two-slot bodies (showcase §7): `:entering { … }` / `:exiting { … }`
    // are legal ONLY on a driver whose registry entry declares
    // `body: entering-exiting` (text-change) — and on that driver they are
    // the ONLY legal items. Everything routes through the `body` field as
    // data; nothing here learns what "text-change" is.
    let body_kind = match registry.entry_field(clause, "body") {
        Some(RegisterValue::Ident(b)) => Some(b.clone()),
        _ => None,
    };
    fn slot_of<'x>(
        entry: &'x std::collections::HashMap<String, CapturedValue>,
    ) -> Option<&'x std::collections::HashMap<String, CapturedValue>> {
        entry.get("slot").and_then(|v| match v {
            CapturedValue::Named(m) => Some(m),
            _ => None,
        })
    }
    let has_slots = items
        .iter()
        .any(|i| matches!(i, CapturedValue::Named(e) if slot_of(e).is_some()));
    if has_slots && body_kind.as_deref() != Some("entering-exiting") {
        return Err(drive_error(
            DiagnosticCode::E0950,
            span,
            format!(
                "`@on &.{member} {{ :entering {{ … }} }}` — two-slot bodies belong to a `body: entering-exiting` driver"
            ),
            "only `text-change` declares that body kind today; plain drivers take keyframe lines",
        ));
    }
    if body_kind.as_deref() == Some("entering-exiting") {
        // F: `as` names a progress publication — an event driver choreographing
        // characters publishes none. Loud, never silently ignored.
        if as_name.is_some() {
            return Err(drive_error(
                DiagnosticCode::E0950,
                span,
                "`as` names a driver's progress publication — `text-change` is an event driver with no progress to publish",
                "drop the `as` clause; read the trigger signal directly if you need the value",
            ));
        }
        return expand_two_slot_body(
            form_match,
            registry,
            &primitive,
            clause,
            &items,
            slot_of,
            nested.as_ref().and_then(|m| m.get("driver_params")),
            subject.as_deref(),
            span,
            decl.span,
        );
    }

    let mut animations: Vec<KeyframeDef> = Vec::new();
    // Settings (`stagger: 70ms;`, `easing: --x;`) are apply-animations ARGS,
    // not keyframes — the root-level builder DROPS setting props inside an
    // animations value. The setting set is the primitive's own signature
    // (data), minus its structural params.
    let setting_names: Vec<String> = registry
        .get_primitive("apply-animations")
        .map(|p| {
            p.params
                .iter()
                .filter_map(|param| match param {
                    crate::parser::meta_ast::PrimitiveParam::Typed { name, .. } => {
                        Some(name.clone())
                    }
                    crate::parser::meta_ast::PrimitiveParam::TypedData { name, .. } => {
                        Some(name.clone())
                    }
                    _ => None,
                })
                .filter(|n| n != "driver" && n != "animations")
                .collect()
        })
        .unwrap_or_else(|| vec!["easing".into(), "stagger".into(), "staggerFrom".into()]);
    let mut settings: Vec<(String, String)> = Vec::new();
    // Mutation items (`$open <- !$open;`) route to the on-mutation-handler
    // primitive as raw action text — they need an EVENT driver to fire on.
    let mut mutations: Vec<String> = Vec::new();
    for item in &items {
        let CapturedValue::Named(entry) = item else {
            continue;
        };
        if let Some(CapturedValue::Named(splice_map)) = entry.get("splice") {
            // splice = Named{ form: Named{ form: Ident, args: [...] } } — the
            // on_splice capture wraps the form_application one more level.
            let form_map = match splice_map.get("form") {
                Some(CapturedValue::Named(m)) => m,
                _ => splice_map,
            };
            let (keyframes, _params) = resolve_form_with_args(matches, registry, form_map, span)?;
            animations.extend(keyframes);
        } else if let Some(CapturedValue::Named(line)) = entry.get("line") {
            let prop = text_of(line.get("prop")).unwrap_or_default();
            let value = match line.get("value") {
                Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => s.clone(),
                _ => continue,
            };
            if setting_names.iter().any(|n| *n == prop) && !value.contains("->") {
                settings.push((prop, value));
                continue;
            }
            let values =
                crate::syntax::events::extractors::custom::split_on_top_level_arrow(&value);
            if values.is_empty() {
                continue;
            }
            animations.push(KeyframeDef {
                property: prop,
                values,
                selector: None,
            });
        } else if let Some(CapturedValue::Named(bad)) = entry.get("bad") {
            // BUG-346: a CSS PROPERTY written where a signal mutation belongs.
            //
            // `<-` is legitimate in two places, and they are different scopes:
            //   `.a { text <- $n; }`              reactive BINDING (selector scope)
            //   `.a { @on &.click { $n <- 1; } }` signal MUTATION (@on body)
            // A property inside an `@on` body is neither, and `ST.runMutations`
            // has no style-write path, so there is nothing to lower it to.
            //
            // Refused HERE, by name, because the alternative was worse in both
            // directions: the line fell through every `on_motion_body` shape and
            // surfaced as E0946 "expected capture `$body:body` but no tokens
            // remaining" (a complaint about the parser's position, for a mistake
            // on one line) — and on the `@on &.load` path the whole handler was
            // dropped silently instead, emitting a page that simply did nothing.
            return Err(property_mutation_error(bad, span));
        } else if let Some(CapturedValue::Named(mutation)) = entry.get("mut") {
            let target = text_of(mutation.get("target")).unwrap_or_default();
            let expr = match mutation.get("expr") {
                Some(CapturedValue::Expr(s)) => s.clone(),
                _ => continue,
            };
            mutations.push(format!("${target} <- {expr}"));
        } else if let Some(CapturedValue::Named(call)) = entry.get("call") {
            // A mutator call on a MEMBER — `$rows.insert(7);` — routed through
            // the SAME runMutations path as every other body statement. st.js
            // matches `$coll.method(args)` and applies it to ST._collections.
            //
            // Without this arm the capture was DROPPED silently and the body
            // fell through to the animation path, where the emitted driver
            // reached for `ST._raf` and died on a handler that had parsed fine.
            let sig = text_of(call.get("signal")).unwrap_or_default();
            let method = text_of(call.get("method")).unwrap_or_default();
            if sig.is_empty() || method.is_empty() {
                continue;
            }
            let arg_text = reconstruct_arg_run(call.get("args"));
            mutations.push(format!("${sig}.{method}({arg_text})"));
        } else if let Some(CapturedValue::Named(stmt)) = entry.get("stmt") {
            // A bare signal call — `$ping();` — is a mutation-consequence
            // statement like any `$dst <- $src`: route it through the SAME
            // runMutations path (st.js signalCallMatch fires the by-name
            // registered callable, signal.st:558). BUG-271 capture half.
            let sig = text_of(stmt.get("signal")).unwrap_or_default();
            if sig.is_empty() {
                continue;
            }
            let arg_text = reconstruct_arg_run(stmt.get("args"));
            mutations.push(format!("${sig}({arg_text})"));
        } else if let Some(CapturedValue::Array(nested)) = entry.get("scope") {
            // `.orb-1 { … }` / `& { … }` — scoped keyframes, inline in the
            // body array as { sel, scope: […] }; the emit groups them into
            // anims.scopes. A `&` scope has NO sel capture — it IS the
            // element, so its lines land at the root (selector: None).
            let sel = text_of(entry.get("sel")).filter(|s| !s.is_empty());
            for item in nested {
                let CapturedValue::Named(entry) = item else {
                    continue;
                };
                if let Some(CapturedValue::Named(bad)) = entry.get("bad") {
                    // BUG-346, nested half. `on_motion_body` is RECURSIVE, so the
                    // `$bad` alternative matches inside `.child { … }` and
                    // `& { … }` too. Handling it only in the top-level loop left
                    // a nested property mutation parsed, matched, and dropped on
                    // the floor — the exact silent-drop this refusal exists to
                    // remove, one level down. Same helper, so the two can't drift.
                    return Err(property_mutation_error(bad, span));
                }
                if let Some(CapturedValue::Named(line)) = entry.get("line") {
                    let prop = text_of(line.get("prop")).unwrap_or_default();
                    let value = match line.get("value") {
                        Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => s.clone(),
                        _ => continue,
                    };
                    let values =
                        crate::syntax::events::extractors::custom::split_on_top_level_arrow(&value);
                    if values.is_empty() {
                        continue;
                    }
                    animations.push(KeyframeDef {
                        property: prop,
                        values,
                        selector: sel.clone(),
                    });
                }
            }
        }
    }
    if !mutations.is_empty() {
        // A mutation fires on an event; a progress/transport driver cannot
        // fire anything — the same Gate/Event distinction the SIP draws.
        let value_type = match registry.entry_field(clause, "value_type") {
            Some(RegisterValue::Ident(v)) => v.clone(),
            _ => String::new(),
        };
        if value_type != "event" {
            return Err(drive_error(
                DiagnosticCode::E0949,
                span,
                format!(
                    "`@on &.{member} {{ $x <- … }}` — `.{member}` is a `{value_type}` driver; mutations need an EVENT driver"
                ),
                "event drivers: click, hover, focus, submit, key — or move the mutation into an arm of `@on $signal { … }`",
            ));
        }
        // BUG-265 routing: a MUTATION body under a SIGNAL subject
        // (`$sig.change`) is the SIGNAL rail — the change-driver runs
        // side-effects, never the DOM `change` event. The subject's SIGIL
        // decides: `$sig.change` is ALWAYS the signal rail; `&.change` /
        // `el.change` is the DOM event. (A signal subject with mutations can
        // only be `$….change` — it is the sole signal EVENT driver — so no
        // ambiguity needs naming.)
        if ssubject.is_some() {
            // A body is a mutation consequence OR a motion body, never both.
            // A mixed signal-head body is ambiguous (which rail does the
            // mutation ride?) and the change-driver has ONE consequence per
            // change, so refuse loudly rather than silently run one half.
            if !animations.is_empty() {
                let sig = ssubject.as_deref().unwrap_or("...");
                return Err(drive_error(
                    DiagnosticCode::E0949,
                    span,
                    format!(
                        "`@on ${sig}.{member} {{ … }}` — a signal-head body is a MUTATION consequence or a MOTION body, not both"
                    ),
                    "split them: run the mutation under `$<sig>.change` and the motion under its own driver",
                ));
            }
        }
    }

    // BUG-265 disambiguation, second half: `.change` is registered as a
    // SIGNAL-rail driver, but on an ELEMENT subject it is the DOM `change`
    // event (form select / input / file — a real event). The subject type
    // decides the rail: `$sig.change` is the signal rail; `&.change` /
    // `&el.change` is the DOM event. An element subject on a signal driver is
    // legal ONLY as a pure mutation consequence (routed to the DOM mutation
    // handler below); a motion or driver-only body has no element motion rail
    // for `.change` and is refused, as before.
    if ssubject.is_none() && on_kinds == "signal"
        && (mutations.is_empty() || !animations.is_empty())
    {
        let subj = match &subject {
            Some(s) => format!("&{s}"),
            None => format!("&.{member} (no subject)"),
        };
        return Err(drive_error(
            DiagnosticCode::E0949,
            span,
            format!(
                "projection `.{member}` applies to `signal` refs, but `{subj}` is an element subject"
            ),
            format!("give the projection a signal subject, e.g. `$<name>.{member}`"),
        ));
    }

    // An EMPTY body is legal: `@on &.scroll { }` is driver-only — the
    // driver's progress is consumable as a CSS var (the old surface compiled
    // it, e.g. webgl-landing's page-progress). Items that FAILED to parse
    // never reach here — the grammar rejects them (E0946), so emptiness here
    // is intent, not a typo.

    // Easing sigil: `--ease-out-expo` is the form-registry name; the runtime's
    // easing map is keyed BARE (`ease-out-expo`). Normalize at the boundary —
    // full easing-form resolution is its own wave (D12 in the report).
    // BUG-297: a `--name` NOTHING declares is a hard E0962 — the runtime
    // would silently fall back to `linear`, shipping different motion.
    for k in animations.iter_mut() {
        if k.property == "easing" {
            for v in k.values.iter_mut() {
                if v.starts_with("--") {
                    ensure_easing_form_declared(v, matches, registry, span)?;
                    *v = v.trim_start_matches("--").to_string();
                }
            }
        } else {
            // PLAN-150 W1: a PER-STEP easing (`0 -> 1 --ease-out-expo`) rides a
            // value stop. Validate the same way as a body `easing:` — an
            // undeclared `--name` is E0962 here, never a silent linear at
            // runtime. The trailing `--form` is peeled paren-depth-aware so a
            // `calc(…)` argument is never mistaken for one. (The stop text is
            // left intact; the shared keyframe serializer strips it.)
            for v in &k.values {
                if let Some(easing) = per_step_easing_of(v) {
                    ensure_easing_form_declared(&easing, matches, registry, span)?;
                }
            }
        }
    }

    // A `name` driver param IS the timeline name (the cutover maps the old
    // directives' `$name` onto it — `@after` chains resolve by name, so the
    // synthetic name must not replace it).
    // W5a/D1: a `domain: inherited` driver (`.clip`) has NO CLOCK of its own —
    // its progress is the window an enclosing score already assigned to this
    // element. It must therefore address that window BY ELEMENT, through the
    // same rule the score used when it published it. A counter here would name
    // a signal nobody publishes, which compiles clean and never moves. (The
    // form-call path above already routes through this; an inline keyframe
    // body is the same driver and must not drift to a second rule.)
    let inherited = crate::pipeline::score::driver_inherits_progress(registry, &member)
        .then(|| crate::pipeline::score::inherited_progress_name(form_match.selector.as_deref()))
        .flatten();
    let drive_name = as_name
        .clone()
        .or_else(|| {
            named_driver_param(nested.as_ref().and_then(|m| m.get("driver_params")), "name")
        })
        .or(inherited)
        .unwrap_or_else(|| format!("__drive_{member}_{counter}"));
    let anim_key = format!("__drive_anim_{counter}");
    let name_key = format!("__drive_name_{counter}");
    let has_animations = !animations.is_empty();
    form_match
        .captures
        .insert(anim_key.clone(), CapturedValue::Keyframes(animations));
    form_match
        .captures
        .insert(name_key.clone(), CapturedValue::Ident(drive_name.clone()));

    let mut out: Vec<BindDecl> = Vec::new();
    // The DRIVER bind is independent of animation presence: an empty body
    // (`@on &.scroll { }`) is driver-only — the driver's progress is
    // consumable as a CSS var (webgl-landing's page-progress pattern). A
    // MUTATION-ONLY body needs NO driver primitive — the mutation handler
    // attaches its own listener (ELEMENT rail). apply-animations is emitted
    // only when keyframes exist.
    //
    // EXCEPT the signal rail: a mutation body under `$sig.change` routes to
    // the change-driver (a mutation consequence, BUG-265), NOT to the DOM
    // mutation handler — the change-driver must be emitted even with no
    // keyframes.
    let mutation_only = !mutations.is_empty() && !has_animations;
    let signal_mutation = ssubject.is_some() && !mutations.is_empty();
    if !mutation_only || signal_mutation {
        // The DRIVER attaches to the SUBJECT when given (`@on &sidebar.scroll
        // { … }` — the sidebar's scroll drives this element's body).
        let body_driver_el = match &subject {
            Some(s) => BindArg::Element {
                name: s.clone(),
                child_selector: None,
            },
            None => BindArg::Element {
                name: "self".to_string(),
                child_selector: None,
            },
        };
        // PUBLICATION ALIGNMENT (review A — same mechanism as expand_one).
        let primitive_has_name_param = registry
            .get_primitive(&primitive)
            .map(|p| {
                p.params.iter().any(|param| match param {
                    crate::parser::meta_ast::PrimitiveParam::Typed { name, .. } => name == "name",
                    crate::parser::meta_ast::PrimitiveParam::TypedData { name, .. } => {
                        name == "name"
                    }
                    _ => false,
                })
            })
            .unwrap_or(false);
        let head_supplied_name =
            named_driver_param(nested.as_ref().and_then(|m| m.get("driver_params")), "name")
                .is_some();
        let mut body_driver_args = vec![body_driver_el];
        if primitive_has_name_param && !head_supplied_name {
            body_driver_args.push(BindArg::Named {
                name: "name".to_string(),
                value: BindValue::String(drive_name.clone()),
            });
        }
        if let Some(sig) = &ssubject {
            body_driver_args.push(BindArg::Named {
                name: "signal".to_string(),
                value: BindValue::String(sig.clone()),
            });
        }
        body_driver_args.extend(
            driver_param_args(
                nested.as_ref().and_then(|m| m.get("driver_params")),
                registry,
                &primitive,
                clause,
                Some(span),
            )
            .map_err(|(_, d)| d)?,
        );
        // BUG-265: the signal rail carries its MUTATION body as the
        // change-driver's `actions` arg — the change-driver runs the
        // side-effect on each signal change (immediate:/debounce: from the
        // head flow through driver_param_args above). Never a DOM listener.
        if signal_mutation {
            body_driver_args.push(BindArg::Named {
                name: "actions".to_string(),
                value: BindValue::String(mutations.join("; ")),
            });
        }
        let progress_export = match registry.entry_field(clause, "progress_export") {
            Some(RegisterValue::Ident(e)) => e.clone(),
            _ => "progress".to_string(),
        };
        let driver_outputs = if primitive_has_name_param {
            vec![BindOutput {
                name: drive_name,
                alias: None,
            }]
        } else {
            vec![BindOutput {
                name: progress_export,
                alias: Some(drive_name.clone()),
            }]
        };
        out.push(BindDecl {
            primitive: primitive.clone(),
            args: body_driver_args,
            outputs: driver_outputs,
            span: decl.span,
        });
    }
    if has_animations {
        // BUG-297: a `--name` easing driver-param nothing declares is a hard
        // E0962 here, never a silent `linear` fallback at runtime.
        for (setting_name, setting_value) in settings.iter() {
            if setting_name == "easing" && setting_value.starts_with("--") {
                ensure_easing_form_declared(setting_value, matches, registry, decl.span)?;
            }
        }
        out.push(BindDecl {
            primitive: "apply-animations".to_string(),
            args: vec![
                BindArg::Element {
                    name: "self".to_string(),
                    child_selector: None,
                },
                BindArg::Named {
                    name: "driver".to_string(),
                    value: BindValue::Variable(name_key),
                },
                BindArg::Named {
                    name: "animations".to_string(),
                    value: BindValue::Variable(anim_key),
                },
            ]
            // BUG-297: a `--name` easing driver-param nothing declares is a
            // hard E0962 here, never a silent `linear` fallback at runtime.
            .into_iter()
            .chain(settings.iter().map(|(name, value)| BindArg::Named {
                name: name.clone(),
                // Easing values carry the form sigil (`--ease-out-expo`); the
                // runtime easing map is keyed bare (same boundary rule as the
                // animations normalization below).
                value: setting_bind_value_typed(
                    registry,
                    name,
                    value.strip_prefix("--").unwrap_or(value),
                ),
            }))
            // `interactive` forwards the driver registry's flag (event
            // drivers) — the retired binds passed `interactive: true` and
            // apply-animations' runtime gates on it.
            .chain(interactive.then_some(BindArg::Named {
                name: "interactive".to_string(),
                value: BindValue::Ident("true".to_string()),
            }))
            .collect(),
            outputs: Vec::new(),
            span: decl.span,
        });
    }

    if !mutations.is_empty() && !signal_mutation {
        // The listener attaches to the SUBJECT when given (`@on &save.click
        // { $x <- … }` — the save button is clicked, THIS scope's signals
        // change). A SIGNAL-subject mutation is excluded here — it already
        // rode the change-driver's `actions` arg above (BUG-265); binding a
        // DOM `change` listener on top would double-fire on the DOM event.
        let mutation_el = match &subject {
            Some(s) => BindArg::Element {
                name: s.clone(),
                child_selector: None,
            },
            None => BindArg::Element {
                name: "self".to_string(),
                child_selector: None,
            },
        };
        check_mutation_driver_params(
            nested.as_ref().and_then(|m| m.get("driver_params")),
            registry,
            clause,
            "on-mutation-handler",
            span,
        )?;
        let mut mutation_args = vec![
            mutation_el,
            BindArg::Named {
                name: "event".to_string(),
                value: BindValue::Ident(member.clone()),
            },
            BindArg::Named {
                name: "actions".to_string(),
                value: BindValue::String(mutations.join("; ")),
            },
        ];
        // BUG-375: the body rail's half of the same forwarding. Both sites
        // hand-build these args, so both must forward — fixing one and not the
        // other is how BUG-346 and BUG-352 each escaped their first fix.
        mutation_args.extend(mutation_driver_param_args(
            nested.as_ref().and_then(|m| m.get("driver_params")),
            registry,
            clause,
        ));
        out.push(BindDecl {
            primitive: "on-mutation-handler".to_string(),
            args: mutation_args,
            outputs: Vec::new(),
            span: decl.span,
        });
    }
    Ok(out)
}

/// Resolve a form application (name + call args): registry lookup, kind check,
/// param substitution with call args by name, else positional in declaration
/// order, else declared defaults. Shared by `drive-arms` and `drive-body`.
fn resolve_form_with_args(
    matches: &[FormMatch],
    registry: &MetaRegistry,
    form_map: &std::collections::HashMap<String, CapturedValue>,
    span: crate::parser::SourceSpan,
) -> Result<(Vec<KeyframeDef>, Vec<crate::syntax::TemplateParamDef>), Diagnostic> {
    let form_name = text_of(form_map.get("form")).ok_or_else(|| {
        drive_error(
            DiagnosticCode::E0948,
            span,
            "a form application did not capture a form name (compiler bug — please report)",
            "expected a `dashed_ident` capture",
        )
    })?;
    let (mut animations, declared_params) =
        resolve_motion_form(matches, registry, &form_name, span)?;

    let call_args: Vec<(Option<String>, String)> = match form_map.get("args") {
        Some(CapturedValue::Array(items)) => items
            .iter()
            .filter_map(|i| {
                if let CapturedValue::Named(a) = i {
                    let name = text_of(a.get("name"));
                    let value = match a.get("value") {
                        Some(CapturedValue::Expr(s)) => Some(s.clone()),
                        Some(CapturedValue::String(s)) | Some(CapturedValue::Ident(s)) => {
                            Some(s.clone())
                        }
                        _ => None,
                    };
                    value.map(|v| (name, v))
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    };
    let mut positional = call_args.iter().filter(|(n, _)| n.is_none());
    let overrides: std::collections::HashMap<String, Option<String>> = declared_params
        .iter()
        .map(|p| {
            let bare = p.name.trim_start_matches('$').to_string();
            let by_name = call_args
                .iter()
                .find(|(n, _)| n.as_deref() == Some(bare.as_str()))
                .map(|(_, v)| v.clone());
            let value = by_name.or_else(|| positional.next().map(|(_, v)| v.clone()));
            (p.name.clone(), value)
        })
        .collect();
    substitute_params(
        &mut animations,
        Some(&CapturedValue::ParamList(declared_params.clone())),
        Some(&CapturedValue::Params(
            declared_params
                .iter()
                .map(|p| crate::syntax::ParamDef {
                    name: p.name.clone(),
                    type_ref: overrides
                        .get(&p.name)
                        .cloned()
                        .flatten()
                        .unwrap_or_default(),
                    default: None,
                })
                .collect(),
        )),
    );
    Ok((animations, declared_params))
}

/// Read a named DRIVER PARAM from the driver_params capture (the cutover's
/// `%into` rewrite passes the old directive's `$name` as `name: <old-name>`).
fn named_driver_param(params: Option<&CapturedValue>, want: &str) -> Option<String> {
    let Some(CapturedValue::Array(items)) = params else {
        return None;
    };
    items.iter().find_map(|i| {
        if let CapturedValue::Named(a) = i {
            if text_of(a.get("name")).as_deref() == Some(want) {
                return match a.get("value") {
                    Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => {
                        Some(s.trim_matches('"').to_string())
                    }
                    _ => None,
                };
            }
        }
        None
    })
}

/// Parse a `"a:b,c:d"` registry field into pairs.
///
/// Shared by `param_map` (rename a caller-supplied arg) and `primitive_args`
/// (supply a constant the author never writes) — one spelling for both, so a
/// row author learns the shape once.
pub(crate) fn registry_pairs(
    registry: &MetaRegistry,
    clause: &crate::parser::meta_ast::RegistersClause,
    field: &str,
) -> Vec<(String, String)> {
    match registry.entry_field(clause, field) {
        Some(RegisterValue::String(s)) => s
            .split(',')
            .filter_map(|pair| {
                pair.split_once(':')
                    .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Map a driver_params capture (call_arg records) onto primitive bind args:
/// named args by name, positionals in order, each TYPED by the driver
/// primitive's own signature (string params emit as strings — `scope: cover`
/// as a bare identifier would be an undefined JS variable) — plus the row's
/// own constant `primitive_args`.
///
/// TAKES THE RESOLVED ROW, never a primitive name (BUG-253). It previously
/// searched `entries_of("driver")` for the first entry whose `primitive`
/// matched — and `.find` returns the FIRST match, so all five rows declaring
/// `primitive: event-driver` read *hover's* row. `.click` therefore emitted
/// hover's trigger and listened for mouseenter. Passing the row in is what
/// makes that class impossible: there is no longer any code path that can
/// reach a sibling row's data.
/// Shared with `score`: the registry-derived driver args for a projection.
///
/// Exposed (crate-visible) rather than copied, because a score's driver and an
/// `@on`'s driver ARE the same thing — the same registry row, the same param
/// map, the same row constants. Two readers of one row is the arrangement
/// BUG-253 punished; two COPIES of the reader would be worse.
pub(crate) fn driver_param_args_for(
    params: Option<&CapturedValue>,
    registry: &MetaRegistry,
    primitive: &str,
    clause: &crate::parser::meta_ast::RegistersClause,
) -> Vec<BindArg> {
    driver_param_args(params, registry, primitive, clause, None)
        .unwrap_or_else(|(args, _)| args)
}

/// Bind a driver's author-supplied args to its primitive's params.
///
/// BUG-352: returns `Err((args_so_far, diagnostic))` when a NAMED arg survives
/// the `param_map` rename without matching any param the primitive declares.
/// The partial args ride along so a caller with nowhere to put a diagnostic can
/// still behave exactly as before — the refusal is added at the call sites that
/// CAN report it, without silently changing the ones that cannot.
///
/// `span` is what makes the error pointable; `None` means the caller has no
/// span to attribute and therefore does not want the check.
fn driver_param_args(
    params: Option<&CapturedValue>,
    registry: &MetaRegistry,
    primitive: &str,
    clause: &crate::parser::meta_ast::RegistersClause,
    span: Option<crate::parser::SourceSpan>,
) -> Result<Vec<BindArg>, (Vec<BindArg>, Diagnostic)> {
    // Constants the ROW supplies to a shared primitive (`trigger:click`).
    // Five event rows share one primitive; this is what tells them apart.
    let row_args = registry_pairs(registry, clause, "primitive_args");
    let emit_row_args = |taken: &[String]| -> Vec<BindArg> {
        row_args
            .iter()
            // A row constant is a DEFAULT, not an override: an author who
            // writes the param explicitly wins.
            .filter(|(name, _)| !taken.iter().any(|t| t == name))
            .map(|(name, value)| BindArg::Named {
                value: typed_primitive_arg(registry, primitive, name, value),
                name: name.clone(),
            })
            .collect()
    };
    let Some(CapturedValue::Array(items)) = params else {
        return Ok(emit_row_args(&[]));
    };
    let mut args: Vec<BindArg> = Vec::new();
    // Author-facing param names may differ from the primitive's signature
    // (text-change's `stagger`/`from` vs `charStagger`/`staggerFrom`): the
    // registry entry's `param_map` renames, as DATA — never a per-driver
    // Rust branch.
    let param_map: Vec<(String, String)> = registry_pairs(registry, clause, "param_map");
    // An UNNAMED arg (`@on &.time(300ms)`) is bound BY NAME to the param the
    // registry entry declares for that slot — as DATA (`positional_params:
    // "duration"`), the same shape as `param_map`, never a Rust branch per
    // driver. Binding by INDEX instead (BUG-322) put the value in whatever
    // param happened to come first in the PRIMITIVE's signature: `300ms`
    // became `time-driver`'s `name`, so the driver published progress under
    // the signal name "300" while the animation watched `__drive_time_N` and
    // nothing ever moved. A driver's author-facing param list and its
    // primitive's signature are different vocabularies — the registry is what
    // knows the mapping.
    let positional_params: Vec<String> = match registry.entry_field(clause, "positional_params") {
        Some(RegisterValue::String(t)) => t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(RegisterValue::Ident(t)) => vec![t.clone()],
        _ => Vec::new(),
    };
    let mut positional_slot = 0usize;
    for item in items {
        let CapturedValue::Named(a) = item else {
            continue;
        };
        let value = match a.get("value") {
            Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => s.clone(),
            _ => continue,
        };
        match text_of(a.get("name")) {
            Some(name) => {
                let name = param_map
                    .iter()
                    .find(|(from, _)| *from == name)
                    .map(|(_, to)| to.clone())
                    .unwrap_or(name);
                // BUG-352: this `unwrap_or` used to be the whole story — an
                // unmapped name was bound to a param nothing declares and
                // vanished. `from:`, `once:` and every typo were absorbed in
                // silence, provably: building with and without the clause gave
                // byte-identical output. The name must now BE something.
                if let Some(span) = span {
                    // `None` = no declaration to check against (stay permissive);
                    // `Some(empty)` = declares NO params, so any named param is
                    // wrong. Conflating the two is what let `@on &.clip(bogus: 1)`
                    // through — `clip` registers `primitive: none`.
                    if let Some(declared) = declared_primitive_params(registry, primitive) {
                        if !declared.iter().any(|d| *d == name) {
                            let driver =
                                registry.entry_key(clause).unwrap_or(primitive).to_string();
                            return Err((
                                args,
                                unknown_param_error(&name, &driver, &declared, span),
                            ));
                        }
                    }
                    // BUG-367: the name is declared — now does the VALUE agree
                    // with the TYPE it was declared with? Same registry read,
                    // same loop, one step later. Before this, `start: #FF0020`
                    // bound a colour into a `number` param in silence.
                    if let Some(diag) = param_type_error(
                        registry,
                        primitive,
                        &name,
                        &value,
                        registry.entry_key(clause).unwrap_or(primitive),
                        span,
                    ) {
                        return Err((args, diag));
                    }
                }
                args.push(BindArg::Named {
                    value: typed_primitive_arg(registry, primitive, &name, &value),
                    name,
                })
            }
            None => {
                let slot = positional_params.get(positional_slot).cloned();
                positional_slot += 1;
                match slot {
                    Some(name) => {
                        let name = param_map
                            .iter()
                            .find(|(from, _)| *from == name)
                            .map(|(_, to)| to.clone())
                            .unwrap_or(name);
                        args.push(BindArg::Named {
                            value: typed_primitive_arg(registry, primitive, &name, &value),
                            name,
                        })
                    }
                    // No declared slot: keep the historical positional bind.
                    // A driver that takes unnamed args declares them; one that
                    // does not is unchanged by this rule.
                    None => args.push(BindArg::Positional(setting_bind_value(&value))),
                }
            }
        }
    }
    let authored: Vec<String> = args
        .iter()
        .filter_map(|a| match a {
            BindArg::Named { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let mut out = emit_row_args(&authored);
    out.extend(args);
    Ok(out)
}

/// BUG-352 — the params a primitive DECLARES, as data.
///
/// This is what makes the refusal registry-driven: a newly registered driver is
/// validated the moment it names a primitive, with no Rust arm per driver. The
/// same `registry.get_primitive(...).params` the typing path already consults.
fn declared_primitive_params(registry: &MetaRegistry, primitive: &str) -> Option<Vec<String>> {
    use crate::parser::meta_ast::PrimitiveParam;
    let Some(p) = registry.get_primitive(primitive) else {
        // No primitive in the registry means no declaration to check against.
        // Refusing here would turn an unrelated registry gap into a param error
        // and blame the author for it, so this stays permissive on purpose — an
        // unknown PRIMITIVE is already E0956's job.
        //
        // `None` rather than an empty Vec, because the two are DIFFERENT answers
        // and conflating them was a real hole reviewers found: `clip` registers
        // `primitive: none` — a sentinel with no declaration — so bailing on an
        // empty list let `@on &.clip(bogus: 1)` sail through silently.
        // "I cannot check" is not "this takes no params"; the latter must still
        // refuse, and now does.
        return None;
    };
    Some(
        p.params
            .iter()
            .filter_map(|param| match param {
                PrimitiveParam::Typed { name, .. } | PrimitiveParam::TypedData { name, .. } => {
                    Some(name.clone())
                }
                PrimitiveParam::Data(name) => Some(name.clone()),
                // Structural, never author-supplied by name (`&el` is the
                // element the scope already selected).
                PrimitiveParam::Element(_) => None,
            })
            .collect(),
    )
}

/// BUG-352 — the mutation rail's half of the same refusal.
///
/// A MUTATION body (`@on &.click(nonsense: 1) { $n <- 1; }`) never reaches
/// `driver_param_args`: both mutation sites hand-build their `on-mutation-handler`
/// args and simply do not read `driver_params`, so an author's params were
/// dropped one step EARLIER than the `unwrap_or` this bug is named for. Fixing
/// only the motion rail would have left `@on &.click(from: ".item") { $n <- 1; }`
/// — the exact spelling in BUG-334 — still silent.
///
/// Shared by both mutation sites on purpose: the same two-loops-one-refusal
/// shape that BUG-346 got wrong first, where the nested case kept dropping
/// silently after the top-level one was fixed.
fn check_mutation_driver_params(
    driver_params: Option<&CapturedValue>,
    registry: &MetaRegistry,
    clause: &crate::parser::meta_ast::RegistersClause,
    primitive: &str,
    span: crate::parser::SourceSpan,
) -> Result<(), Diagnostic> {
    let Some(CapturedValue::Array(items)) = driver_params else {
        return Ok(());
    };
    let param_map: Vec<(String, String)> = registry_pairs(registry, clause, "param_map");
    // What the AUTHOR may supply — NOT what the primitive declares.
    //
    // Reviewers caught the first version validating against the primitive's own
    // signature, which made this refusal actively harmful: both mutation sites
    // hand-build `event` and `actions` themselves and forward NOTHING the author
    // wrote, so `target:`/`event:`/`actions:` passed validation and were then
    // dropped — and the hint ADVERTISED `target` as accepted, recommending the
    // very silent drop this bug exists to kill. (`@on &.click(target: ".item")
    // { $n <- 1; }` still emitted `const delegateSelector = null`.)
    //
    // A param is author-consumable on this rail only if the registry row maps an
    // author name onto it. Today no event row declares a `param_map`, so the set
    // is empty and EVERY named param is refused — which is correct: this rail
    // takes none. The moment a row declares one, it is accepted, with no code
    // change here. Registry as the source of truth, per the elegance bar.
    let accepted: Vec<String> = param_map.iter().map(|(from, _)| from.clone()).collect();
    // BUG-375: a row may also declare POSITIONAL slots (`positional_params:
    // "key"`). Those name author-facing params too — `@on &.keydown(key:
    // "Enter")` and `@on &.keydown("Enter")` are the same binding written two
    // ways, so the named spelling must be accepted wherever the positional one
    // is. Reading BOTH fields here keeps the accept-set and the forward-set
    // (`mutation_driver_param_args`) derived from the same registry row.
    let positional: Vec<String> = match registry.entry_field(clause, "positional_params") {
        Some(RegisterValue::String(t)) => t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(RegisterValue::Ident(t)) => vec![t.clone()],
        _ => Vec::new(),
    };
    let accepted: Vec<String> = accepted.into_iter().chain(positional).collect();
    for item in items {
        let CapturedValue::Named(a) = item else {
            continue;
        };
        let Some(name) = text_of(a.get("name")) else {
            // An unnamed arg is bound by SLOT, not by name — it has no name to
            // check. Refusing it here would reject `@on &.keydown("Enter")`,
            // the spelling this rail exists to support.
            continue;
        };
        if !accepted.iter().any(|d| *d == name) {
            let driver = registry.entry_key(clause).unwrap_or(primitive).to_string();
            return Err(unknown_param_error(&name, &driver, &accepted, span));
        }
    }
    Ok(())
}

/// BUG-375 — the mutation rail's forwarding half.
///
/// `check_mutation_driver_params` above REFUSES what the author may not write;
/// this passes on what they may. The two read the SAME registry field
/// (`param_map`), so a row that declares a param both accepts it and delivers
/// it — they cannot drift into a state where one says yes and the other drops
/// the value, which is exactly the shape BUG-352 was: validation that
/// advertised `target` while the hand-built args forwarded nothing.
///
/// Returns the extra `BindArg`s to append to a hand-built `on-mutation-handler`
/// bind. The author-facing name is mapped to the primitive's own
/// (`key` → `key` today; the indirection is what lets a row rename).
fn mutation_driver_param_args(
    driver_params: Option<&CapturedValue>,
    registry: &MetaRegistry,
    clause: &crate::parser::meta_ast::RegistersClause,
) -> Vec<BindArg> {
    let Some(CapturedValue::Array(items)) = driver_params else {
        return Vec::new();
    };
    let param_map: Vec<(String, String)> = registry_pairs(registry, clause, "param_map");
    // An UNNAMED arg binds to the slot the row declares, exactly as the motion
    // rail does: `@on &.keydown("Enter")` is the common spelling and must not
    // require the author to write `key:` every time.
    let positional_params: Vec<String> = match registry.entry_field(clause, "positional_params") {
        Some(RegisterValue::String(t)) => t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(RegisterValue::Ident(t)) => vec![t.clone()],
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    let mut positional_slot = 0usize;
    for item in items {
        let CapturedValue::Named(a) = item else {
            continue;
        };
        let value = match a.get("value") {
            Some(CapturedValue::Expr(s)) | Some(CapturedValue::String(s)) => s.clone(),
            _ => continue,
        };
        let author_name = match text_of(a.get("name")) {
            Some(n) => Some(n),
            None => {
                let slot = positional_params.get(positional_slot).cloned();
                positional_slot += 1;
                slot
            }
        };
        let Some(author_name) = author_name else {
            continue;
        };
        let name = param_map
            .iter()
            .find(|(from, _)| *from == author_name)
            .map(|(_, to)| to.clone())
            .unwrap_or(author_name);
        out.push(BindArg::Named {
            // A quoted key (`"Enter"`) arrives with its quotes; the primitive
            // wants the VALUE. Stripping here keeps every call site identical.
            value: BindValue::String(value.trim_matches('"').to_string()),
            name,
        });
    }
    out
}

/// BUG-352 — an author-supplied param the driver's primitive does not declare.
///
/// Deliberately E0960, the SAME code a `@form` call site gets for the same
/// mistake (`--surface(padding: 3rem)` against a form without `$padding`).
/// This is one defect at two layers, so it is one diagnostic: an author who
/// learns the form message can read the driver message, and a new code would
/// have split one concept in two for no gain.
///
/// The refusal's value is the enumeration — a message that only says "no" moves
/// the guessing rather than ending it.
fn unknown_param_error(
    param: &str,
    driver: &str,
    accepted: &[String],
    span: crate::parser::SourceSpan,
) -> Diagnostic {
    let mut names: Vec<String> = accepted.to_vec();
    names.sort_unstable();
    names.dedup();
    let declared = if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    };
    drive_error(
        DiagnosticCode::E0960,
        span,
        format!(
            "unknown argument `{param}` for driver `{driver}` — declared parameters: {declared}"
        ),
        "check the spelling, or declare the parameter on the driver's primitive",
    )
}

/// The DECLARED TYPE of one named param, as its `%primitive` wrote it
/// (BUG-367 / FEAT-109 W4).
///
/// Sibling of `declared_primitive_params`, reading the same registry entry —
/// that function answers "is this param NAME declared?", this one answers
/// "what TYPE was it declared with?". Keeping them adjacent is deliberate: the
/// name check (BUG-352) and the type check are the same question asked of the
/// same declaration, and splitting them across modules is how the type half
/// ended up in a validator that never ran.
///
/// Only `Simple("...")` answers. A union (`("x" | "y")`) is an enumeration, not
/// a scalar type, and inference has nothing to say about it; an array or an
/// optional is a shape. Each of those returns `None` = "no scalar claim to
/// check", which is an abstention, not a pass.
fn declared_param_type(registry: &MetaRegistry, primitive: &str, param: &str) -> Option<String> {
    use crate::parser::meta_ast::{ParamType, PrimitiveParam};
    let p = registry.get_primitive(primitive)?;
    p.params.iter().find_map(|prm| match prm {
        PrimitiveParam::Typed {
            name,
            ty: ParamType::Simple(ty),
            ..
        } if name == param => Some(ty.clone()),
        PrimitiveParam::TypedData { name, ty } if name == param => Some(ty.clone()),
        _ => None,
    })
}

/// Refuse an argument whose VALUE contradicts the TYPE its parameter declares
/// (BUG-367 / FEAT-109 W4 — Tier 3).
///
/// # Why this lives here and not in a validator
///
/// It was written in `metasystem::validate::check_param_types`, proved by unit
/// gates, and had NO CALLER — which is precisely the defect BUG-367 was filed
/// about, reproduced by the fix for it. The bug's own subject,
/// `src/validator/functions.rs`, walked `ast.presets` (always empty, `@preset`
/// being retired) and matched on `Value::FunctionCall`, an enum variant nothing
/// in the codebase ever constructs. Two dead layers stacked on each other.
///
/// This is the surface where a declared parameter and an authored argument
/// genuinely meet at compile time: the same loop that already refuses an
/// unknown param NAME (BUG-352). One place, both halves of one question.
///
/// # The refusal discipline
///
/// Reported ONLY when inference positively names a scalar that contradicts the
/// declaration. Every other outcome is silent, and each silence is load-bearing:
///
/// - a REFERENCE (`--ink`, `$brand.duration`) — its type is a resolution
///   question answerable only elsewhere, possibly in another file; refusing it
///   would block a build over correct code;
/// - an ABSTENTION (`calc(100% - 3px)`, `1px solid red`) — silence is not
///   evidence of a mismatch. 30% of the corpus is values the grammar does not
///   model, and turning each into an error would make the check unusable;
/// - a bare number in a dimensional slot — CORRECT by Tier 2's rule, since the
///   slot supplies the unit the spelling omits;
/// - a DURATION literal in a `number` slot — `duration: number = 1000` is
///   declared numeric and authors write `600ms`. Both spellings mean the same
///   milliseconds; refusing one would refuse the corpus's own idiom. This is the
///   stagger lesson at the call site: the unit belongs to the DECLARATION, so a
///   value carrying the matching unit is agreement, not conflict.
///
/// A false refusal is worse than a missed diagnostic: one blocks a build with a
/// complaint the author cannot act on, the other costs five minutes.
fn param_type_error(
    registry: &MetaRegistry,
    primitive: &str,
    param: &str,
    raw: &str,
    driver: &str,
    span: crate::parser::SourceSpan,
) -> Option<Diagnostic> {
    use crate::types::value_infer::{Inferred, infer_value_type_in_context, inferrable_scalars};

    let declared = declared_param_type(registry, primitive, param)?;
    // A param declared with a type the language has no scalar row for is a fault
    // in the DECLARATION, and a different diagnostic owns it. Checking values
    // against it would report a mismatch for every argument.
    if !inferrable_scalars().contains(&declared) {
        return None;
    }

    let Inferred::Scalar(found) = infer_value_type_in_context(raw, Some(declared.as_str())) else {
        // Reference, Ambiguous, Unknown — no positive claim, so no refusal.
        return None;
    };
    if found == declared {
        return None;
    }
    // A time literal in a numeric time slot: see the discipline above.
    if declared == "number" && (found == "duration" || found == "time") {
        return None;
    }

    Some(drive_error(
        DiagnosticCode::E173,
        span,
        format!(
            "argument `{param}` of driver `{driver}` is declared `{declared}`, \
             but `{raw}` is a {found}"
        ),
        format!("pass a {declared} value, or change the parameter's declared type"),
    ))
}

/// A value typed by the NAMED primitive's signature (vs setting_bind_value_typed,
/// which is fixed to apply-animations).
fn typed_primitive_arg(
    registry: &MetaRegistry,
    primitive: &str,
    name: &str,
    raw: &str,
) -> BindValue {
    let is_string = registry.get_primitive(primitive).and_then(|p| {
        p.params.iter().find_map(|param| match param {
            crate::parser::meta_ast::PrimitiveParam::Typed { name: n, ty, .. } if n == name => {
                Some(format!("{ty:?}").to_lowercase().contains("string"))
            }
            _ => None,
        })
    });
    match is_string {
        Some(true) => BindValue::String(raw.to_string()),
        _ => setting_bind_value(raw),
    }
}

/// BUG-346: `<prop> <- value` in an `@on` body — the ONE place that refusal is
/// worded, shared by the top-level body loop and the nested-scope loop so the
/// two can never drift into disagreeing (or, as first written, into one of them
/// silently dropping the line).
fn property_mutation_error(
    bad: &std::collections::HashMap<String, CapturedValue>,
    span: crate::parser::SourceSpan,
) -> Diagnostic {
    let prop = text_of(bad.get("prop")).unwrap_or_default();
    // The RHS is whatever the author wrote, and the hint has to echo it back
    // verbatim to be worth reading. `text_of` only covers Expr/String/Ident, and
    // a bare `0.8` is none of those — without the wider match the hint would
    // degrade to `opacity: <from> -> ;`.
    let value = match bad.get("expr") {
        Some(CapturedValue::Expr(s)) => s.clone(),
        Some(CapturedValue::Number(n)) => n.to_string(),
        Some(CapturedValue::Bool(b)) => b.to_string(),
        Some(CapturedValue::Time(ms)) => format!("{ms}ms"),
        other => text_of(other).unwrap_or_else(|| "<value>".to_string()),
    };
    drive_error(
        DiagnosticCode::E0955,
        span,
        format!("`<-` assigns a signal, and `{prop}` is a CSS property, not one"),
        format!(
            "to ANIMATE it write `{prop}: <from> -> {value};`; to TRACK a value \
             declare a signal (`$name <type>: <initial>;`), mutate that \
             (`$name <- {value};`), and bind the property to it at selector \
             scope (`{prop}: var(--st-name);`)"
        ),
    )
}

fn drive_error(
    code: DiagnosticCode,
    span: crate::parser::SourceSpan,
    message: impl Into<String>,
    hint: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(code, message.into())
        .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end))
        .with_hint(hint.into())
}


/// BUG-297 — the D12 strip boundary's loud half: a `--name` easing reference
/// must name a DECLARED `@form easing` (the page, its imports, the project
/// overlay, or the stdlib curve library). The runtime easing map resolves by
/// bare name and falls back to `linear` on a miss, so an undeclared name
/// here compiled green and shipped different motion — the arc's banned
/// silent acceptance, now E0962.
/// Peel a trailing top-level `--<ident>` per-step easing off a keyframe stop
/// (`0 --ease-out-expo` -> Some("--ease-out-expo")). Paren-depth aware so a
/// `calc(…)` argument never trips it; a stop that is ENTIRELY a `--ref` (a
/// form splice, no value before it) returns None — its own path owns it.
/// Mirrors `metasystem::expand::peel_trailing_easing`, the serializer's copy;
/// both must agree on what a per-step easing is.
fn per_step_easing_of(stop: &str) -> Option<String> {
    let b = stop.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut last_ws: Option<usize> = None;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'"' && (i == 0 || b[i - 1] != b'\\') {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0 && c.is_ascii_whitespace() => last_ws = Some(i),
            _ => {}
        }
        i += 1;
    }
    let ws = last_ws?;
    let tail = stop[ws..].trim();
    let head = stop[..ws].trim();
    (tail.starts_with("--") && !head.is_empty()).then(|| tail.to_string())
}

fn ensure_easing_form_declared(
    raw: &str,
    matches: &[FormMatch],
    registry: &MetaRegistry,
    span: crate::parser::SourceSpan,
) -> Result<(), Diagnostic> {
    let bare = raw.trim_start_matches("--");
    // PLAN-150 W1: `--steps(N)` is a built-in parameterized easing (a stepped
    // curve of N discrete jumps) resolved by the runtime's getEasing, like
    // `cubic-bezier(…)`/`spring(…)`. Accept it here so a literal-arg step form
    // is not mistaken for an undeclared `@form easing`. (Author-declared
    // PARAM easing forms — `@form easing --steps($n)` splicing an arg — are a
    // separate resolution task; this covers the literal-count case the film
    // counter/typewriter need, e.g. `--steps(1800)`.)
    if let Some(arg) = bare.strip_prefix("steps(").and_then(|r| r.strip_suffix(')')) {
        if arg.trim().parse::<u32>().is_ok() {
            return Ok(());
        }
    }
    let is_easing_form = |m: &FormMatch| {
        registry
            .get_macro(m.matched_macro.as_deref().unwrap_or(&m.macro_name))
            .and_then(|def| def.registers.as_ref())
            .is_some_and(|r| r.name == "form")
            && text_of(m.captures.get("kind")).as_deref() == Some("easing")
    };
    let declared_here = matches.iter().any(|m| {
        is_easing_form(m)
            && text_of(m.captures.get("name"))
                .as_deref()
                .map(|n| n.trim_start_matches("--"))
                == Some(bare)
    });
    let declared_stdlib = STDLIB_FORM_DECLARATIONS
        .iter()
        .any(|(kind, name)| kind == "easing" && name.trim_start_matches("--") == bare);
    if declared_here || declared_stdlib {
        return Ok(());
    }
    let custom: Vec<String> = matches
        .iter()
        .filter(|m| is_easing_form(m))
        .filter_map(|m| text_of(m.captures.get("name")))
        .collect();
    let hint = if custom.is_empty() {
        "declare it first: `@form easing <name> { cubic-bezier(…) }` — or use the stdlib curve library (`--ease-out-expo`, `--spring-gentle`, …)".to_string()
    } else {
        format!(
            "declared on this page: {} · or the stdlib curve library (`--ease-out-expo`, `--spring-gentle`, …)",
            custom.join(", ")
        )
    };
    Err(drive_error(
        DiagnosticCode::E0962,
        span,
        format!("unknown easing form `{raw}` — no `@form easing` declaration registers it"),
        hint,
    ))
}

fn capture_named<'a>(
    form_match: &'a FormMatch,
    key: &str,
) -> Option<&'a std::collections::HashMap<String, CapturedValue>> {
    match form_match.captures.get(key) {
        Some(CapturedValue::Named(map)) => Some(map),
        _ => None,
    }
}

fn text_of(value: Option<&CapturedValue>) -> Option<String> {
    match value? {
        // `Selector` is what a `$sel:selector` capture yields — without it a
        // nested `.child { … }` motion scope loses its selector here and its
        // keyframes flatten into the root list, where a same-named property
        // collides with the root's own and wins per frame (last writer),
        // e.g. a scope's `opacity: 0 -> 1` silently overriding the root's
        // four-stop opacity. The emit side (expand.rs, BUG-201) already
        // groups selector-carrying KeyframeDefs into anims.scopes; this is
        // the read side that must not drop the address.
        CapturedValue::Ident(s)
        | CapturedValue::String(s)
        | CapturedValue::Binding(s)
        | CapturedValue::Selector(s) => Some(s.clone()),
        _ => None,
    }
}

/// Declaration defaults (`$distance = 24px`) substituted with call-site
/// overrides (`--rise(distance: 12px)`). Textual per-value substitution inside
/// the captured keyframes — the same level at which `%binds` substitution has
/// always worked.
fn substitute_params(
    keyframes: &mut [KeyframeDef],
    declared: Option<&CapturedValue>,
    call_site: Option<&CapturedValue>,
) {
    // Declaration params arrive as ParamList (param_list capture, the
    // post-FUP-040 declaration shape); a `params`-typed capture would be
    // Params. Accept both.
    let declared: Vec<(String, Option<String>)> = match declared {
        Some(CapturedValue::ParamList(p)) => p
            .iter()
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect(),
        Some(CapturedValue::Params(p)) => p
            .iter()
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect(),
        _ => Vec::new(),
    };
    let overrides: std::collections::HashMap<String, String> = match call_site {
        Some(CapturedValue::Params(p)) => p
            .iter()
            // An EMPTY override is no override: callers build a ParamDef per
            // declared param (unset positionals included), and an empty
            // type_ref would otherwise WIN over the declaration default and
            // emit an empty keyframe value (W3 review P1).
            .filter(|p| !p.type_ref.is_empty())
            .map(|p| (p.name.clone(), p.type_ref.clone()))
            .collect(),
        _ => Default::default(),
    };
    for (name, default) in declared {
        let value = overrides.get(&name).cloned().or(default);
        let Some(value) = value else { continue };
        let needle = format!("${name}");
        for keyframe in keyframes.iter_mut() {
            for v in keyframe.values.iter_mut() {
                *v = replace_param_token(v, &needle, &value);
            }
        }
    }
}

fn scalar_bind_value(raw: &str) -> BindValue {
    let raw = raw.trim();
    if let Ok(n) = raw.parse::<f64>() {
        return BindValue::Number(n);
    }
    if raw == "true" || raw == "false" {
        return BindValue::Ident(raw.to_string());
    }
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        return BindValue::String(raw[1..raw.len() - 1].to_string());
    }
    BindValue::Ident(raw.to_string())
}

/// A setting value, TYPED by the primitive's own signature: string-typed
/// params (`staggerFrom: "first"`) must emit as strings — an Ident would
/// reach the JS as a bare, undefined identifier. Number/time params parse to
/// the numeric value the signature declares (setting_bind_value).
fn setting_bind_value_typed(registry: &MetaRegistry, name: &str, raw: &str) -> BindValue {
    let is_string = registry
        .get_primitive("apply-animations")
        .and_then(|p| {
            p.params.iter().find_map(|param| match param {
                crate::parser::meta_ast::PrimitiveParam::Typed { name: n, ty, .. } if n == name => {
                    Some(format!("{ty:?}").to_lowercase().contains("string"))
                }
                _ => None,
            })
        })
        .unwrap_or(false);
    if is_string {
        return BindValue::String(raw.to_string());
    }
    setting_bind_value(raw)
}

/// A SETTING value: time-valued settings (`stagger: 70ms;`) must reach the
/// primitive as the NUMBER its signature declares (ms) — an Ident("70ms")
/// reaches the runtime as a string and computes NaN (W3 review P2).
fn setting_bind_value(raw: &str) -> BindValue {
    let raw = raw.trim();
    if raw.ends_with("ms") || raw.ends_with('s') || raw.ends_with('f') {
        return BindValue::Number(parse_duration_ms(raw));
    }
    scalar_bind_value(raw)
}

#[cfg(test)]
mod g3_duration_parity_tests {
    //! G3 — ONE MEANING FOR A DURATION LITERAL. (PLAN-141 W4: unified.)
    //!
    //! There used to be two duration parsers. `parse_duration_ms` here knew
    //! `ms`, `s` and `f`; `syntax::conversions::parse_duration_with_unit` knew
    //! `ms`, `s`, `us`, `m`, `h` and `fps`. So `2m` meant 120000ms canonically
    //! and 300ms (the fallback) through a driver — one literal, two durations,
    //! and nothing anywhere failed. That is GH-11's three-filter-tables defect
    //! in a different costume.
    //!
    //! These tests used to PIN that divergence so it was visible in the suite
    //! rather than in a user's page. The divergence is now gone: the frame unit
    //! `f` moved into the canonical parser (a score duration authored in frames
    //! is still a duration) and this module's parser delegates. What remains is
    //! the contract that keeps it gone.

    use super::parse_duration_ms;
    use crate::syntax::conversions::parse_duration_with_unit;

    /// Every unit means the same number through both entry points.
    ///
    /// `m` and `h` are the ones that used to diverge, so they are the ones that
    /// matter most here — but the list is deliberately the WHOLE unit set, so a
    /// unit added to one side and not the other fails immediately.
    #[test]
    fn every_unit_means_the_same_duration_through_both_entry_points() {
        for (lit, want_ms) in [
            ("300ms", 300.0),
            ("2s", 2000.0),
            ("0.5s", 500.0),
            ("2m", 120_000.0),
            ("1h", 3_600_000.0),
            ("6f", 100.0),
        ] {
            let driver = parse_duration_ms(lit);
            let canonical = parse_duration_with_unit(lit)
                .unwrap_or_else(|| panic!("the canonical parser rejected `{lit}`"))
                .0 as f64;

            assert!(
                (driver - want_ms).abs() < 1.0,
                "`{lit}` must be {want_ms}ms through a driver, got {driver}ms"
            );
            assert!(
                (driver - canonical).abs() < 1.0,
                "`{lit}`: driver says {driver}ms, canonical says {canonical}ms — \
                 one literal must not mean two durations"
            );
        }
    }

    /// The frame unit lives in the SHARED parser now, not in a private one.
    ///
    /// This is the assertion that stops the fork from growing back: if someone
    /// re-adds a local `f` branch and drops it from `conversions`, this fails.
    #[test]
    fn the_frame_unit_is_shared_not_forked() {
        assert_eq!(
            parse_duration_with_unit("6f").map(|(ms, _)| ms),
            Some(100),
            "6 frames at 60fps is 100ms, and the CANONICAL parser must be the one \
             that knows it — a private frame parser is how the two drifted before"
        );
    }

    /// Behavior that predates the unification and must survive it: a bare number
    /// is milliseconds, and an unparseable value falls back to 300ms rather than
    /// panicking or yielding zero.
    #[test]
    fn the_pre_existing_fallbacks_are_preserved() {
        assert!((parse_duration_ms("450") - 450.0).abs() < f64::EPSILON);
        assert!((parse_duration_ms("banana") - 300.0).abs() < f64::EPSILON);
    }
}
