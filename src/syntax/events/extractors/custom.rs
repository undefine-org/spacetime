//! Custom extractors — compile `%capture_type` pattern definitions into extractors.
//!
//! When stdlib registers a `%capture_type`, its `CapturePatternAst` is compiled
//! into a `Box<dyn CaptureExtractor>` that operates on token slices. This makes
//! the pattern system fully extensible from `.st` files.
//!
//! The compilation operates on `&[TokenData]` slices from the event-based parser.

use crate::parser::meta_ast::{CaptureModifier, CapturePatternAst, CaptureType};
use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::CapturedValue;

use super::{CaptureExtractor, ExtractResult, ExtractorRegistry, TokenData};

/// Compile a CapturePatternAst into a CaptureExtractor.
///
/// This is the entry point for registering custom capture types. When a
/// `%capture_type` definition is encountered in stdlib, its pattern AST
/// is compiled into an extractor that can be used like any built-in type.
pub fn compile_pattern(
    ast: &CapturePatternAst,
    registry: &ExtractorRegistry,
) -> Box<dyn CaptureExtractor> {
    compile_pattern_with_defs(
        ast,
        registry,
        &std::collections::HashMap::new(),
        &mut Vec::new(),
    )
}

/// Compile a pattern with access to sibling `%capture_type` definitions so a production can
/// reference another by name (types-on-types; PLAN-023 W2). `visiting` guards against
/// recursive cycles (a self-referential type compiles its recursive position as Expr).
pub fn compile_pattern_with_defs(
    ast: &CapturePatternAst,
    registry: &ExtractorRegistry,
    defs: &std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst>,
    visiting: &mut Vec<String>,
) -> Box<dyn CaptureExtractor> {
    match ast {
        CapturePatternAst::Capture {
            capture_type,
            modifier,
            ..
        } => {
            // Resolve a Custom (stdlib) reference by compiling the referenced production
            // inline from `defs` (types-on-types). Cycle-guarded; unresolved -> Expr.
            let inner = if let CaptureType::Custom(name) = capture_type {
                compile_custom_ref(name, registry, defs, visiting)
            } else {
                capture_type_to_extractor(capture_type, registry)
            };
            match modifier {
                CaptureModifier::Required => inner,
                CaptureModifier::Optional => Box::new(OptionalExtractor { inner }),
                CaptureModifier::ZeroOrMore => Box::new(RepeatExtractor {
                    inner,
                    at_least_one: false,
                }),
                CaptureModifier::OneOrMore => Box::new(RepeatExtractor {
                    inner,
                    at_least_one: true,
                }),
                // Counted repetition is a property of a CHARACTER CLASS terminal
                // (`[0-9a-f]{3|4|6|8}`), where it constrains a byte run. On a
                // `$name:type` capture there is no run to count — the modifier is
                // meaningless, so the capture matches once.
                CaptureModifier::Counted(_) => inner,
            }
        }

        CapturePatternAst::Literal(s) => Box::new(LiteralExtractor(s.clone())),

        CapturePatternAst::CharClass {
            chars,
            negated,
            counts,
        } => Box::new(CharClassExtractor {
            chars: chars.clone(),
            negated: *negated,
            counts: *counts,
        }),

        CapturePatternAst::Group { pattern, modifier } => {
            let inner = compile_pattern_with_defs(pattern, registry, defs, visiting);
            match modifier {
                None | Some(CaptureModifier::Required) => inner,
                Some(CaptureModifier::Optional) => Box::new(OptionalExtractor { inner }),
                Some(CaptureModifier::ZeroOrMore) => Box::new(RepeatExtractor {
                    inner,
                    at_least_one: false,
                }),
                Some(CaptureModifier::OneOrMore) => Box::new(RepeatExtractor {
                    inner,
                    at_least_one: true,
                }),
                // See the capture arm above: counted repetition belongs to a char
                // class, not to a group.
                Some(CaptureModifier::Counted(_)) => inner,
            }
        }

        CapturePatternAst::Sequence(elements) => {
            // Thread each element's capture NAME alongside its extractor so the sequence
            // can build a structured record (CapturedValue::Named). This is the foundation
            // of structured-output %capture_type (PLAN-023 W2 reifier boundary).
            let extractors: Vec<(Option<String>, Box<dyn CaptureExtractor>)> = elements
                .iter()
                .map(|e| {
                    (
                        capture_name_of(e),
                        compile_pattern_with_defs(e, registry, defs, visiting),
                    )
                })
                .collect();
            Box::new(SequenceExtractor(extractors))
        }

        CapturePatternAst::Choice(alternatives) => {
            // Each alternative carries the name of its (single) capture, when present, so a
            // Choice records WHICH branch matched (e.g. param_list's $name vs &name -> the
            // binding/element kind discriminator).
            let extractors: Vec<(Option<String>, Box<dyn CaptureExtractor>)> = alternatives
                .iter()
                .map(|a| {
                    (
                        capture_name_of(a),
                        compile_pattern_with_defs(a, registry, defs, visiting),
                    )
                })
                .collect();
            Box::new(ChoiceExtractor(extractors))
        }
    }
}

/// Compile a reference to another `%capture_type` by name, inlining its production from
/// `defs` (PLAN-023 W2 types-on-types). Recursion is cycle-guarded via `visiting`: a type
/// that references itself compiles the recursive position as Expr (avoids infinite compile).
/// An unknown name falls back to Expr.
fn compile_custom_ref(
    name: &str,
    registry: &ExtractorRegistry,
    defs: &std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst>,
    visiting: &mut Vec<String>,
) -> Box<dyn CaptureExtractor> {
    if visiting.iter().any(|n| n == name) {
        return Box::new(super::simple::ExprExtractor);
    }
    let Some(def) = defs.get(name) else {
        return Box::new(super::simple::ExprExtractor);
    };
    visiting.push(name.to_string());
    let inner = compile_pattern_with_defs(&def.pattern, registry, defs, visiting);
    visiting.pop();
    // A referenced type may itself be a reifying type (e.g. nested param_list); apply its
    // reifier so the typed shape is preserved through composition.
    wrap_reifier(name, inner)
}

/// Extract the capture variable name from a pattern, if it is (or wraps) a single named
/// capture. Used to build structured records: `$prop:ident` -> Some("prop"); a Group around
/// a single capture unwraps to that capture's name; sequences/choices/literals -> None.
fn capture_name_of(ast: &CapturePatternAst) -> Option<String> {
    match ast {
        CapturePatternAst::Capture { var_name, .. } => Some(var_name.clone()),
        // PLAN-150 W8: a REPEATED group whose body is a sequence with exactly
        // one named capture (`( "->" $next:score_step )*`) takes that capture's
        // name, so its RepeatExtractor's Array surfaces under `$next` in the
        // parent record. Without this the group is unnamed and the arrow chain
        // after the head is silently dropped (golden §9 sequencing gap). This is
        // SCOPED to repeated groups: an unmodified/optional group keeps the
        // prior behaviour, so handle-blocks, param_list, sum-type variants and
        // every other grouped capture are unaffected.
        CapturePatternAst::Group { pattern, modifier }
            if matches!(
                modifier,
                Some(CaptureModifier::ZeroOrMore) | Some(CaptureModifier::OneOrMore)
            ) =>
        {
            capture_name_of(pattern).or_else(|| sole_sequence_capture_name(pattern))
        }
        CapturePatternAst::Group { pattern, .. } => capture_name_of(pattern),
        _ => None,
    }
}

/// The single named capture of a SEQUENCE pattern, if it has exactly one.
/// Used only to name a repeated group (see `capture_name_of`).
fn sole_sequence_capture_name(ast: &CapturePatternAst) -> Option<String> {
    if let CapturePatternAst::Sequence(elements) = ast {
        let mut names = elements.iter().filter_map(capture_name_of);
        match (names.next(), names.next()) {
            (Some(only), None) => Some(only),
            _ => None,
        }
    } else {
        None
    }
}

/// Public accessor for the inner capture name of a pattern (FEAT-104). A body group
/// that wraps a SINGLE named capture (`( $s:string )?`, `( $row:ident )*`) compiles
/// to a scalar/Array rather than a Named record, so the form matcher needs the inner
/// name to bind the value. Sequences/choices/literals → None (they self-name via the
/// Named record path).
pub fn pattern_capture_name(ast: &CapturePatternAst) -> Option<String> {
    capture_name_of(ast)
}

/// Wrap a compiled stdlib extractor with a REIFIER when one exists for `name` (PLAN-023 W2
/// reifier boundary). A reifier maps the generic parse record (Named/Array<Named>) back into
/// the domain-typed CapturedValue (Properties/Params/Keyframes/ParamList) that the ~43
/// downstream consumers expect. When no reifier exists, the extractor is returned unchanged.
/// The productions whose captured value is their SOURCE TEXT — read from the
/// `%capture` column of `stdlib/scalars/types.st`, never from a list in Rust.
///
/// FEAT-168 made that table the single source for what a scalar MEANS (schema,
/// zero, widget). This is the fourth question — what SHAPE its value takes —
/// and it was the one place still answered by a hardcoded match arm, which made
/// it a seventh hand-synced scalar list and silently broke the wave's promise:
/// a scalar added purely as stdlib data reified to a RECORD of its grammar's
/// internal parts (`{n: 96, u: "dpi"}`) instead of the string `"96dpi"`.
///
/// The closure over the grammar is deliberate. A scalar's production is built
/// from named parts so it can VALIDATE (`( $n:number $u:length_unit )`), and
/// those parts are an implementation detail of the check, not the value — a
/// scalar that survived its grammar is exactly the text the author wrote. The
/// unit terminals themselves (`length_unit`, …) are NOT scalars and are absent
/// from the table, so they keep their own extractors.
pub(crate) fn is_scalar_capture(name: &str) -> bool {
    crate::syntax::stdlib_registry::STDLIB_REGISTRY
        .scalar_captures()
        .contains(name)
}

pub fn wrap_reifier(name: &str, inner: Box<dyn CaptureExtractor>) -> Box<dyn CaptureExtractor> {
    match name {
        "param_list" => Box::new(ReifyingExtractor {
            inner,
            reify: reify_param_list,
        }),
        "properties" | "fields" => Box::new(ReifyingExtractor {
            inner,
            reify: reify_properties,
        }),
        "params" => Box::new(ReifyingExtractor {
            inner,
            reify: reify_params,
        }),
        "keyframes" => Box::new(ReifyingExtractor {
            inner,
            reify: reify_keyframes,
        }),
        // BUG-100: `handle_arm`'s `body`/`stmt` field is RUNTIME-INTERPRETED text (the
        // signal-handler primitive splits it on `;` and runs each statement via
        // ST.runMutations — see stdlib/primitives/signal-handler.st). It must reach
        // codegen as a JS STRING LITERAL (quoted + escaped), not a raw `Expr` splice.
        // Left as `Expr`, `captured_to_js_with_registry`'s Expr branch emits the arm
        // body's raw source — including any `//`/`/* */` comment content — UNQUOTED
        // into the generated object literal, corrupting/desyncing it (the comment
        // eats the rest of the line, or an apostrophe/colon inside it is misread by
        // a later JS re-scan). Retag body/stmt as String so the String branch's
        // quote+escape path (which IS comment/quote-content-safe, since it treats
        // the whole field as opaque text) applies instead.
        "handle_arm" => Box::new(ReifyingExtractor {
            inner,
            reify: reify_handle_arm,
        }),
        // PLAN-122 W1.2: a CSS scalar's value IS its source text.
        //
        // These productions are written as grammars with named parts
        // (`( $n:number $u:length_unit ) | ( $wide:css_wide )`), so the generic
        // Sequence/Choice extractors return a `Named` RECORD — `{n: "8", u: "px"}`.
        // Codegen would then serialize a JS object where every consumer expects
        // the string `"8px"`: `%color.rgba` would try to parse a record as a
        // colour (E0800), and an animation duration would arrive as an object
        // and compute `NaN`.
        //
        // Flattening here keeps the decision PLAN-122 actually made — the
        // serialized repr of a scalar is its CSS string, always, with the TYPE
        // as compile-time metadata. The grammar's internal parts exist to
        // VALIDATE the value, not to restructure it; a scalar that survived its
        // grammar is exactly the text the author wrote.
        _ if is_scalar_capture(name) => Box::new(ScalarSourceTextExtractor { inner }),
        // A dimension's unit must be ADJACENT to its number. `40px` is a length;
        // `40 px` is a number and an identifier, and CSS has no such value.
        //
        // The check lives on the unit terminal rather than in `SequenceExtractor`
        // because that extractor skips inter-element trivia for every grammar in
        // stdlib — `$a, &b` in `param_list` depends on it. Only the unit sets
        // need the stricter rule, so only they opt in, and the rule stays
        // positional: no list of legal units appears here.
        "length_unit" | "time_unit" | "angle_unit" => {
            Box::new(AdjacentExtractor { inner })
        }
        _ => inner,
    }
}

/// Requires the inner extractor to match with NO leading trivia — the matched
/// token must begin exactly where the previous one ended.
///
/// This is how `40 px` is refused while `40px` is accepted. It cannot be done
/// by the unit grammar itself: a `%capture_type` union is a set of literals and
/// has no way to say "and no space before me", while `SequenceExtractor` skips
/// inter-element trivia on purpose for every other grammar in stdlib.
struct AdjacentExtractor {
    inner: Box<dyn CaptureExtractor>,
}

impl CaptureExtractor for AdjacentExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // A leading trivia token means the caller already separated these; the
        // unit is not part of the value in front of it.
        if tokens.first().is_some_and(|t| t.kind.is_trivia()) {
            return None;
        }
        self.inner.extract(tokens, source)
    }

    fn requires_adjacency(&self) -> bool {
        true
    }
}

/// A CSS scalar's captured value is the SOURCE TEXT its grammar matched.
///
/// The grammar arms carry named sub-captures so a malformed value can be
/// REFUSED precisely (`#e8ee1` fails the counted hex class; `40banana` fails
/// the unit union). Once a value has passed, those parts have done their job
/// and the value is its own best representation — `8px`, `600ms`, `#e8eef7`.
///
/// The text is taken as a SPAN over the tokens the inner extractor consumed,
/// not rebuilt from the captured parts. Rebuilding was tried and is wrong: a
/// grammar arm's parts land in a `HashMap`, whose iteration order is arbitrary,
/// so `6s` reassembled as `"s6"` about half the time — a defect that a
/// single-case test passes by luck. A span is exact by construction and cannot
/// reorder, and it also preserves anything the parts would drop.
struct ScalarSourceTextExtractor {
    inner: Box<dyn CaptureExtractor>,
}

impl ScalarSourceTextExtractor {
    /// Consume a leading token reference (`--tok`, `var(--tok)`,
    /// `var(--tok, fallback)`) and return it as its own source text.
    ///
    /// The token run is a whole ARGUMENT's worth, so the reference must be
    /// delimited by hand rather than by "take everything": a `var(` is closed
    /// at DEPTH ZERO, and a bare `--tok` ends at its own name. Taking the rest
    /// of the run would swallow a following argument and turn a precise type
    /// into a greedy one.
    fn match_token_reference(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        use crate::syntax::cst::SyntaxKind;

        let start_idx = tokens.iter().position(|t| !t.kind.is_trivia())?;
        let first = &tokens[start_idx];

        // `var(` — balance to the closing paren at depth zero. A `)` inside a
        // nested `var(--a, var(--b, red))` must not end the reference.
        let is_var_head = matches!(first.kind, SyntaxKind::IDENT)
            && source
                .get(first.text_range.0..first.text_range.1)
                .is_some_and(|t| t.eq_ignore_ascii_case("var"));
        if is_var_head {
            let mut depth = 0i32;
            for (i, tok) in tokens.iter().enumerate().skip(start_idx) {
                match tok.kind {
                    SyntaxKind::L_PAREN => depth += 1,
                    SyntaxKind::R_PAREN => {
                        depth -= 1;
                        if depth == 0 {
                            let text =
                                source.get(first.text_range.0..tok.text_range.1)?;
                            return super::super::is_token_reference(text)
                                .then(|| (CapturedValue::String(text.to_string()), i + 1));
                        }
                    }
                    // A `var` IDENT not immediately opening a paren is just a
                    // word (a colour keyword, a preset name) — not a reference.
                    _ if depth == 0 && i > start_idx && !tok.kind.is_trivia() => return None,
                    _ => {}
                }
            }
            return None;
        }

        // `--tok`: a single lexed unit, or `-` `-` `name` depending on how the
        // lexer split it. Grow only while the accumulated text still reads as a
        // reference, so the match ends at the name and not at the comma after it.
        let mut end = start_idx;
        let mut best: Option<(String, usize)> = None;
        while end < tokens.len() {
            let tok = &tokens[end];
            if tok.kind.is_trivia() {
                break;
            }
            let text = source.get(first.text_range.0..tok.text_range.1)?;
            if super::super::is_token_reference(text) {
                best = Some((text.to_string(), end + 1));
            } else if best.is_some() {
                break;
            }
            end += 1;
        }
        best.map(|(text, consumed)| (CapturedValue::String(text), consumed))
    }
}

impl CaptureExtractor for ScalarSourceTextExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // A SCALAR also accepts a REFERENCE to a value of itself: `--brand-ink`,
        // `var(--ink)`, `var(--ink, #fff)`. `capture_type_accepts` has said so
        // since PLAN-136 W2 — but that is the PREDICATE, and the form matcher
        // never calls it. The matcher drives THIS extractor, whose inner grammar
        // is the literal production (`( $n:number $u:length_unit )`) and knows
        // nothing about references. So the two halves of one rule disagreed:
        //
        //     @fade-in { easing: --myease; }   builds     (declaration path)
        //     @fade-in(easing: --myease)       E0946      (paren-param path)
        //
        // Same curve, declared in the same file, legal by the language's own
        // rules, accepted in one position and refused in the other — a FALSE
        // REFUSAL, which blocks a build the author cannot fix. It went unseen
        // because the predicate had a gate and the extractor had none.
        //
        // Fixed HERE, at the one wrapper every scalar already passes through
        // (`is_scalar_capture` selects it), rather than in each grammar: a
        // `%scalar_type` row added tomorrow inherits reference support with no
        // Rust edit, which is the same reasoning that put the predicate's
        // admission in one place instead of six.
        //
        // Only the SHAPE is judged. Whether `--x` resolves is the cascade's
        // business — it may come from a project prelude, a `@tokens` block, or
        // plain CSS this compiler never sees — so admitting an unresolved name
        // is correct, and refusing it would assert something we cannot know.
        if let Some((value, consumed)) = self.match_token_reference(tokens, source) {
            return Some((value, consumed));
        }

        let (value, consumed) = self.inner.extract(tokens, source)?;
        if consumed == 0 {
            return Some((value, consumed));
        }

        // Span from the first consumed token to the last, skipping trailing
        // trivia so `8px ` does not capture its separator.
        let matched = &tokens[..consumed];
        let first = matched.iter().find(|t| !t.kind.is_trivia());
        let last = matched.iter().rev().find(|t| !t.kind.is_trivia());
        let (Some(first), Some(last)) = (first, last) else {
            return Some((value, consumed));
        };

        let (start, end) = (first.text_range.0, last.text_range.1);
        match source.get(start..end) {
            Some(text) => Some((CapturedValue::String(text.to_string()), consumed)),
            // Defensive: a range outside `source` means the caller threaded a
            // different source than the tokens came from. Keep the structured
            // value rather than panicking or inventing text.
            None => Some((value, consumed)),
        }
    }
}

/// Reify a handle_arm parse result: retag its `body` / `stmt` field (arm effect
/// statements, currently `CapturedValue::Expr`) as `CapturedValue::String` so codegen
/// quotes+escapes it as opaque runtime text instead of splicing it as raw JS source
/// (BUG-100). A pure retag — the field's TEXT is unchanged, only its captured KIND.
fn reify_handle_arm(value: CapturedValue) -> CapturedValue {
    let CapturedValue::Named(mut map) = value else {
        return value;
    };
    for key in ["body", "stmt"] {
        if let Some(CapturedValue::Expr(text)) = map.get(key) {
            map.insert(key.to_string(), CapturedValue::String(text.clone()));
        }
    }
    CapturedValue::Named(map)
}

/// Reify a keyframes parse result into `Keyframes(Vec<KeyframeDef>)` (PLAN-023 W2). Each
/// record carries `property` and `values` (a balanced run); the run is split on `->` into
/// the value chain (pure scalar split). Empty value chains are dropped (matching the Rust
/// extractor).
fn reify_keyframes(value: CapturedValue) -> CapturedValue {
    use crate::syntax::form_match::KeyframeDef;

    let items = match value {
        CapturedValue::Array(items) => items,
        single @ CapturedValue::Named(_) => vec![single],
        _ => return CapturedValue::Keyframes(Vec::new()),
    };

    let mut keyframes = Vec::new();
    for item in items {
        let CapturedValue::Named(map) = item else {
            continue;
        };
        // BUG-201: a nested scope item (`sel { prop: a -> b; }`) arrives with
        // `selector` + an already-recursively-reified `scope` payload — tag its
        // inner keyframes with the selector and flatten them in, instead of
        // silently dropping the whole block (which is what made
        // `.parent { @mouse ... { .child { ... } } }` animate nothing).
        if let (Some(selector), Some(CapturedValue::Keyframes(inner))) =
            (field_text(map.get("selector")), map.get("scope"))
        {
            for mut kf in inner.clone() {
                kf.selector = Some(selector.clone());
                keyframes.push(kf);
            }
            continue;
        }
        let Some(property) = field_text(map.get("property")) else {
            continue;
        };
        let raw = field_text(map.get("values")).unwrap_or_default();
        let values = split_on_top_level_arrow(&raw);
        if values.is_empty() {
            continue;
        }
        keyframes.push(KeyframeDef {
            property,
            values,
            selector: None,
        });
    }
    CapturedValue::Keyframes(keyframes)
}

/// Reify a params parse result into `Params(Vec<ParamDef>)` (PLAN-023 W2). Each record
/// carries `name` and `value` (`type [= default]`); the value is split on the FIRST `=` at
/// the string level into type_ref + optional default (`=` is meta-layer default-assignment).
/// Pure scalar split — no token parsing.
fn reify_params(value: CapturedValue) -> CapturedValue {
    use crate::syntax::form_match::ParamDef;

    let items = match value {
        CapturedValue::Array(items) => items,
        single @ CapturedValue::Named(_) => vec![single],
        _ => return CapturedValue::Params(Vec::new()),
    };

    let mut params = Vec::new();
    for item in items {
        let CapturedValue::Named(map) = item else {
            continue;
        };
        let Some(name) = field_text(map.get("name")) else {
            continue;
        };
        let raw = field_text(map.get("value")).unwrap_or_default();
        let (type_ref, default) = match split_on_bare_eq(&raw) {
            Some((t, d)) => (t.trim().to_string(), Some(d.trim().to_string())),
            None => (raw.clone(), None),
        };
        params.push(ParamDef {
            name,
            type_ref,
            default,
        });
    }
    CapturedValue::Params(params)
}

/// Reify a properties parse result into `Properties(Vec<PropertyDef>)` (PLAN-023 W2).
///
/// The stdlib grammar yields an Array of per-item records; field items carry `name`,
/// `value` (the balanced run) and optionally `optional` (the `?` marker); skipped
/// `@`-directive blocks yield empty markers, which are filtered out. Pure rename/restructure.
fn reify_properties(value: CapturedValue) -> CapturedValue {
    use crate::syntax::form_match::PropertyDef;

    let items = match value {
        CapturedValue::Array(items) => items,
        single @ CapturedValue::Named(_) => vec![single],
        _ => return CapturedValue::Properties(Vec::new()),
    };

    let mut props = Vec::new();
    for item in items {
        let CapturedValue::Named(map) = item else {
            continue; // skip-block marker or non-record
        };
        // A form SPLICE inside a form body (`--card-surface;`) rides in the
        // same list as the declarations around it (BUG-327). It is carried as
        // a PropertyDef whose NAME is the form reference — `expand_style_form_splices`
        // resolves it recursively and drops the placeholder. Encoding it this
        // way keeps one body shape: every consumer that walks `properties`
        // sees a list of PropertyDefs, and a consumer that does not understand
        // splices (a `@type` body, say) sees a name it can reject rather than
        // a silently truncated list.
        if let Some(splice) = field_text(map.get("splice")) {
            let args = field_text(map.get("splice_args")).unwrap_or_default();
            props.push(PropertyDef {
                name: splice,
                type_ref: args,
                optional: false,
            });
            continue;
        }
        let Some(name) = field_text(map.get("name")) else {
            continue;
        };
        let type_ref = field_text(map.get("value")).unwrap_or_default();
        let optional = matches!(
            map.get("optional"),
            Some(CapturedValue::String(s)) if s == "?"
        );
        props.push(PropertyDef {
            name,
            type_ref,
            optional,
        });
    }
    CapturedValue::Properties(props)
}

/// Split a param value run at the FIRST *bare* `=` (meta-layer default-assignment), ignoring
/// `=` that is part of a comparison/relational operator (`==`, `!=`, `>=`, `<=`, `=>`) and any
/// `=` nested inside `()[]{}`. Returns (type_ref, default) halves, or None if there is no bare
/// top-level `=`. Mirrors the token-level `EQUALS`-only split of the deleted Rust
/// ParamsExtractor without re-tokenizing (PLAN-023 W2 review hardening).
fn split_on_bare_eq(s: &str) -> Option<(&str, &str)> {
    let b = s.as_bytes();
    let mut depth: i32 = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'=' if depth == 0 => {
                let prev = if i > 0 { Some(b[i - 1]) } else { None };
                let next = b.get(i + 1).copied();
                let part_of_op = matches!(prev, Some(b'!' | b'<' | b'>' | b'='))
                    || matches!(next, Some(b'=' | b'>'));
                if !part_of_op {
                    return Some((&s[..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Split a keyframe value chain on top-level `->` arrows, respecting `()[]{}` nesting and
/// double-quoted strings (so `->` inside a function call or string literal does not split).
/// Mirrors the token-level `ARROW`-at-top-level split of the deleted Rust KeyframesExtractor
/// (PLAN-023 W2 review hardening).
pub(crate) fn split_on_top_level_arrow(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    let mut parts = Vec::new();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut seg_start = 0;
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
            b'-' if depth == 0 && b.get(i + 1) == Some(&b'>') => {
                parts.push(s[seg_start..i].trim().to_string());
                i += 2;
                seg_start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(s[seg_start..].trim().to_string());
    parts.into_iter().filter(|x| !x.is_empty()).collect()
}

/// Extract a plain string from a captured field value (Ident or Expr), trimmed.
fn field_text(v: Option<&CapturedValue>) -> Option<String> {
    match v {
        Some(CapturedValue::Ident(s)) | Some(CapturedValue::Expr(s)) => Some(s.trim().to_string()),
        // A `selector`-typed capture (e.g. keyframes' nested-scope `$selector:selector`)
        // yields Selector; its text is the selector source (`.child`, `&:not(.x)`). BUG-201.
        Some(CapturedValue::Selector(s)) if !s.is_empty() => Some(s.trim().to_string()),
        // A `typeref`-typed capture (e.g. param_list's `$ptype:typeref`) yields TypeRef;
        // its text is the type expression (`url`, `Type[]`, `Type?`). FUP-040.
        Some(CapturedValue::TypeRef(s)) if !s.is_empty() => Some(s.trim().to_string()),
        Some(CapturedValue::String(s)) if !s.is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

/// An extractor that runs `inner` then maps its generic result into a domain-typed value.
struct ReifyingExtractor {
    inner: Box<dyn CaptureExtractor>,
    reify: fn(CapturedValue) -> CapturedValue,
}

impl CaptureExtractor for ReifyingExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let (value, consumed) = self.inner.extract(tokens, source)?;
        Some(((self.reify)(value), consumed))
    }
}

/// Reify a param_list parse result into `ParamList(Vec<TemplateParamDef>)`.
///
/// The stdlib grammar `( ( $binding:binding | $element:element ) $optional:("?")? ","? )*`
/// produces an Array of per-item Named records; each record carries either a `binding` or
/// `element` key (the matched Choice branch = the kind) and an `optional` key when `?` was
/// present. This is a pure rename/restructure (the reifier-honesty rule): NO parsing.
fn reify_param_list(value: CapturedValue) -> CapturedValue {
    use crate::syntax::form_match::{TemplateParamDef, TemplateParamKind};

    let items = match value {
        CapturedValue::Array(items) => items,
        // A single item (Repeat collapsed) — wrap it.
        single @ CapturedValue::Named(_) => vec![single],
        // Nothing matched -> empty param list (templates allow `()`).
        _ => return CapturedValue::ParamList(Vec::new()),
    };

    let mut params = Vec::new();
    for item in items {
        let CapturedValue::Named(map) = item else {
            continue;
        };
        let (name, kind) = if let Some(CapturedValue::Binding(n)) = map.get("binding") {
            (
                n.trim_start_matches('$').to_string(),
                TemplateParamKind::Binding,
            )
        } else if let Some(CapturedValue::Element(n)) = map.get("element") {
            (
                n.trim_start_matches('&').to_string(),
                TemplateParamKind::Element,
            )
        } else {
            continue;
        };
        // The named optional group `$optional:("?")?` always inserts a key (Optional yields
        // an empty String when absent), so test the VALUE, not mere presence.
        let optional = matches!(
            map.get("optional"),
            Some(CapturedValue::String(s)) if s == "?"
        );
        // Optional SPACE-form type annotation (`$href url`) and `= default` value
        // (`$alt string = ""`). Pure rename/restructure — no parsing (reifier-honesty).
        let type_ref = field_text(map.get("ptype"));
        let default = field_text(map.get("default"));
        // Collection marker `[]` (`&items[]`): the `collmark` group inserts a key
        // when present (Array of the two bracket tokens). Test presence of a
        // non-empty captured value, mirroring the optmark discipline.
        let collection = match map.get("collection") {
            Some(CapturedValue::Array(a)) => !a.is_empty(),
            Some(CapturedValue::String(s)) => !s.is_empty(),
            Some(_) => true,
            None => false,
        };
        params.push(TemplateParamDef {
            name,
            kind,
            optional,
            type_ref,
            default,
            collection,
        });
    }
    CapturedValue::ParamList(params)
}

/// Resolve a BUILTIN `CaptureType` to its extractor.
///
/// Exposed so `capture_type_accepts` can ask a builtin type the same question it
/// asks a stdlib grammar. Without it that function saw only stdlib productions
/// and returned `true` for every builtin name — which is how `number` came to
/// accept `#e8eef7` and `bool` to accept `abc` (PLAN-136 W1).
pub fn builtin_extractor_for(
    ct: &CaptureType,
    registry: &ExtractorRegistry,
) -> Box<dyn CaptureExtractor> {
    capture_type_to_extractor(ct, registry)
}

/// Get an extractor for a built-in CaptureType, or fall back to Expr.
fn capture_type_to_extractor(
    ct: &CaptureType,
    registry: &ExtractorRegistry,
) -> Box<dyn CaptureExtractor> {
    // Parameterized terminal: balanced(delim) is depth-aware and not registry-keyed.
    if let CaptureType::Balanced(delim) = ct {
        return Box::new(BalancedExtractor { delim: *delim });
    }
    if let CaptureType::SkipBlock = ct {
        return Box::new(SkipBlockExtractor);
    }
    // Custom (stdlib %capture_type) reference inside another production is resolved by the
    // defs-aware compiler (compile_pattern_with_defs). At this registry-only entry point we
    // have no access to sibling definitions, so an unresolved Custom falls back to Expr.
    if let CaptureType::Custom(_) = ct {
        return Box::new(super::simple::ExprExtractor);
    }
    // For built-in types, clone from the registry
    if let Some(_ext) = registry.get(ct) {
        // Since we can't clone trait objects, create new instances
        match ct {
            CaptureType::Ident => Box::new(super::simple::IdentExtractor),
            CaptureType::DashedIdent => Box::new(super::simple::DashedIdentExtractor),
            CaptureType::EventName => Box::new(super::simple::EventNameExtractor),
            CaptureType::String => Box::new(super::simple::StringExtractor),
            CaptureType::Number => Box::new(super::simple::NumberExtractor),
            CaptureType::Bool => Box::new(super::simple::BoolExtractor),
            CaptureType::Time | CaptureType::Duration => Box::new(super::simple::TimeExtractor),
            CaptureType::Length => Box::new(super::simple::LengthExtractor),
            CaptureType::Event => Box::new(super::simple::EventExtractor),
            CaptureType::Easing => Box::new(super::simple::EasingExtractor),
            CaptureType::Expr => Box::new(super::simple::ExprExtractor),
            CaptureType::Typeref => Box::new(super::simple::TyperefExtractor),
            CaptureType::Binding => Box::new(super::reference::BindingExtractor),
            CaptureType::Element => Box::new(super::reference::ElementExtractor),
            CaptureType::Preset => Box::new(super::reference::PresetExtractor),
            CaptureType::Selector => Box::new(super::reference::SelectorExtractor),
            CaptureType::Color => Box::new(super::simple::ColorExtractor),
            CaptureType::JsBlock => Box::new(super::blocks::JsBlockExtractor),
            // `template_invocation` (`&name(args)`) must use its structured extractor (Named
            // {name, args}) here too — not fall to Expr, which would swallow the whole `&...`
            // run as a raw string and break a `( $x:template_invocation )*` repetition. The macro
            // %form path already wired this; the %capture_type engine must agree (BUG-066).
            CaptureType::TemplateInvocation => {
                Box::new(super::pattern::TemplateInvocationExtractor)
            }
            // properties/fields/params/keyframes/param_list migrated to stdlib (PLAN-023 W2):
            // resolved via Custom(name) + reifier.
            _ => Box::new(super::simple::ExprExtractor),
        }
    } else {
        // Unknown type — fall back to Expr
        Box::new(super::simple::ExprExtractor)
    }
}

/// Collect tokens until `delim` appears at nesting depth 0 (respecting `()[]{}`), the one
/// depth-aware PEG terminal (PLAN-023 W2). Produces the raw spanned text as
/// `CapturedValue::Expr`. Leading whitespace is skipped; the delimiter itself is NOT
/// consumed (a following Literal in the sequence matches it, mirroring `raw_until_semicolon`
/// + `";"?`). Returns None if no non-whitespace content precedes the delimiter/EOF.
pub struct BalancedExtractor {
    pub delim: char,
}

impl CaptureExtractor for BalancedExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let mut pos = 0;
        while pos < tokens.len() && tokens[pos].kind.is_trivia() {
            pos += 1;
        }
        let start = pos;
        if start >= tokens.len() {
            return None;
        }

        // SOURCE byte-scan (not token-slice walk). The directive grammar splits
        // call-args `(...)`, index `[...]`, and nested `{...}` into separate
        // structural child nodes (ARG_LIST/BODY), so they are MISSING from the
        // flat inline token slice. A token-only walk of `sum($items)` would see
        // just `sum`. Scanning the contiguous source from the first token's byte
        // offset recovers the full balanced run — mirroring W0's HTML byte-scanner.
        // String literals are skipped so a delimiter inside quotes does not end
        // the run. The scan stops at the delimiter (or an unmatched closer) at
        // depth 0; token `consumed` is then every inline token that began before
        // the scan end, advancing the cursor past the structurally-hidden args.
        let byte_start = tokens[start].text_range.0;
        // Bound the scan to the token slice extent, NOT source.len(). The hidden
        // ARG_LIST/BODY child tokens lie BETWEEN this slice's tokens in source, so
        // the last token's end still covers them — but a missing depth-0 delimiter
        // or an unterminated string must NOT run past the directive into sibling
        // source. In the well-formed case the delimiter (or a depth-0 closer) stops
        // the scan before this bound; the bound only caps malformed input so the
        // capture degrades (under-captures) instead of swallowing later directives.
        let scan_end = tokens
            .last()
            .map(|t| t.text_range.1)
            .unwrap_or(byte_start)
            .min(source.len());
        let bytes = source.as_bytes();
        let mut i = byte_start;
        let mut depth: i32 = 0;
        while i < scan_end {
            let c = bytes[i] as char;
            // Comments (BUG-100): a `//` line comment or `/* */` block comment's
            // CONTENT must never be scanned for string-opening quotes or
            // structural separators (`:`, brackets) — a comment like `// serde's
            // default` or `// std::time::Duration` would otherwise desync the
            // scanner (the `'` looks like a string open that never closes within
            // the scan bound, or `::` gets spliced in as bogus structure). Detect
            // and skip the WHOLE comment before it ever reaches the quote/bracket/
            // delim match below, so nothing inside it is structurally significant.
            if c == '/' && i + 1 < scan_end && bytes[i + 1] as char == '/' {
                i += 2;
                while i < scan_end && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if c == '/' && i + 1 < scan_end && bytes[i + 1] as char == '*' {
                i += 2;
                while i + 1 < scan_end && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                // Land past the closing `*/` (or at scan_end if unterminated).
                i = (i + 2).min(scan_end);
                continue;
            }
            match c {
                '"' | '\'' | '`' => {
                    let quote = c;
                    i += 1;
                    while i < scan_end {
                        let ch = bytes[i] as char;
                        i += 1;
                        if ch == '\\' && i < scan_end {
                            i += 1;
                            continue;
                        }
                        if ch == quote {
                            break;
                        }
                    }
                    continue;
                }
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    if depth == 0 {
                        // Closer belonging to an enclosing group (e.g. the macro
                        // body's `}`) ends the run without being consumed.
                        break;
                    }
                    depth -= 1;
                }
                _ => {
                    if depth == 0 && c == self.delim {
                        break;
                    }
                }
            }
            i += 1;
        }
        let byte_end = i;
        let text = source[byte_start..byte_end].trim().to_string();
        if text.is_empty() {
            return None;
        }
        // Consume every token in the slice that started before the scan end.
        let mut consumed = start;
        while consumed < tokens.len() && tokens[consumed].text_range.0 < byte_end {
            consumed += 1;
        }
        if consumed == start {
            consumed = start + 1; // always make progress
        }
        Some((CapturedValue::Expr(text), consumed))
    }
}

/// Skip a balanced `{ ... }` block (depth-aware), consuming everything up to and including
/// the matching close brace. Produces an empty String marker. Used to step a repeated
/// production over nested directive blocks (e.g. `@dark { ... }`) so later siblings are
/// still collected (PLAN-023 W2). Returns None if the cursor is not at a `{` (after leading
/// whitespace) so it can be a non-matching Choice alternative.
pub struct SkipBlockExtractor;

impl CaptureExtractor for SkipBlockExtractor {
    fn extract(&self, tokens: &[TokenData], _source: &str) -> ExtractResult {
        let mut pos = 0;
        while pos < tokens.len() && tokens[pos].kind.is_trivia() {
            pos += 1;
        }
        // Consume any leading tokens up to the first `{` that are part of the block opener
        // (e.g. `@dark` or `@media("...")`), but ONLY if a `{` follows before a `;` or `}`
        // at depth 0 — otherwise this isn't a block and we must not match.
        let mut scan = pos;
        let mut found_brace = false;
        while scan < tokens.len() {
            match tokens[scan].kind {
                SyntaxKind::L_BRACE => {
                    found_brace = true;
                    break;
                }
                SyntaxKind::SEMICOLON | SyntaxKind::R_BRACE => break,
                _ => scan += 1,
            }
        }
        if !found_brace {
            return None;
        }
        // scan is at the opening brace; consume the balanced block.
        let mut depth = 0i32;
        let mut i = scan;
        while i < tokens.len() {
            match tokens[i].kind {
                SyntaxKind::L_BRACE => depth += 1,
                SyntaxKind::R_BRACE => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        return Some((CapturedValue::String(String::new()), i));
                    }
                }
                _ => {}
            }
            i += 1;
        }
        // Unterminated block: consume to EOF.
        Some((CapturedValue::String(String::new()), tokens.len()))
    }
}

/// Matches a literal token text.
struct LiteralExtractor(String);

impl CaptureExtractor for LiteralExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // Skip leading trivia (whitespace AND comments — BUG-135).
        let mut pos = 0;
        while pos < tokens.len() && tokens[pos].kind.is_trivia() {
            pos += 1;
        }

        if pos >= tokens.len() {
            return None;
        }

        // Fast path: the literal matches a single token's text.
        //
        // ASCII case-insensitively, because the constructs these literals spell
        // are: CSS keywords and function names (`RGB(...)`, `TRANSPARENT`,
        // `INHERIT`) are case-insensitive per spec, and so are HTML/CSS at-rule
        // and unit spellings. Matching exactly would refuse valid stylesheets
        // for their capitalisation — and the value is returned as the AUTHOR
        // wrote it, not normalised, so nothing downstream sees a rewritten
        // spelling.
        let text = tokens[pos].text(source);
        if text.eq_ignore_ascii_case(&self.0) {
            return Some((CapturedValue::String(text.to_string()), pos + 1));
        }

        // Multi-token literals: a grammar literal like `=>` lexes as several tokens
        // (EQUALS + GT), and `@match` as AT_SIGN + IDENT. The lexer's token granularity is
        // an implementation detail the grammar author should not have to know, so match the
        // literal against the CONCATENATED text of consecutive tokens. The tokens must be
        // CONTIGUOUS in source (no gap/whitespace between them) so we never match across an
        // unintended boundary (e.g. `=` `>` with a space stays unmatched).
        if !self.0.is_empty() && self.0.len() > text.len() {
            let mut acc = String::new();
            let mut end = pos;
            while end < tokens.len() {
                // Reject a whitespace token in the middle of a multi-token literal.
                if tokens[end].kind.is_trivia() {
                    break;
                }
                // Reject a source gap between this token and the previous one
                // (defends against non-adjacent tokens forming a spurious match).
                if end > pos && tokens[end].text_range.0 != tokens[end - 1].text_range.1 {
                    break;
                }
                acc.push_str(tokens[end].text(source));
                end += 1;
                if acc == self.0 {
                    return Some((CapturedValue::String(self.0.clone()), end));
                }
                if acc.len() >= self.0.len() {
                    break; // overshot without matching
                }
            }
        }

        None
    }
}

/// Matches a character class: [abc] or [^abc].
struct CharClassExtractor {
    chars: String,
    negated: bool,
    /// Permitted repetition counts (`xdigit{3|4|6|8}`), longest first. `None` =
    /// the bare `[…]` form, which matches exactly one character.
    counts: Option<crate::parser::meta_ast::CountSet>,
}

impl CharClassExtractor {
    /// Whether `c` is in this class, honouring negation. ASCII case-insensitive:
    /// CSS is case-insensitive throughout (`#FFF` ≡ `#fff`, `10PX` ≡ `10px`), so a
    /// class written `[0-9a-f]` accepts `A`–`F` without the author restating the
    /// range. Declaring the fold on the terminal rather than duplicating every
    /// range is what keeps a stdlib grammar readable.
    fn accepts(&self, c: char) -> bool {
        let in_class = Self::member(&self.chars, c)
            || (c.is_ascii_alphabetic()
                && (Self::member(&self.chars, c.to_ascii_lowercase())
                    || Self::member(&self.chars, c.to_ascii_uppercase())));
        if self.negated { !in_class } else { in_class }
    }

    /// Membership in a class spec, honouring `a-f` RANGES.
    ///
    /// Ranges are what make a hex class writable at all (`[0-9a-fA-F]` vs sixteen
    /// literals), and they are the one piece of regex syntax a token grammar
    /// genuinely needs. A trailing or leading `-` is a literal dash, matching the
    /// usual convention.
    fn member(spec: &str, c: char) -> bool {
        let chars: Vec<char> = spec.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i + 2 < chars.len() && chars[i + 1] == '-' {
                // `a-f` — a range needs all three positions to exist.
                if c >= chars[i] && c <= chars[i + 2] {
                    return true;
                }
                i += 3;
            } else {
                if chars[i] == c {
                    return true;
                }
                i += 1;
            }
        }
        false
    }
}

impl CaptureExtractor for CharClassExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // Skip leading trivia (whitespace AND comments — BUG-135).
        let mut pos = 0;
        while pos < tokens.len() && tokens[pos].kind.is_trivia() {
            pos += 1;
        }

        if pos >= tokens.len() {
            return None;
        }

        let text = tokens[pos].text(source);

        let Some(counts) = self.counts else {
            // Bare `[…]`: one character, one token — the original behaviour.
            let c = text.chars().next()?;
            return if self.accepts(c) {
                Some((CapturedValue::String(c.to_string()), pos + 1))
            } else {
                None
            };
        };

        // Counted form. The run is matched WITHIN and ACROSS tokens by bytes,
        // because the lexer's token boundaries do not align with a value's shape:
        // `#e8eef7` arrives as HASH + IDENT but `#123456` as HASH + NUMBER, and
        // `#e8ee` splits differently again. Length is the discriminator here, so
        // the scan has to see characters, not token kinds — this is the crossing
        // point where a grammar stops inheriting the lexer's guesses about a
        // domain it does not know.
        // The scan window is the CONTIGUOUS run of tokens starting here — each
        // beginning exactly where the previous ended, and none of them trivia.
        //
        // Taking `tokens.last()` instead was wrong in a way that only showed up
        // in one position: inside a CSS declaration the slice happens to end at
        // the `;`, but inside a directive arg list it runs to the closing `)`,
        // so `color: #c65b3c)` scanned `c65b3c)` — over-long against every
        // permitted count, and the colour was refused in exactly the place
        // authors write it most (`@object(color: #c65b3c)`).
        //
        // A value cannot span a gap or a separator, so the window must not
        // either; where the slice happens to end is not the grammar's business.
        let byte_start = tokens[pos].text_range.0;
        let mut scan_end = tokens[pos].text_range.1;
        for tok in &tokens[pos + 1..] {
            if tok.kind.is_trivia() || tok.text_range.0 != scan_end {
                break;
            }
            scan_end = tok.text_range.1;
        }
        let scan_end = scan_end.min(source.len());
        let run = &source[byte_start..scan_end];

        // Scan one past the largest permitted count: an over-long run (a 9-digit
        // hex) must FAIL rather than silently truncate to a legal 8.
        let max_count = counts.counts().first().copied().unwrap_or(0) as usize;
        let mut available = 0usize;
        for c in run.chars() {
            if available > max_count || !self.accepts(c) {
                break;
            }
            available += 1;
        }

        // An exact-length match against the permitted set. `counts` is sorted
        // descending, but the comparison is equality on the FULL run — matching a
        // shorter prefix would accept `#e8ee` as a 3-digit colour and leave `e` to
        // break the surrounding sequence somewhere less obvious.
        let matched = counts
            .counts()
            .iter()
            .copied()
            .find(|n| *n as usize == available)?;

        let matched_bytes: usize = run.chars().take(matched as usize).map(char::len_utf8).sum();
        let text_matched = &run[..matched_bytes];
        let end_byte = byte_start + matched_bytes;

        // The run must END AT A TOKEN BOUNDARY, not part-way through one.
        //
        // Without this, `#e8eef7z` is accepted: the scan stops at the non-hex
        // `z` with six digits banked, six is a legal count, and the malformed
        // colour sails through with its suffix attached. The stopping character
        // alone cannot distinguish the two cases — in `#c65b3c)` the scan also
        // stops at a non-hex character, but there the digits fill the IDENT
        // exactly and the `)` is a separate token that legitimately ends the
        // value. "Did the run consume whole tokens?" is the question that
        // separates a terminator from a corruption.
        let ends_at_token_boundary = tokens[pos..]
            .iter()
            .any(|t| t.text_range.1 == end_byte)
            || end_byte == scan_end;
        if !ends_at_token_boundary {
            return None;
        }

        // Consume every token that began before the scan end; a counted run may end
        // mid-token (`#fff` where IDENT is `fff`), in which case the token is still
        // consumed — the run is the capture's value, and the grammar continues from
        // the next token.
        let mut consumed = pos;
        while consumed < tokens.len() && tokens[consumed].text_range.0 < end_byte {
            consumed += 1;
        }
        if consumed == pos {
            consumed = pos + 1;
        }

        Some((CapturedValue::String(text_matched.to_string()), consumed))
    }
}

/// Makes an inner extractor optional — returns empty string on failure.
struct OptionalExtractor {
    inner: Box<dyn CaptureExtractor>,
}

impl CaptureExtractor for OptionalExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        match self.inner.extract(tokens, source) {
            Some(result) => Some(result),
            None => Some((CapturedValue::String(String::new()), 0)),
        }
    }
}

/// Repeats an inner extractor zero-or-more or one-or-more times.
struct RepeatExtractor {
    inner: Box<dyn CaptureExtractor>,
    at_least_one: bool,
}

impl CaptureExtractor for RepeatExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let mut values = Vec::new();
        let mut total_consumed = 0;

        loop {
            let remaining = &tokens[total_consumed..];
            if remaining.is_empty() {
                break;
            }

            match self.inner.extract(remaining, source) {
                Some((value, consumed)) if consumed > 0 => {
                    values.push(value);
                    total_consumed += consumed;
                }
                _ => break,
            }
        }

        if self.at_least_one && values.is_empty() {
            return None;
        }

        Some((CapturedValue::Array(values), total_consumed))
    }
}

/// Matches a sequence of extractors in order. Each element may carry a capture NAME.
///
/// If any element is named, the sequence produces a structured `CapturedValue::Named` map
/// (name -> value) — the record that reifiers consume (PLAN-023 W2). If NO element is named
/// (pure literal/charclass glue), it falls back to the legacy `last_value` semantics so
/// existing unnamed sequences are unaffected.
struct SequenceExtractor(Vec<(Option<String>, Box<dyn CaptureExtractor>)>);

impl CaptureExtractor for SequenceExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let mut total_consumed = 0;
        let mut last_value = CapturedValue::String(String::new());
        let mut record: std::collections::HashMap<String, CapturedValue> =
            std::collections::HashMap::new();
        let mut any_named = false;

        for (name, extractor) in &self.0 {
            // Skip inter-element whitespace so reference extractors (binding/element), which
            // match at position 0, can see the next token after a separator + spaces
            // (e.g. `$a, &b`). Whitespace-tolerant extractors are unaffected.
            //
            // An element that CARES about adjacency (a dimension's unit — `40px`
            // is a value, `40 px` is not) is offered the un-skipped slice first,
            // so it can see the whitespace it needs to refuse. Only extractors
            // that opt in via `requires_adjacency` take this path; for every
            // other grammar the behaviour is unchanged.
            let skipped = {
                let mut n = total_consumed;
                while n < tokens.len() && tokens[n].kind.is_trivia() {
                    n += 1;
                }
                n
            };
            if extractor.requires_adjacency() && skipped != total_consumed {
                return None;
            }
            total_consumed = skipped;
            let remaining = &tokens[total_consumed..];
            match extractor.extract(remaining, source) {
                Some((value, consumed)) => {
                    match name {
                        Some(n) => {
                            record.insert(n.clone(), value.clone());
                            any_named = true;
                        }
                        None => {
                            // An UNNAMED element that itself produced a structured record
                            // (e.g. a Group wrapping a named Choice) merges its keys up into
                            // this sequence's record, so nested discriminators survive.
                            if let CapturedValue::Named(sub) = &value {
                                for (k, v) in sub {
                                    record.insert(k.clone(), v.clone());
                                }
                                any_named = true;
                            }
                        }
                    }
                    last_value = value;
                    total_consumed += consumed;
                }
                None => return None,
            }
        }

        if any_named {
            Some((CapturedValue::Named(record), total_consumed))
        } else {
            Some((last_value, total_consumed))
        }
    }
}

/// Tries alternatives in order, returns the first match. When the matching alternative is a
/// named capture, the result is wrapped as a single-key `Named { name -> value }` so callers
/// can tell WHICH branch matched (the discriminator a reifier needs, e.g. param_list's
/// binding vs element kind). Unnamed alternatives return the inner value as-is (back-compat).
struct ChoiceExtractor(Vec<(Option<String>, Box<dyn CaptureExtractor>)>);

impl CaptureExtractor for ChoiceExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        for (name, extractor) in &self.0 {
            if let Some((value, consumed)) = extractor.extract(tokens, source) {
                return match name {
                    Some(n) => {
                        let mut map = std::collections::HashMap::new();
                        map.insert(n.clone(), value);
                        Some((CapturedValue::Named(map), consumed))
                    }
                    None => Some((value, consumed)),
                };
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(kind: SyntaxKind, start: usize, end: usize) -> TokenData {
        TokenData {
            kind,
            text_range: (start, end),
        }
    }

    // Helper: lex a source string into TokenData (mirrors production token slices).
    fn lex_lit(source: &str) -> Vec<TokenData> {
        crate::syntax::cst::lexer::Lexer::new(source)
            .tokenize()
            .into_iter()
            .filter(|t| t.kind != SyntaxKind::EOF)
            .map(|t| TokenData {
                kind: t.kind,
                text_range: (t.offset, t.offset + t.len()),
            })
            .collect()
    }

    // --- Counted char classes (FEAT-164 / PLAN-122 W0) -----------------------
    //
    // A hex colour is the motivating case: its LENGTH is the discriminator
    // (3/4/6/8 digits, nothing else), and the lexer's token boundaries do not
    // align with it — `#e8eef7` arrives as HASH+IDENT but `#123456` as
    // HASH+NUMBER. So the terminal has to count characters, not tokens.

    fn hex_class(counts: &[u8]) -> CharClassExtractor {
        CharClassExtractor {
            chars: "0-9a-fA-F".to_string(),
            negated: false,
            counts: Some(crate::parser::meta_ast::CountSet::new(counts.to_vec())),
        }
    }

    /// Run a counted class over the source AFTER a leading `#`, the way a colour
    /// grammar (`"#" $hex:xdigit{3|4|6|8}`) would.
    fn hex_after_hash(src: &str, counts: &[u8]) -> Option<String> {
        let toks = lex_lit(src);
        let after_hash: Vec<TokenData> = toks.into_iter().skip(1).collect();
        match hex_class(counts).extract(&after_hash, src) {
            Some((CapturedValue::String(s), _)) => Some(s),
            _ => None,
        }
    }

    #[test]
    fn counted_class_matches_every_legal_hex_length() {
        for src in ["#fff", "#ffff", "#e8eef7", "#11223344"] {
            let expected = &src[1..];
            assert_eq!(
                hex_after_hash(src, &[3, 4, 6, 8]).as_deref(),
                Some(expected),
                "{src} is a legal hex colour"
            );
        }
    }

    #[test]
    fn counted_class_rejects_illegal_hex_lengths() {
        // 5 and 7 are not colour lengths. Crucially the extractor must REJECT
        // rather than match a shorter legal prefix (3 of the 5, 6 of the 7) and
        // leave a stray digit to break the surrounding sequence somewhere less
        // obvious — that silent truncation is the bug this shape exists to avoid.
        for src in ["#e8ee1", "#e8eef71"] {
            assert_eq!(
                hex_after_hash(src, &[3, 4, 6, 8]),
                None,
                "{src} has no legal hex length"
            );
        }
    }

    #[test]
    fn counted_class_is_case_insensitive() {
        // CSS is case-insensitive: `#E8EEF7` ≡ `#e8eef7`. The fold lives on the
        // terminal so a grammar writes `[0-9a-f]` once instead of restating ranges.
        assert_eq!(
            hex_after_hash("#E8EEF7", &[3, 4, 6, 8]).as_deref(),
            Some("E8EEF7")
        );
        assert_eq!(
            hex_after_hash("#AaBbCc", &[3, 4, 6, 8]).as_deref(),
            Some("AaBbCc")
        );
    }

    #[test]
    fn counted_class_spans_token_boundaries() {
        // The whole point: `#e8eef7` lexes as HASH + IDENT, `#123456` as HASH +
        // NUMBER, and `#1a2b3c` may split into several tokens. A token-shaped
        // matcher would see different structures for the same kind of value; a
        // byte run sees one.
        for src in ["#e8eef7", "#123456", "#1a2b3c", "#a1b2c3"] {
            assert_eq!(
                hex_after_hash(src, &[3, 4, 6, 8]).as_deref(),
                Some(&src[1..]),
                "{src} must match regardless of how it tokenized"
            );
        }
    }

    #[test]
    fn counted_class_single_count_form() {
        // `{4}` — the single-count spelling, e.g. a four-digit year.
        let src = "2026";
        let toks = lex_lit(src);
        let ext = CharClassExtractor {
            chars: "0-9".to_string(),
            negated: false,
            counts: Some(crate::parser::meta_ast::CountSet::new(vec![4])),
        };
        assert!(matches!(
            ext.extract(&toks, src),
            Some((CapturedValue::String(ref s), _)) if s == "2026"
        ));
    }

    #[test]
    fn bare_class_still_matches_one_char() {
        // No counts = the pre-existing behaviour, unchanged.
        let src = "abc";
        let toks = lex_lit(src);
        let ext = CharClassExtractor {
            chars: "a-z".to_string(),
            negated: false,
            counts: None,
        };
        assert!(matches!(
            ext.extract(&toks, src),
            Some((CapturedValue::String(ref s), _)) if s == "a"
        ));
    }

    #[test]
    fn char_class_ranges_and_negation() {
        // Ranges are what make `[0-9a-fA-F]` writable at all; negation must invert
        // the range test, not just the literal test.
        let pos = CharClassExtractor {
            chars: "0-9".to_string(),
            negated: false,
            counts: None,
        };
        let neg = CharClassExtractor {
            chars: "0-9".to_string(),
            negated: true,
            counts: None,
        };
        assert!(pos.accepts('5'));
        assert!(!pos.accepts('x'));
        assert!(neg.accepts('x'));
        assert!(!neg.accepts('5'));
        // A trailing dash is a literal dash, per the usual convention.
        let dash = CharClassExtractor {
            chars: "a-".to_string(),
            negated: false,
            counts: None,
        };
        assert!(dash.accepts('-'));
        assert!(dash.accepts('a'));
    }

    #[test]
    fn count_set_is_sorted_descending_and_deduped() {
        // The invariant the extractor relies on: longest-first, no zeroes, no dups.
        let set = crate::parser::meta_ast::CountSet::new(vec![3, 8, 3, 0, 6, 4]);
        assert_eq!(set.counts(), &[8, 6, 4, 3]);
        assert!(!set.is_empty());
        assert!(crate::parser::meta_ast::CountSet::new(vec![0]).is_empty());
    }

    /// THE PAYOFF (PLAN-122 W1.0): the colour grammar matches the counted hex
    /// terminal and REFUSES illegal lengths.
    ///
    /// Every other counted-class test above feeds the extractor a token stream
    /// that already has the shape it wants. This one starts from SOURCE, at a
    /// real value position (`{ ink: … }`), and runs the actual compiled
    /// terminals — `LiteralExtractor("#")` then `CharClassExtractor` — in
    /// sequence, exactly as the form compiler assembles
    ///
    ///     %capture_type color { "#" [0-9a-fA-F]{3|4|6|8} | … }
    ///
    /// so it measures the claim the cutover rests on rather than restating it.
    ///
    /// Why the grammar can hold this domain at all: the lexer used to fuse
    /// `#e8eef7` into one COLOR token in a value position. An extractor consumes
    /// whole tokens (`ExtractResult` = `(value, tokens_consumed)`), and there is
    /// no way to consume half of one — so a grammar cannot own a domain the
    /// lexer has already fused. PLAN-122 handed the domain back (the lexer now
    /// emits `HASH` + `IDENT`), which is why the stdlib grammars could not simply
    /// have been written first: while the lexer fused, they would have compiled,
    /// matched nothing, and failed silently in precisely the position they exist
    /// to serve.
    ///
    /// The legal lengths match and the illegal ones are refused. The refusal is
    /// the user-visible prize: `#e8ee1` used to pass `check` and land verbatim
    /// in the emitted stylesheet, where the browser drops the declaration and
    /// the author sees an element rendering wrong with no diagnostic anywhere in
    /// the toolchain.
    #[test]
    fn colour_grammar_accepts_legal_hex_lengths_and_refuses_illegal_ones() {
        /// The two real terminals, run in sequence at the value position.
        fn colour(source: &str) -> Option<String> {
            let tokens = lex_lit(source);
            let at = tokens.iter().position(|t| t.kind == SyntaxKind::COLON)? + 1;
            let value = &tokens[at..];

            let (_, consumed) = LiteralExtractor("#".to_string()).extract(value, source)?;
            let hex = CharClassExtractor {
                chars: "0-9a-fA-F".to_string(),
                negated: false,
                counts: Some(crate::parser::meta_ast::CountSet::new(vec![3, 4, 6, 8])),
            };
            match hex.extract(&value[consumed..], source)? {
                (CapturedValue::String(s), _) => Some(s),
                _ => None,
            }
        }

        // Every legal CSS hex length, one case each.
        for (source, expected) in [
            ("{ ink: #fff; }", "fff"),
            ("{ ink: #fffa; }", "fffa"),
            ("{ ink: #e8eef7; }", "e8eef7"),
            ("{ ink: #11223344; }", "11223344"),
            // Case-folding: CSS is case-insensitive, and the grammar says so once.
            ("{ ink: #E8EEF7; }", "E8EEF7"),
        ] {
            assert_eq!(
                colour(source).as_deref(),
                Some(expected),
                "{source:?}: the grammar failed to match a legal hex — the lexer did \
                 not actually hand the domain back to the grammar"
            );
        }

        // Malformed colours are REFUSED rather than truncated to a legal prefix.
        for source in [
            "{ ink: #e8ee1; }",   // 5 digits — no such CSS colour
            "{ ink: #e8eef71; }", // 7 digits — nor this
            "{ ink: #zz; }",      // not hex at all
        ] {
            assert_eq!(
                colour(source),
                None,
                "{source:?} matched — an illegal colour must be refused outright, or \
                 the failure surfaces somewhere less obvious than the literal itself"
            );
        }
    }

    #[test]
    fn literal_extractor_fat_arrow_single_token() {
        // FAT_ARROW is a first-class token, so the literal takes the fast path.
        let ext = LiteralExtractor("=>".to_string());
        let src = "=>";
        let toks = lex_lit(src);
        assert_eq!(toks.len(), 1, "=> must lex as one FAT_ARROW token");
        assert_eq!(toks[0].kind, SyntaxKind::FAT_ARROW);
        assert_eq!(
            ext.extract(&toks, src),
            Some((CapturedValue::String("=>".to_string()), 1))
        );
    }

    #[test]
    fn literal_extractor_multitoken_at_directive() {
        // `@match` lexes as AT_SIGN + IDENT.
        let ext = LiteralExtractor("@match".to_string());
        let src = "@match";
        let toks = lex_lit(src);
        let r = ext.extract(&toks, src);
        assert_eq!(
            r,
            Some((CapturedValue::String("@match".to_string()), toks.len()))
        );
    }

    #[test]
    fn literal_extractor_multitoken_rejects_gap() {
        // `= >` (with a space) must NOT match the literal `=>` — the tokens are not
        // contiguous in source, so a spurious cross-boundary match is rejected.
        let ext = LiteralExtractor("=>".to_string());
        let src = "= >";
        let toks = lex_lit(src);
        assert_eq!(
            ext.extract(&toks, src),
            None,
            "gap between = and > must not match =>"
        );
    }

    #[test]
    fn literal_extractor_multitoken_leading_ws_ok() {
        // Leading whitespace before a multi-token literal is skipped (as for single-token).
        let ext = LiteralExtractor("=>".to_string());
        let src = "  =>";
        let toks = lex_lit(src);
        assert!(
            ext.extract(&toks, src).is_some(),
            "leading ws before => should be skipped"
        );
    }

    #[test]
    fn literal_extractor_matches() {
        let ext = LiteralExtractor(",".to_string());
        let source = ",";
        let tokens = [tok(SyntaxKind::COMMA, 0, 1)];
        let result = ext.extract(&tokens, source);
        assert!(result.is_some());
        assert_eq!(result.unwrap().1, 1);
    }

    #[test]
    fn literal_extractor_rejects() {
        let ext = LiteralExtractor(",".to_string());
        let source = ";";
        let tokens = [tok(SyntaxKind::SEMICOLON, 0, 1)];
        assert!(ext.extract(&tokens, source).is_none());
    }

    #[test]
    fn optional_extractor_succeeds() {
        let ext = OptionalExtractor {
            inner: Box::new(super::super::simple::IdentExtractor),
        };
        let source = "hello";
        let tokens = [tok(SyntaxKind::IDENT, 0, 5)];
        let result = ext.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("hello".to_string()), 1)));
    }

    #[test]
    fn optional_extractor_returns_empty_on_failure() {
        let ext = OptionalExtractor {
            inner: Box::new(super::super::simple::IdentExtractor),
        };
        let source = "42";
        let tokens = [tok(SyntaxKind::NUMBER, 0, 2)];
        let result = ext.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::String(String::new()), 0)));
    }

    #[test]
    fn repeat_extractor_zero_or_more() {
        let ext = RepeatExtractor {
            inner: Box::new(super::super::simple::IdentExtractor),
            at_least_one: false,
        };
        // Empty input — succeeds with empty array
        let source = "";
        let tokens: [TokenData; 0] = [];
        let result = ext.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Array(vec![]), 0)));
    }

    #[test]
    fn repeat_extractor_one_or_more_fails_empty() {
        let ext = RepeatExtractor {
            inner: Box::new(super::super::simple::IdentExtractor),
            at_least_one: true,
        };
        let source = "";
        let tokens: [TokenData; 0] = [];
        assert!(ext.extract(&tokens, source).is_none());
    }

    #[test]
    fn repeat_extractor_collects_values() {
        let ext = RepeatExtractor {
            inner: Box::new(super::super::simple::IdentExtractor),
            at_least_one: false,
        };
        let source = "abc def";
        let tokens = [tok(SyntaxKind::IDENT, 0, 3), tok(SyntaxKind::IDENT, 4, 7)];
        let result = ext.extract(&tokens, source);
        let (val, consumed) = result.unwrap();
        assert_eq!(consumed, 2);
        match val {
            CapturedValue::Array(arr) => assert_eq!(arr.len(), 2),
            _ => panic!("Expected Array"),
        }
    }

    #[test]
    fn sequence_extractor_matches_all() {
        let ext = SequenceExtractor(vec![
            (None, Box::new(super::super::simple::IdentExtractor)),
            (None, Box::new(LiteralExtractor(",".to_string()))),
            (None, Box::new(super::super::simple::IdentExtractor)),
        ]);
        let source = "abc,def";
        let tokens = [
            tok(SyntaxKind::IDENT, 0, 3),
            tok(SyntaxKind::COMMA, 3, 4),
            tok(SyntaxKind::IDENT, 4, 7),
        ];
        let result = ext.extract(&tokens, source);
        assert!(result.is_some());
        assert_eq!(result.unwrap().1, 3);
    }

    #[test]
    fn sequence_extractor_fails_partial() {
        let ext = SequenceExtractor(vec![
            (None, Box::new(super::super::simple::IdentExtractor)),
            (None, Box::new(LiteralExtractor(",".to_string()))),
        ]);
        let source = "abc;";
        let tokens = [
            tok(SyntaxKind::IDENT, 0, 3),
            tok(SyntaxKind::SEMICOLON, 3, 4),
        ];
        assert!(ext.extract(&tokens, source).is_none());
    }

    #[test]
    fn choice_extractor_first_match() {
        let ext = ChoiceExtractor(vec![
            (None, Box::new(super::super::simple::NumberExtractor)),
            (None, Box::new(super::super::simple::IdentExtractor)),
        ]);
        let source = "hello";
        let tokens = [tok(SyntaxKind::IDENT, 0, 5)];
        let result = ext.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("hello".to_string()), 1)));
    }

    #[test]
    fn compile_pattern_simple_capture() {
        let registry = ExtractorRegistry::new();
        let pattern = CapturePatternAst::Capture {
            var_name: "name".to_string(),
            capture_type: CaptureType::Ident,
            modifier: CaptureModifier::Required,
        };
        let extractor = compile_pattern(&pattern, &registry);

        let source = "hello";
        let tokens = [tok(SyntaxKind::IDENT, 0, 5)];
        let result = extractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("hello".to_string()), 1)));
    }

    #[test]
    fn compile_pattern_optional_capture() {
        let registry = ExtractorRegistry::new();
        let pattern = CapturePatternAst::Capture {
            var_name: "name".to_string(),
            capture_type: CaptureType::Ident,
            modifier: CaptureModifier::Optional,
        };
        let extractor = compile_pattern(&pattern, &registry);

        // Should succeed even with non-matching input
        let source = "42";
        let tokens = [tok(SyntaxKind::NUMBER, 0, 2)];
        let result = extractor.extract(&tokens, source);
        assert!(result.is_some());
        assert_eq!(result.unwrap().1, 0); // consumed nothing
    }

    // === PLAN-023 W2 review hardening: depth/op-aware splits ===

    #[test]
    fn split_on_bare_eq_ignores_operators_and_nesting() {
        // bare = splits
        assert_eq!(split_on_bare_eq("number = 0"), Some(("number ", " 0")));
        // == / >= / <= / != / => are NOT splits
        assert_eq!(
            split_on_bare_eq("bool = a == b"),
            Some(("bool ", " a == b"))
        );
        assert_eq!(split_on_bare_eq("a >= b"), None);
        assert_eq!(split_on_bare_eq("a <= b"), None);
        assert_eq!(split_on_bare_eq("a != b"), None);
        assert_eq!(split_on_bare_eq("(x) => y"), None);
        // = nested in brackets is not a top-level split
        assert_eq!(
            split_on_bare_eq("obj = {a = 1}"),
            Some(("obj ", " {a = 1}"))
        );
        // no = at all
        assert_eq!(split_on_bare_eq("number"), None);
    }

    #[test]
    fn split_on_top_level_arrow_respects_nesting_and_strings() {
        assert_eq!(split_on_top_level_arrow("0 -> 1"), vec!["0", "1"]);
        assert_eq!(
            split_on_top_level_arrow("1 -> 0.95 -> 1"),
            vec!["1", "0.95", "1"]
        );
        // arrow inside parens does NOT split
        assert_eq!(
            split_on_top_level_arrow("rotate(0) -> translateX(calc(100% - 10px))"),
            vec!["rotate(0)", "translateX(calc(100% - 10px))"]
        );
        // a function arg that itself contains -> stays in one segment
        assert_eq!(
            split_on_top_level_arrow("f(a -> b) -> c"),
            vec!["f(a -> b)", "c"]
        );
        // arrow inside a string literal does NOT split
        assert_eq!(
            split_on_top_level_arrow(r#""a->b" -> "c""#),
            vec![r#""a->b""#, r#""c""#]
        );
    }

    // === PLAN-023 W2 S5: keyframes — differential (stdlib grammar+reifier == Rust) ===

    /// GOLDEN: the stdlib keyframes grammar+reifier output. Proven == the (now-deleted) Rust
    /// KeyframesExtractor via differential before deletion (PLAN-023 W2).
    #[test]
    fn keyframes_via_stdlib_matches_rust_extractor() {
        use crate::syntax::form_match::KeyframeDef;
        let kf = |property: &str, values: &[&str]| KeyframeDef {
            selector: None,
            property: property.to_string(),
            values: values.iter().map(|s| s.to_string()).collect(),
        };
        let stdlib_ext = stdlib_extractor("keyframes");
        let cases: Vec<(&str, Vec<KeyframeDef>)> = vec![
            ("opacity: 0 -> 1", vec![kf("opacity", &["0", "1"])]),
            (
                "scale: 1 -> 0.95 -> 1;",
                vec![kf("scale", &["1", "0.95", "1"])],
            ),
            (
                "translate-y: 20px -> 0",
                vec![kf("translate-y", &["20px", "0"])],
            ),
            (
                "opacity: 0 -> 1; scale: 1 -> 1.1",
                vec![kf("opacity", &["0", "1"]), kf("scale", &["1", "1.1"])],
            ),
            (
                "box-shadow: none -> 0 4px 20px rgba(0,0,0,0.15)",
                vec![kf("box-shadow", &["none", "0 4px 20px rgba(0,0,0,0.15)"])],
            ),
            ("color: red", vec![kf("color", &["red"])]),
        ];
        for (src, expected) in cases {
            let tokens = lex(src);
            let got = match stdlib_ext.extract(&tokens, src).map(|(v, _)| v) {
                Some(CapturedValue::Keyframes(k)) => Some(k),
                _ => None,
            };
            assert_eq!(got, Some(expected), "keyframes golden mismatch for {src:?}");
        }
    }

    /// Build the production stdlib extractor for a capture type by name (compile its
    /// registered grammar + wrap with its reifier). Shared by the golden tests.
    fn stdlib_extractor(name: &str) -> Box<dyn CaptureExtractor> {
        let reg = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
        let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
            reg.capture_types()
                .map(|c| (c.name.clone(), c.clone()))
                .collect();
        let ct = defs
            .get(name)
            .unwrap_or_else(|| panic!("{name} stdlib %capture_type must be registered"));
        let extractors = ExtractorRegistry::new();
        wrap_reifier(
            name,
            compile_pattern_with_defs(&ct.pattern, &extractors, &defs, &mut Vec::new()),
        )
    }

    // NB (FUP-040): the stdlib `params` %capture_type (colon-form `name: type`) was
    // RETIRED — @fn / @compute migrated to `param_list` (space-form `$name type`), the
    // one author-facing param-declaration surface. Its differential golden is replaced
    // by `param_list_via_stdlib_matches_rust_extractor` below. The Rust
    // `CaptureType::Params` variant + `reify_params` remain only as internal value
    // shapes; no stdlib grammar or author surface produces them now.

    // === PLAN-023 W2 S3: properties — differential (stdlib grammar+reifier == Rust) ===

    /// GOLDEN: stdlib properties grammar+reifier output, including the nested @-directive
    /// skip (via skip_block). Proven == the deleted Rust PropertiesExtractor (PLAN-023 W2).
    #[test]
    fn properties_via_stdlib_matches_rust_extractor() {
        use crate::syntax::form_match::PropertyDef;
        let pd = |name: &str, type_ref: &str, optional: bool| PropertyDef {
            name: name.to_string(),
            type_ref: type_ref.to_string(),
            optional,
        };
        let stdlib_ext = stdlib_extractor("properties");
        let cases: Vec<(&str, Vec<PropertyDef>)> = vec![
            (
                "id: string; price: number; name: string",
                vec![
                    pd("id", "string", false),
                    pd("price", "number", false),
                    pd("name", "string", false),
                ],
            ),
            ("id: string", vec![pd("id", "string", false)]),
            (
                "title?: string; count: number",
                vec![pd("title", "string", true), pd("count", "number", false)],
            ),
            (
                "position: fixed; background: white",
                vec![
                    pd("position", "fixed", false),
                    pd("background", "white", false),
                ],
            ),
            (
                // nested @-directive skipped; later declaration still collected
                "position: fixed; @dark { background: black; } display: block",
                vec![
                    pd("position", "fixed", false),
                    pd("display", "block", false),
                ],
            ),
            (
                "x: number; y: number;",
                vec![pd("x", "number", false), pd("y", "number", false)],
            ),
        ];
        for (src, expected) in cases {
            let tokens = lex(src);
            let got = match stdlib_ext.extract(&tokens, src).map(|(v, _)| v) {
                Some(CapturedValue::Properties(p)) => Some(p),
                _ => None,
            };
            assert_eq!(
                got,
                Some(expected),
                "properties golden mismatch for {src:?}"
            );
        }
    }

    // === PLAN-023 W2 S2: param_list keystone — differential (stdlib grammar+reifier == Rust) ===

    /// GOLDEN: the stdlib param_list grammar+reifier produces the canonical
    /// ParamList(Vec<TemplateParamDef>) shape. These golden values were proven equal to the
    /// legacy Rust ParamListExtractor via a differential test before that extractor was
    /// deleted (PLAN-023 W2). The test now pins the behavior to the goldens so the stdlib
    /// grammar can't silently regress.
    #[test]
    fn param_list_via_stdlib_matches_rust_extractor() {
        use crate::syntax::form_match::{TemplateParamDef, TemplateParamKind};
        let bind = |name: &str, optional: bool| TemplateParamDef {
            name: name.to_string(),
            kind: TemplateParamKind::Binding,
            optional,
            type_ref: None,
            default: None,
            collection: false,
        };
        let elem = |name: &str, optional: bool| TemplateParamDef {
            name: name.to_string(),
            kind: TemplateParamKind::Element,
            optional,
            type_ref: None,
            default: None,
            collection: false,
        };
        // Element param carrying a `[]` collection marker (`&items[]`).
        let elem_coll = |name: &str| TemplateParamDef {
            name: name.to_string(),
            kind: TemplateParamKind::Element,
            optional: false,
            type_ref: None,
            default: None,
            collection: true,
        };
        // Build the stdlib extractor from the REAL registered grammar so the test tracks the
        // actual .st definition, not a hand copy.
        let reg = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
        let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
            reg.capture_types()
                .map(|c| (c.name.clone(), c.clone()))
                .collect();
        let ct = defs
            .get("param_list")
            .expect("param_list must be a registered stdlib %capture_type");
        let extractors = ExtractorRegistry::new();
        let stdlib_ext = wrap_reifier(
            "param_list",
            compile_pattern_with_defs(&ct.pattern, &extractors, &defs, &mut Vec::new()),
        );

        // Typed param helper: `$href url`, `$alt string = ""` (FEAT-096 SPACE form).
        let typed = |name: &str, ty: &str, default: Option<&str>| TemplateParamDef {
            name: name.to_string(),
            kind: TemplateParamKind::Binding,
            optional: false,
            type_ref: Some(ty.to_string()),
            default: default.map(|d| d.to_string()),
            collection: false,
        };
        let cases: [(&str, Vec<TemplateParamDef>); 14] = [
            ("$title", vec![bind("title", false)]),
            (
                "$a, &b, $c",
                vec![bind("a", false), elem("b", false), bind("c", false)],
            ),
            (
                "$title, &content, $footer",
                vec![
                    bind("title", false),
                    elem("content", false),
                    bind("footer", false),
                ],
            ),
            ("$x?", vec![bind("x", true)]),
            (
                "$a, $b?, &c?",
                vec![bind("a", false), bind("b", true), elem("c", true)],
            ),
            ("&only", vec![elem("only", false)]),
            ("", vec![]),
            // FEAT-096: typed value params (SPACE form) + `= default`.
            ("$href url", vec![typed("href", "url", None)]),
            (
                "$src url, $alt string",
                vec![typed("src", "url", None), typed("alt", "string", None)],
            ),
            (
                "$alt string = \"\"",
                vec![typed("alt", "string", Some("\"\""))],
            ),
            // Mixed: untyped selection + typed attr param (the @mark &link shape).
            (
                "$sel, $href url",
                vec![bind("sel", false), typed("href", "url", None)],
            ),
            // FUP-043: string-literal-led union type annotation round-trips verbatim
            // (parity with the parenthesized `%primitive` union form).
            (
                "$size \"S\" | \"M\" | \"L\"",
                vec![typed("size", "\"S\" | \"M\" | \"L\"", None)],
            ),
            // FUP-042 R3: `&items[]` collection marker → element param, collection=true.
            ("&items[]", vec![elem_coll("items")]),
            (
                "&sel, &items[]",
                vec![elem("sel", false), elem_coll("items")],
            ),
        ];
        for (src, expected) in cases {
            let tokens = lex(src);
            let stdlib = stdlib_ext.extract(&tokens, src).map(|(v, _)| v);
            let norm = |v: Option<CapturedValue>| match v {
                Some(CapturedValue::ParamList(p)) => Some(p),
                _ => None,
            };
            assert_eq!(
                norm(stdlib),
                Some(expected),
                "param_list golden mismatch for input {src:?}"
            );
        }
    }

    // === PLAN-023 W2 S1: named-capture retention (structured records) ===

    /// A named sequence `$prop:ident ":" $val:balanced(';')` must produce a Named record
    /// mapping prop+val (the foundation reifiers consume).
    #[test]
    fn sequence_with_names_builds_named_record() {
        use crate::parser::meta_ast::{CaptureModifier, CapturePatternAst, CaptureType};
        let registry = ExtractorRegistry::new();
        let pattern = CapturePatternAst::Sequence(vec![
            CapturePatternAst::Capture {
                var_name: "prop".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
            },
            CapturePatternAst::Literal(":".to_string()),
            CapturePatternAst::Capture {
                var_name: "val".to_string(),
                capture_type: CaptureType::Balanced(';'),
                modifier: CaptureModifier::Required,
            },
        ]);
        let ext = compile_pattern(&pattern, &registry);
        let source = "opacity: 0 -> 1 ;";
        let tokens = lex(source);
        let (val, _) = ext.extract(&tokens, source).expect("should match");
        match val {
            CapturedValue::Named(map) => {
                assert!(matches!(map.get("prop"), Some(CapturedValue::Ident(s)) if s == "opacity"));
                assert!(matches!(map.get("val"), Some(CapturedValue::Expr(s)) if s == "0 -> 1"));
            }
            other => panic!("expected Named record, got {other:?}"),
        }
    }

    /// An UNNAMED sequence (pure literal/charclass glue) keeps legacy last_value semantics
    /// — critical for back-compat of existing %capture_type compositions.
    #[test]
    fn sequence_without_names_keeps_last_value() {
        use crate::parser::meta_ast::CapturePatternAst;
        let registry = ExtractorRegistry::new();
        let pattern = CapturePatternAst::Sequence(vec![
            CapturePatternAst::Literal("a".to_string()),
            CapturePatternAst::Literal("b".to_string()),
        ]);
        let ext = compile_pattern(&pattern, &registry);
        let source = "a b";
        let tokens = lex(source);
        let (val, _) = ext.extract(&tokens, source).expect("should match");
        // Last literal value, NOT a Named map.
        assert!(matches!(val, CapturedValue::String(s) if s == "b"));
    }

    /// A named Choice records WHICH branch matched (the discriminator reifiers need).
    #[test]
    fn choice_with_names_tags_matched_branch() {
        use crate::parser::meta_ast::{CaptureModifier, CapturePatternAst, CaptureType};
        let registry = ExtractorRegistry::new();
        // ( $bind:binding ) | ( &elem:element )
        let pattern = CapturePatternAst::Choice(vec![
            CapturePatternAst::Capture {
                var_name: "bind".to_string(),
                capture_type: CaptureType::Binding,
                modifier: CaptureModifier::Required,
            },
            CapturePatternAst::Capture {
                var_name: "elem".to_string(),
                capture_type: CaptureType::Element,
                modifier: CaptureModifier::Required,
            },
        ]);
        let ext = compile_pattern(&pattern, &registry);
        // `$count` matches the binding branch.
        let source = "$count";
        let tokens = lex(source);
        let (val, _) = ext.extract(&tokens, source).expect("should match");
        match val {
            CapturedValue::Named(map) => {
                assert!(map.contains_key("bind"), "binding branch should be tagged");
                assert!(!map.contains_key("elem"));
            }
            other => panic!("expected Named (branch-tagged), got {other:?}"),
        }
    }

    // === PLAN-023 W2: BalancedExtractor ===

    /// Lex real source into TokenData (mirrors the production Token -> TokenData mapping in
    /// form_compiler.rs). Used by both unit and property tests so the extractor is exercised
    /// against the ACTUAL lexer, not hand-built token streams.
    fn lex(source: &str) -> Vec<TokenData> {
        crate::syntax::cst::lexer::Lexer::new(source)
            .tokenize()
            .into_iter()
            .filter(|t| t.kind != SyntaxKind::EOF)
            .map(|t| TokenData {
                kind: t.kind,
                text_range: (t.offset, t.offset + t.len()),
            })
            .collect()
    }

    /// The text the extractor captured, given source + delimiter.
    fn balanced_of(source: &str, delim: char) -> Option<(String, usize)> {
        let ext = BalancedExtractor { delim };
        let tokens = lex(source);
        ext.extract(&tokens, source).map(|(v, n)| match v {
            CapturedValue::Expr(s) => (s, n),
            other => panic!("expected Expr, got {other:?}"),
        })
    }

    #[test]
    fn balanced_simple_until_semicolon() {
        let (text, _) = balanced_of("a + b ; rest", ';').unwrap();
        assert_eq!(text, "a + b");
    }

    #[test]
    fn balanced_respects_paren_nesting() {
        // The ';' inside f(x; y) must NOT terminate the run.
        let (text, _) = balanced_of("f(x ; y) + 1 ; tail", ';').unwrap();
        assert_eq!(text, "f(x ; y) + 1");
    }

    #[test]
    fn balanced_respects_all_bracket_kinds() {
        let (text, _) = balanced_of("g([a ; b], {c ; d}) ; tail", ';').unwrap();
        assert_eq!(text, "g([a ; b], {c ; d})");
    }

    #[test]
    fn balanced_stops_at_unmatched_closer_depth0() {
        // A depth-0 closing brace ends the run (e.g. the `}` closing a macro body).
        let (text, _) = balanced_of("a + b }", ';').unwrap();
        assert_eq!(text, "a + b");
    }

    #[test]
    fn balanced_to_eof_when_no_delim() {
        let (text, _) = balanced_of("a + b + c", ';').unwrap();
        assert_eq!(text, "a + b + c");
    }

    #[test]
    fn balanced_empty_before_delim_is_none() {
        assert!(balanced_of("; rest", ';').is_none());
    }

    #[test]
    fn balanced_does_not_consume_delim() {
        // The consumed-token count must stop BEFORE the delimiter so a following Literal
        // in a sequence can match it (mirrors raw_until_semicolon + ";"?).
        let ext = BalancedExtractor { delim: ';' };
        let source = "a ; b";
        let tokens = lex(source);
        let (_, consumed) = ext.extract(&tokens, source).unwrap();
        // Token at index `consumed` must be the delimiter (possibly after whitespace).
        let mut i = consumed;
        while i < tokens.len() && tokens[i].kind.is_trivia() {
            i += 1;
        }
        assert_eq!(tokens[i].text(source), ";");
    }

    #[test]
    fn balanced_alternate_delimiter_comma() {
        let (text, _) = balanced_of("a(1, 2) , next", ',').unwrap();
        assert_eq!(text, "a(1, 2)");
    }

    // === BUG-100: comment-aware balanced scanning ===

    #[test]
    fn balanced_line_comment_with_apostrophe_does_not_desync() {
        let src = "$noop <- 1;\n// serde's default\n$noop <- 2;\n}";
        let (text, _) =
            balanced_of(src, '\u{0}').unwrap_or_else(|| panic!("expected a match for {src:?}"));
        assert!(
            text.contains("$noop <- 2"),
            "trailing statement must survive an apostrophe inside a // comment: {text:?}"
        );
        assert!(
            !text.contains('}'),
            "scan must stop at the depth-0 closer, not run past it: {text:?}"
        );
    }

    #[test]
    fn balanced_line_comment_with_double_colon_does_not_splice() {
        let src = "$noop <- 1;\n// std::time::Duration\n$noop <- 2;\n}";
        let (text, _) = balanced_of(src, '\u{0}').unwrap();
        assert!(
            text.contains("$noop <- 2"),
            "trailing statement must survive a :: inside a // comment: {text:?}"
        );
        assert!(!text.contains('}'));
    }

    #[test]
    fn balanced_block_comment_with_double_colon_does_not_splice() {
        let src = "$noop <- 1;\n/* spacetime_host_core::auth::DeviceFlowStart */\n$noop <- 2;\n}";
        let (text, _) = balanced_of(src, '\u{0}').unwrap();
        assert!(text.contains("$noop <- 2"), "got: {text:?}");
        assert!(!text.contains('}'));
    }

    /// End-to-end via the REAL stdlib handle_arm %capture_type: an apostrophe inside a
    /// `//` comment in the arm body must not desync the capture, and the reified
    /// `body` field must come back as `CapturedValue::String` (BUG-100 fix #2 — the
    /// field must reach codegen as a quoted string, not a raw `Expr` splice).
    #[test]
    fn handle_arm_stdlib_survives_apostrophe_comment() {
        let src = "Started(body) => {\n      $noop <- 1;\n      // serde's default\n      $noop <- 2;\n    }";
        let ext = stdlib_extractor("handle_arm");
        let tokens = lex(src);
        let (val, _consumed) = ext
            .extract(&tokens, src)
            .unwrap_or_else(|| panic!("handle_arm should match {src:?}"));
        let CapturedValue::Named(map) = &val else {
            panic!("expected Named record, got {val:?}");
        };
        assert_eq!(
            map.get("ctor"),
            Some(&CapturedValue::Ident("Started".to_string()))
        );
        match map.get("body") {
            Some(CapturedValue::String(s)) => {
                assert!(s.contains("$noop <- 1"), "got: {s:?}");
                assert!(s.contains("serde's default"), "got: {s:?}");
                assert!(
                    s.contains("$noop <- 2"),
                    "trailing statement must survive: {s:?}"
                );
            }
            other => panic!("expected body to be reified to String, got {other:?}"),
        }
    }

    /// End-to-end via handle_receive (the `(...)+` repetition over handle_arm) with
    /// TWO arms, apostrophe comment in the first — both arms must survive intact.
    #[test]
    fn handle_receive_stdlib_survives_apostrophe_comment() {
        let src = "receive {\n    Started(body) => {\n      $noop <- 1;\n      // serde's default\n      $noop <- 2;\n    }\n    Failed(_) => { }\n  }";
        let ext = stdlib_extractor("handle_receive");
        let tokens = lex(src);
        let (val, _consumed) = ext
            .extract(&tokens, src)
            .unwrap_or_else(|| panic!("handle_receive should match {src:?}"));
        let CapturedValue::Named(map) = &val else {
            panic!("expected Named record, got {val:?}");
        };
        let Some(CapturedValue::Array(arms)) = map.get("arms") else {
            panic!("expected arms array, got {:?}", map.get("arms"));
        };
        assert_eq!(arms.len(), 2, "both arms must be captured: {arms:?}");
        let CapturedValue::Named(first) = &arms[0] else {
            panic!("expected first arm to be Named");
        };
        match first.get("body") {
            Some(CapturedValue::String(s)) => {
                assert!(s.contains("$noop <- 2"), "got: {s:?}");
            }
            other => panic!("expected first arm body to be String, got {other:?}"),
        }
    }

    /// Full pipeline: parse the ACTUAL BUG-100 repro `.st` source through the whole
    /// `syntax::events::parse_matches` pipeline (CST -> node context -> try_match),
    /// exactly as the real compiler does, then check the resulting `handle` match's
    /// captured `arms` — the actual end-to-end fix for both BUG-100 root causes
    /// together.
    #[test]
    fn handle_directive_full_pipeline_survives_apostrophe_comment() {
        let src = "@data signal $go {\n  fire { }\n  receive to Result {\n    Started(body) as {} => { }\n    Failed(msg) as string => { }\n  }\n}\n\n@handle $go {\n  receive {\n    Started(body) => {\n      $noop <- 1;\n      // serde's default\n      $noop <- 2;\n    }\n    Failed(_) => { }\n  }\n}\n";
        let (matches, diags) = crate::syntax::events::parse_matches(
            src,
            &crate::syntax::stdlib_registry::STDLIB_REGISTRY,
        );
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let handle_match = matches
            .iter()
            .find(|m| m.macro_name == "handle")
            .unwrap_or_else(|| panic!("no handle match found in {matches:?}"));
        let Some(CapturedValue::Array(arms)) = handle_match.captures.get("arms") else {
            panic!(
                "expected arms array, got {:?}",
                handle_match.captures.get("arms")
            );
        };
        assert_eq!(arms.len(), 2, "both arms must survive: {arms:?}");
        let CapturedValue::Named(first) = &arms[0] else {
            panic!("expected first arm to be Named");
        };
        match first.get("body") {
            Some(CapturedValue::String(s)) => {
                assert!(s.contains("$noop <- 1"), "got: {s:?}");
                assert!(s.contains("serde's default"), "got: {s:?}");
                assert!(s.contains("$noop <- 2"), "got: {s:?}");
            }
            other => panic!("expected body reified to String, got {other:?}"),
        }
    }

    // --- Property / fuzz tests (PLAN-023 mandate) ---

    use proptest::prelude::*;

    /// Generate a balanced bracket string (never contains the delimiter ';' at depth 0).
    /// Atoms avoid ';' entirely; nesting is balanced by construction.
    fn balanced_atom() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-z]{1,3}".prop_map(|s| s),
            Just(" ".to_string()),
            Just("+".to_string()),
        ]
    }

    fn balanced_expr() -> impl Strategy<Value = String> {
        let leaf = prop::collection::vec(balanced_atom(), 0..4).prop_map(|v| v.join(""));
        leaf.prop_recursive(3, 24, 3, |inner| {
            prop_oneof![
                (inner.clone()).prop_map(|s| format!("({s})")),
                (inner.clone()).prop_map(|s| format!("[{s}]")),
                (inner.clone()).prop_map(|s| format!("{{{s}}}")),
                prop::collection::vec(inner, 1..4).prop_map(|v| v.join("")),
            ]
        })
    }

    proptest! {
        /// Property 1: a balanced prefix followed by ';' is captured EXACTLY up to (not
        /// including) the depth-0 delimiter, and the delimiter is never split inside nesting.
        #[test]
        fn prop_balanced_stops_at_depth0_delim(prefix in balanced_expr(), suffix in balanced_expr()) {
            let source = format!("{prefix};{suffix}");
            let result = balanced_of(&source, ';');
            if prefix.trim().is_empty() {
                // Empty (or whitespace-only) prefix => no content before delim => None.
                prop_assert!(result.is_none());
            } else {
                let (text, _) = result.unwrap();
                // Captured text is the trimmed prefix (delimiter not consumed, suffix untouched).
                prop_assert_eq!(text, prefix.trim().to_string());
            }
        }

        /// Property 2: a ';' nested inside brackets NEVER terminates the run.
        #[test]
        fn prop_balanced_ignores_nested_delim(inner in balanced_expr()) {
            let source = format!("({inner};{inner})x");
            let (text, _) = balanced_of(&source, ';').unwrap();
            // The whole parenthesised group is captured; the inner ';' did not split it.
            prop_assert!(text.starts_with('('));
            prop_assert!(text.contains(';'));
        }

        /// Property 3: arbitrary (possibly unbalanced) input never panics and never returns
        /// a consumed count past the token stream.
        #[test]
        fn prop_balanced_no_panic_on_arbitrary(s in ".{0,40}") {
            let ext = BalancedExtractor { delim: ';' };
            let tokens = lex(&s);
            let result = ext.extract(&tokens, &s);
            if let Some((_, consumed)) = result {
                prop_assert!(consumed <= tokens.len());
            }
        }
    }
}

#[cfg(test)]
mod driver_form_probe {
    use super::*;

    fn lex_lit(source: &str) -> Vec<TokenData> {
        crate::syntax::cst::lexer::Lexer::new(source)
            .tokenize()
            .into_iter()
            .filter(|t| t.kind != SyntaxKind::EOF)
            .map(|t| TokenData {
                kind: t.kind,
                text_range: (t.offset, t.offset + t.len()),
            })
            .collect()
    }

    /// BUG-246 regression: two custom types in sequence — the second one's
    /// optional paren group lost its contents when the first consumed its
    /// own. `@on &.visible(threshold: 0.2) --rise(distance: 8px);` dropped
    /// the form's args (defaults silently substituted).
    #[test]
    fn driver_params_do_not_eat_form_args() {
        let _ = crate::compiler::load_stdlib_registry();
        let (matches, diags) = crate::syntax::events::parse_matches(
            ".x { @on &.visible(threshold: 0.2) --rise(distance: 8px); }",
            &crate::syntax::stdlib_registry::STDLIB_REGISTRY,
        );
        assert!(diags.is_empty(), "diags: {diags:?}");
        assert_eq!(matches.len(), 1, "the @on directive matches");
        // PLAN-150 W3-arc: the bare `@on <driver> --form;` statement now captures
        // its consequence under `consequence` (an `on_consequence`, whose form
        // arm nests the `form_application` under `form`), lowering like the colon
        // form — so the call args are at `consequence.form.args`.
        let consequence = matches[0]
            .captures
            .get("consequence")
            .expect("consequence capture");
        let CapturedValue::Named(cmap) = consequence else {
            panic!("consequence must be a Named map: {consequence:?}");
        };
        let form = cmap.get("form").expect("consequence.form");
        let CapturedValue::Named(map) = form else {
            panic!("form capture must be a Named map: {form:?}");
        };
        let args = map
            .get("args")
            .expect("the form's call args survive a preceding driver-params group (BUG-246)");
        let CapturedValue::Array(items) = args else {
            panic!("args must be an Array: {args:?}");
        };
        assert_eq!(items.len(), 1, "one call arg: {items:?}");
        let CapturedValue::Named(arg) = &items[0] else {
            panic!("arg is a Named map: {items:?}");
        };
        assert!(
            matches!(arg.get("value"), Some(CapturedValue::Expr(v)) if v == "8px"),
            "the arg VALUE is 8px: {arg:?}"
        );
    }
}

#[cfg(test)]
mod scalar_flattening_reads_the_table {
    use super::*;
    use crate::syntax::events::extractors::{CaptureExtractor, ExtractorRegistry, TokenData};

    fn shape(ct: &str, src: &str) -> String {
        let reg = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
        let ereg = ExtractorRegistry::new();
        let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
            reg.capture_types().map(|c| (c.name.clone(), c.clone())).collect();
        let def = defs.get(ct).unwrap_or_else(|| panic!("no %capture_type {ct}"));
        let inner = compile_pattern_with_defs(&def.pattern, &ereg, &defs, &mut vec![]);
        let ext = wrap_reifier(ct, inner);
        let toks: Vec<TokenData> = crate::syntax::cst::lexer::Lexer::new(src)
            .tokenize()
            .into_iter()
            .filter(|t| t.kind != SyntaxKind::EOF && !t.kind.is_trivia())
            .map(|t| TokenData { kind: t.kind, text_range: (t.offset, t.offset + t.len()) })
            .collect();
        match ext.extract(&toks, src) {
            Some((v, _)) => format!("{v:?}"),
            None => "None".into(),
        }
    }

    /// A scalar's value is its SOURCE TEXT, and which productions get that
    /// treatment is DATA — the `%capture` column of `stdlib/scalars/types.st`.
    ///
    /// This was a hardcoded list of seven names in `wrap_reifier`, which made it
    /// the seventh hand-synced scalar list in the codebase and quietly broke
    /// FEAT-168's whole promise: a scalar added purely as stdlib data got no
    /// flattening, so its value reified to a RECORD of the grammar's internal
    /// parts (`{n: 96, u: "dpi"}`) where every consumer expects `"96dpi"`.
    ///
    /// The failure is silent — the type resolves, the page compiles, and the
    /// wrong shape only surfaces downstream as `NaN` or a colour-parse error.
    #[test]
    fn a_scalar_named_only_in_stdlib_still_flattens_to_its_source_text() {
        for (ct, src, want) in [
            ("length", "8px", "8px"),
            ("duration", "600ms", "600ms"),
            ("color", "#e8eef7", "#e8eef7"),
            ("percentage", "50%", "50%"),
        ] {
            assert_eq!(
                shape(ct, src),
                format!("String({want:?})"),
                "{ct} must flatten to its source text, not a record of grammar parts"
            );
        }
    }

    /// The join is the TABLE, not the name: every `%capture` a scalar row points
    /// at must flatten, and nothing decides that by matching on a string literal
    /// in Rust.
    #[test]
    fn every_capture_named_by_the_scalar_table_is_a_flattening_production() {
        let (reg, _) = crate::compiler::cached_stdlib_registry();
        let captures: Vec<String> = reg
            .scalar_types()
            .filter(|s| !s.capture.is_empty())
            .map(|s| s.capture.clone())
            .collect();
        assert!(
            captures.len() >= 3,
            "the scalar table should name several captures, got {captures:?}"
        );
        for c in captures {
            assert!(
                super::is_scalar_capture(&c),
                "`{c}` is named by a %scalar_type row but is not treated as a scalar \
                 production — the table is not the source of truth"
            );
        }
    }
}
