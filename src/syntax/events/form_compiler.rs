//! FormCompiler — Layer 2 of the MatchSink architecture.
//!
//! Compiles a `FormClause` pattern into a `CompiledForm` that can match against
//! collected `NodeContext` data. The compiled form walks the FormClause pattern
//! alongside token data, dispatching to CaptureExtractors (Layer 1) for each
//! capture element.
//!
//! # Architecture
//!
//! ```text
//! FormClause + ExtractorRegistry → CompiledForm
//!   CompiledForm.try_match(NodeContext, source) → Result<FormMatch, MatchFailure>
//! ```
//!
//! During INIT-033, this is a direct interpreter. The FormClause pattern is
//! walked alongside tokens at match time. In INIT-034, complex extractors
//! may use chumsky combinators for structured captures.

use std::collections::HashMap;

use crate::parser::SourceSpan;
use crate::parser::meta_ast::{
    CaptureModifier, CaptureType, FormClause, FormInlineElement, FormParam, MacroScope,
    ParamDefault,
};
use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::{CapturedValue, ExportDecl, FormMatch};
use crate::syntax::registry::SyntaxRegistry;

use super::extractors::{ExtractorRegistry, TokenData};

/// `element_index` for a failure that occurs at the BODY stage of a form —
/// after every inline element, param, and post-arglist element has matched.
///
/// The failure-ranking heuristic in MatchSink prefers the form that consumed
/// MORE of its pattern before failing (`element_index > current`): reaching
/// the body is the furthest a form can go, so a body-rejection is the most
/// committed, most informative failure. But body-stage failures used to be
/// reported with `element_index: 0` — the LEAST specific position — so a
/// sibling whose head failed later (e.g. the colon form's `:` LiteralMismatch
/// at element 2, or the retired dispatcher's `$name:ident(` CaptureTypeMismatch
/// at element 1) outranked the body-rejection and, being a LiteralMismatch,
/// suppressed the committed-then-failed report (BUG-271: `@on $sig.change
/// { $ping(); }` compiled green and emitted NOTHING).

/// A body-stage failure is the furthest a form can reach, so it carries the
/// largest possible `element_index` so the ranking treats it as most-specific.
const BODY_ELEMENT_INDEX: usize = usize::MAX;

// =============================================================================
// CompiledForm
// =============================================================================

/// A compiled form ready for matching against token data.
#[derive(Debug)]
pub struct CompiledForm {
    /// The macro name (e.g., "data-fetch", "on-event")
    pub macro_name: String,
    /// The public API name from `%creates` (e.g., "on" from `%creates @on`).
    /// Falls back to `macro_name` when `%creates` is not set.
    pub creates_name: String,
    /// The original FormClause pattern
    pub form: FormClause,
    /// Specificity score for disambiguation (higher = more specific)
    pub specificity: u32,
    /// Scope restrictions from the macro's `%scope` declarations.
    /// Empty means allowed everywhere (backward compat).
    pub scopes: Vec<MacroScope>,
    /// The macro registers into category `form` (`@form <kind> --name`). Such a
    /// macro's inline grammar is a CONTRACT — leftover tokens after it are
    /// rejected (step 3.6) — because the form registry it feeds must be
    /// authoritative. Every other macro keeps historical trailing leniency.
    pub registers_form: bool,
    /// The macro's declared `%drops` — args it deliberately accepts and
    /// discards, carried as DATA rather than inferred from token positions.
    ///
    /// A retired shape kept alive for migration (`@value-change count(...)`,
    /// whose leading bareword the on-cutover rewrite drops via `%drops ( _ )`)
    /// must keep matching; a CURRENT macro handed a bareword it never declared
    /// (`@object torusknot(...)` — GH-14) must REFUSE rather than silently fall
    /// back to every default. Both look identical to the parser — "extra tokens
    /// before the arg list" — and only the declaration tells them apart.
    pub drops: Vec<String>,
}

impl CompiledForm {
    /// Try to match this form against a node context.
    ///
    /// Returns `Ok(FormMatch)` on success, `Err(MatchFailure)` with diagnostic
    /// context on failure.
    /// Whether this form has any consumer for a `{ … }` body block — a
    /// `body_capture`, `body_params`, or `body_groups`. Forms WITHOUT a body
    /// consumer (paren-args / inline-only, e.g. the 3D `@light(type, …)`) match a
    /// brace-body invocation by IGNORING the body; the sibling-overload resolver in
    /// MatchSink uses this to prefer a body-consuming sibling (the responsive
    /// `light-mode` `@light { $styles }`) when both forms match the same node.
    pub fn consumes_body(&self) -> bool {
        self.form.body_capture.is_some()
            || !self.form.body_params.is_empty()
            || !self.form.body_groups.is_empty()
    }

    pub fn try_match(
        &self,
        ctx: &NodeContext,
        source: &str,
        extractors: &ExtractorRegistry,
    ) -> Result<(FormMatch, bool), MatchFailure> {
        let mut captures: HashMap<String, CapturedValue> = HashMap::new();
        // BUG-264: whether this match succeeded only by IGNORING input it could not
        // bind (a clause a param/body group dropped, a tail it skipped). A lenient
        // match is NOT clean: it must rank BELOW a matching error sibling, so a
        // near-miss spelling is still refused loudly instead of silently accepted.
        let mut lenient = false;
        let mut capture_spans: HashMap<String, SourceSpan> = HashMap::new();

        // A directive written `@assert (cond) "msg"` is parsed by the CST with the
        // `(cond)` captured as an ARG_LIST structural child, leaving only `"msg"` in
        // inline_tokens. When the form has INLINE captures but declares NO params
        // (e.g. `@assert $condition:expr $message:string?`), that ARG_LIST belongs to
        // the inline expression, not a parameter list. Splice its tokens (parens
        // included) back to the front of the inline stream so the inline `:expr`
        // capture consumes `(cond)` and the trailing `"msg"` binds to `$message`.
        // Bare `@assert cond "msg"` (no parens, no ARG_LIST) is unaffected.
        let spliced_tokens: Vec<TokenData>;
        let inline_source: &[TokenData] =
            if self.form.params.is_empty() && !self.form.inline_elements.is_empty() {
                {
                    // Order ALL ARG_LIST tokens before the post-arglist inline
                    // tokens by span. There may be SEVERAL arg-lists (BUG-246:
                    // `@on &.visible(threshold: 0.2) --rise(distance: 8px);`
                    // parses as two ARG_LIST children — the driver's params
                    // and the form's call args) — splicing only the first
                    // silently dropped the second.
                    let arg_tokens: Vec<TokenData> = ctx
                        .children
                        .iter()
                        .filter(|c| c.kind == SyntaxKind::ARG_LIST)
                        .flat_map(|c| c.tokens.iter().cloned())
                        .collect();
                    if !arg_tokens.is_empty() {
                        let mut merged: Vec<TokenData> = arg_tokens;
                        merged.extend(ctx.inline_tokens.iter().cloned());
                        merged.sort_by_key(|t| t.text_range.0);
                        spliced_tokens = merged;
                        &spliced_tokens
                    } else {
                        &ctx.inline_tokens
                    }
                }
            } else {
                &ctx.inline_tokens
            };
        let mut cursor = TokenCursor::new(inline_source);

        // 1. Match the directive name prefix
        //    The directive_name is like "@on" or "$" or "&"
        //    The tokens should start with the prefix tokens (AT_SIGN, IDENT "on", etc.)
        //    Returns any namespace qualifier (`scene` in `@scene/camera`) so it
        //    survives into the FormMatch for dispatch-time resolution (FUP-055).
        let namespace_qualifier = self.match_directive_prefix(&mut cursor, source)?;

        // 2. Match inline elements (captures and literals)
        let num_inline = self.form.inline_elements.len();
        for (i, element) in self.form.inline_elements.iter().enumerate() {
            // Trailing ";" in form patterns is a syntactic terminator already handled
            // by the grammar parser. If the cursor has no more non-trivia tokens and
            // the remaining element is just Literal(";"), skip it.
            if let FormInlineElement::Literal(s) = element
                && s == ";"
                && i == num_inline - 1
                && cursor.remaining_non_trivia() == 0
            {
                break;
            }
            self.match_inline_element(
                element,
                &mut cursor,
                source,
                extractors,
                &mut captures,
                &mut capture_spans,
                i,
            )
            .map_err(|failure| {
                // A trailing `;` literal is the statement's COMMITMENT point:
                // the whole inline grammar matched, so content REMAINING
                // before it (`@on &.visible --a --b;`) is extra input, not a
                // different overload — LiteralMismatch would be suppressed
                // (W3 review P2: the two-form surface vanished silently).
                if i == num_inline - 1
                    && matches!(element, FormInlineElement::Literal(s) if s == ";")
                    && matches!(failure.kind, FailureKind::LiteralMismatch { .. })
                    && cursor.remaining_non_trivia() > 0
                {
                    MatchFailure {
                        form_name: failure.form_name,
                        element_index: failure.element_index,
                        kind: FailureKind::ExtraInlineTokens {
                            remaining: cursor.remaining_non_trivia(),
                        },
                    }
                } else {
                    failure
                }
            })?;
        }

        // 2b. Reject if unconsumed inline tokens appear BEFORE the arg list.
        //     This prevents `&counter &psr-counter()` from matching the
        //     `template-invoke-bare` form as name="counter" — the extra
        //     `& psr - counter` tokens between `$name:ident` and `(` are
        //     a structural mismatch. Tokens AFTER the arg list (e.g.,
        //     `: string` in `@fn name(params): string { body }`) are fine.
        //
        //     A form with NO inline elements can still be handed a leading
        //     bareword, and two very different things look identical there:
        //
        //       @value-change count(duration: 500ms)   ← retired shape, kept
        //         alive for migration; the on-cutover rewrite drops the
        //         positional via `%drops ( _ )`. Must keep matching.
        //       @object torusknot(size: 1.2)           ← GH-14: a CURRENT
        //         macro handed a bareword it never declared. Silently matched
        //         with `shape` left at its default, emitting a plausible
        //         icosahedron — the worst failure, because it looks like it
        //         worked.
        //
        //     Tolerance therefore reads the macro's DECLARED `%drops` (`_` =
        //     positional extras are dropped) instead of guessing from
        //     `num_inline > 0`. Undeclared extras refuse, so a retired shape
        //     migrates and a live typo is diagnosed — one rule, both cases,
        //     no per-macro special case in Rust.
        let drops_positional = self.drops.iter().any(|d| d == "_");
        if (num_inline > 0 || !drops_positional)
            && !self.form.params.is_empty()
            && cursor.remaining_non_trivia() > 0
            && let Some(arg_node) = ctx.child_by_kind(SyntaxKind::ARG_LIST)
        {
            let arg_start = arg_node.span_start;
            let has_tokens_before_args = cursor
                .remaining()
                .iter()
                .any(|t| !t.kind.is_trivia() && t.text_range.0 < arg_start);
            if has_tokens_before_args {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: num_inline,
                    kind: FailureKind::ExtraInlineTokens {
                        remaining: cursor.remaining_non_trivia(),
                    },
                });
            }
        }

        // 3. Match parenthesized params (from ARG_LIST child node)
        if !self.form.params.is_empty() {
            if let Some(arg_node) = ctx.child_by_kind(SyntaxKind::ARG_LIST) {
                let mut arg_cursor = TokenCursor::new(&arg_node.tokens);
                // Skip opening paren
                arg_cursor.skip_kind(SyntaxKind::L_PAREN);

                self.match_params(
                    &self.form.params,
                    &mut arg_cursor,
                    source,
                    extractors,
                    &mut captures,
                    &mut capture_spans,
                    &mut lenient,
                )?;
            } else {
                // Check if all params can be skipped (optional or have defaults)
                let all_skippable = self.form.params.iter().all(|p| {
                    // A param is skippable if it has a default value at the param level
                    if p.default.is_some() {
                        return true;
                    }
                    // Or if all its captures are optional or have inline defaults.
                    // Note: ZeroOrMore ($x:type*) is NOT skippable — the arg list
                    // (parentheses) must still be present even for zero arguments.
                    // This distinguishes `&name` from `&name()`.
                    p.elements.iter().all(|e| match e {
                        FormInlineElement::Capture(cap, default) => {
                            cap.modifier == CaptureModifier::Optional || default.is_some()
                        }
                        _ => true, // Literals in params are structural, skip if no arg list
                    })
                });
                if !all_skippable {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index: 0,
                        kind: FailureKind::MissingArgList,
                    });
                }
                // Apply defaults for missing params
                self.apply_param_defaults(&self.form.params, &mut captures);
            }
        }

        // 3.5 Match post-arglist inline elements (the `): T` run; PLAN-025 / BUG-045).
        //    ARG_LIST is a structural child (not flattened into inline_tokens), so the
        //    main `cursor` — already past the pre-arglist inline elements from step 2 —
        //    is now positioned at the tokens BETWEEN `)` and `{` (the grammar's second
        //    inline_args pass wrapped them in ARG nodes that ARE flattened in).
        //
        //    The whole post-arglist run is INHERENTLY OPTIONAL: a form written
        //    `@fn n(p): T { … }` and an invocation `@fn n(p) { … }` (no return type) must
        //    BOTH match the one form. So if the cursor has no more non-trivia tokens
        //    (invocation went straight from `)` to `{`), skip the run entirely. Only
        //    when post-arglist tokens ARE present do we match each element (then a
        //    genuine mismatch is a real failure). Runs whether or not params matched.
        if cursor.remaining_non_trivia() > 0 {
            for (i, element) in self.form.post_arg_inline.iter().enumerate() {
                self.match_inline_element(
                    element,
                    &mut cursor,
                    source,
                    extractors,
                    &mut captures,
                    &mut capture_spans,
                    i,
                )?;
            }
        }

        // 3.6 A FORM-DECLARING macro's inline grammar is a CONTRACT: nothing may
        //     remain after it. Without this, `@form motion --x junk { … }`
        //     matched — `motion` and `--x` consumed, `junk` IGNORED — and the
        //     %registers clause recorded a form the author never declared into
        //     the registry splices are validated against. Scoped to macros that
        //     register into category `form` because the FORM REGISTRY must be
        //     authoritative (a junk-declared entry is a landmine there); every
        //     other macro keeps the historical trailing-token leniency, whose
        //     general tightening needs its own measured wave (stdlib grammars
        //     like `@preset`/`@loop` currently RELY on it — 20 failing tests
        //     when enforced globally).
        // …and for a BODY-LESS form generally (the @on statement shape — a
        // body-less form claims the WHOLE statement, so a leftover is always
        // suspect: `@on &.visible --a --b;` compiled with `--b` ignored, W3
        // review P2). Body-bearing forms keep historical leniency — and so
        // do invocations with an ARG_LIST: extra tokens BEFORE the parens
        // are a legitimate surface (`@value-change count(duration: 500ms)`
        // names its watcher — 2b's own comment), and the arg list is what
        // follows the inline grammar there. Strict only when nothing
        // legitimate can follow.
        if (self.registers_form || self.form.body_capture.is_none())
            && ctx.child_by_kind(SyntaxKind::ARG_LIST).is_none()
            && cursor.remaining_non_trivia() > 0
        {
            // …but a lone trailing `;` is the STATEMENT's terminator, not
            // content: body-less forms whose grammar has no `;` literal
            // (`@import "./faq.st";`) legitimately end with one.
            let remaining: Vec<_> = cursor.remaining().iter().collect();
            let only_terminator = {
                let non_trivia: Vec<_> = remaining.iter().filter(|t| !t.kind.is_trivia()).collect();
                non_trivia.len() == 1 && non_trivia[0].kind == SyntaxKind::SEMICOLON
            };
            if only_terminator {
                // fall through to body matching
            } else {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: self.form.post_arg_inline.len(),
                    kind: FailureKind::ExtraInlineTokens {
                        remaining: cursor.remaining_non_trivia(),
                    },
                });
            }
        } else if (self.registers_form || self.form.body_capture.is_none())
            && ctx.child_by_kind(SyntaxKind::ARG_LIST).is_some()
            && Self::has_dropped_tokens(&cursor, source)
        {
            // A lone trailing `;` is the STATEMENT's terminator (as in the hard
            // arm above), never a dropped clause — a body-less ARG_LIST form
            // ending `…;` is a clean match.
            let remaining: Vec<_> = cursor.remaining().iter().collect();
            let only_terminator = {
                let non_trivia: Vec<_> =
                    remaining.iter().filter(|t| !t.kind.is_trivia()).collect();
                non_trivia.len() == 1 && non_trivia[0].kind == SyntaxKind::SEMICOLON
            };
            if !only_terminator {
                // BUG-264 class 3 (the presence matcher gap): a body-less form
                // WITH an ARG_LIST can still drop post-inline trailing tokens.
                // ARG_LIST is a structural child, so the hard-error arm above
                // (which requires NO arg list) cannot fire here; without this
                // the leftover would be silently tolerated — a lenient match an
                // error sibling could never outrank (`@presence(room: …) as
                // viewers: 5` dropped the `: 5`). Mark it lenient, exactly like
                // an arg-list leftover.
                lenient = true;
            }
        }

        // 4. Match body params (from BODY child node)
        //    Separate regular body_params (CSS property-like) from pseudo-selectors.
        //    Regular params go through cursor-based matching; pseudo-selectors go through
        //    SCOPE_BLOCK child matching in step 4b.
        let has_regular_body_params = self.form.body_params.iter().any(|p| {
            p.elements
                .iter()
                .any(|e| !matches!(e, FormInlineElement::PseudoSelector { .. }))
        });
        let has_pseudo_selectors = self.form.body_params.iter().any(|p| {
            p.elements
                .iter()
                .any(|e| matches!(e, FormInlineElement::PseudoSelector { .. }))
        });

        if has_regular_body_params {
            if let Some(body_node) = ctx.child_by_kind(SyntaxKind::BODY) {
                // Only pass non-pseudo-selector params to cursor-based matching
                let regular_params: Vec<_> = self
                    .form
                    .body_params
                    .iter()
                    .filter(|p| {
                        p.elements
                            .iter()
                            .any(|e| !matches!(e, FormInlineElement::PseudoSelector { .. }))
                    })
                    .cloned()
                    .collect();
                self.match_body_params(
                    &regular_params,
                    body_node,
                    source,
                    extractors,
                    &mut captures,
                    &mut capture_spans,
                    &mut lenient,
                )?;
            } else {
                // Body required but not present — check if all regular params are optional
                let all_optional = self.form.body_params.iter().all(|p| {
                    p.elements.iter().all(|e| match e {
                        FormInlineElement::Capture(cap, _) => {
                            cap.modifier == CaptureModifier::Optional
                                || cap.modifier == CaptureModifier::ZeroOrMore
                        }
                        FormInlineElement::PseudoSelector { modifier, .. } => {
                            *modifier == CaptureModifier::Optional
                                || *modifier == CaptureModifier::ZeroOrMore
                        }
                        _ => false,
                    })
                });
                if !all_optional {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index: BODY_ELEMENT_INDEX,
                        kind: FailureKind::MissingBody,
                    });
                }
            }
        }

        // 4b. Match pseudo-selector captures from body SCOPE_BLOCK children
        if has_pseudo_selectors && let Some(body_node) = ctx.child_by_kind(SyntaxKind::BODY) {
            self.match_pseudo_selectors(
                body_node,
                source,
                extractors,
                &mut captures,
                &mut capture_spans,
            )?;
        }
        // If no BODY and pseudo-selectors are Optional, match_pseudo_selectors
        // would handle the error — but if there's no BODY, there are no SCOPE_BLOCK
        // children to search, so Optional ones just don't get captured (correct).

        // 4c. Match leading body GROUPS (FEAT-103). A `( ... )modifier` run at the
        //     head of the body region matches the BODY token prefix via the SAME PEG
        //     that powers %capture_type (compile_pattern). On success it binds the
        //     group's inner captures and yields a BYTE boundary; the remaining body
        //     run is then handed to the body capture (step 5) starting at that
        //     boundary. This lets a form express e.g. `( shortcut: $s:string ; )?`
        //     ahead of a greedy `$body:component_body` — the two no longer fight over
        //     the head bytes. Empty body_groups → this step is a no-op (legacy path).
        let mut body_inner_start: Option<usize> = None;
        if !self.form.body_groups.is_empty()
            && let Some(body_node) = ctx.child_by_kind(SyntaxKind::BODY)
        {
            body_inner_start =
                self.match_body_groups(body_node, source, extractors, &mut captures)?;
        }

        // 5. Handle body_capture (body captures — may contain multiple $name:type specs)
        if let Some(capture_spec) = &self.form.body_capture {
            if let Some(body_node) = ctx.child_by_kind(SyntaxKind::BODY) {
                let r = self.extract_body_captures(
                    capture_spec,
                    body_node,
                    source,
                    extractors,
                    &mut captures,
                    body_inner_start,
                );
                r?;
            } else if self.form.body_params.is_empty() && body_capture_has_required(capture_spec) {
                // body_params is empty (step 4 skipped), but body_capture
                // has required captures — BODY must be present.
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: BODY_ELEMENT_INDEX,
                    kind: FailureKind::MissingBody,
                });
            }
        }

        Ok((FormMatch {
            macro_name: self.creates_name.clone(),
            namespace_qualifier,
            doc: None,
            // Preserve the exact %macro parse selected (specificity-ranked, literal-aware)
            // so resolve/evaluate honor it instead of re-guessing from captures alone.
            matched_macro: Some(self.macro_name.clone()),
            // DESIGN: creates_name is the public API name derived from the directive form.
            // For @-prefixed forms, this is the directive name (e.g., "data" from @data).
            // For $ and & forms, this is the internal macro name (e.g., "local-state").
            captures,
            capture_spans,
            selector: None,
            // GH-27: `span_start` is the offset when the node OPENED — which for a
            // directive following a previous one is the preceding newline/trivia,
            // not the directive's own `@`. Trim leading trivia so consecutive
            // directives anchor at themselves (the second `@data` was reported at
            // col 27 of the previous line).
            span: SourceSpan::new(skip_leading_trivia(source, ctx.span_start), ctx.span_end),
            source_file: None,
        }, lenient))
    }

    /// Match the directive prefix tokens.
    ///
    /// For "@on", expects AT_SIGN then IDENT "on".
    /// For "$", expects DOLLAR.
    /// For "&", expects AMPERSAND.
    fn match_directive_prefix(
        &self,
        cursor: &mut TokenCursor,
        source: &str,
    ) -> Result<Vec<String>, MatchFailure> {
        let directive = &self.form.directive_name;
        let mut namespace_qualifier: Vec<String> = Vec::new();

        // Skip leading whitespace in cursor
        cursor.skip_trivia();

        if let Some(name_part) = directive.strip_prefix('@') {
            // Expect AT_SIGN
            if !cursor.eat_kind(SyntaxKind::AT_SIGN) {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: 0,
                    kind: FailureKind::DirectivePrefixMismatch {
                        expected: directive.clone(),
                    },
                });
            }
            // FEAT-118: a namespace-qualified invocation `@scene/camera` (or
            // aliased `@s/camera`) still matches the unqualified `@camera`
            // `%form` — skip the leading `qualifier/` prefix so the leaf name
            // matches. The qualifier is RETURNED (FUP-055) so dispatch can
            // resolve it against the file's ImportScope rather than discarding it.
            namespace_qualifier = cursor.skip_namespace_qualifier(source);
            // The rest of the directive name (e.g., "on" from "@on", "data" from "@data")
            if !name_part.is_empty() {
                // May be space-separated: "@on hover" means the directive name includes the word
                let parts: Vec<&str> = name_part.split_whitespace().collect();
                for part in parts {
                    cursor.skip_trivia();
                    if !cursor.eat_text(source, part) {
                        return Err(MatchFailure {
                            form_name: self.macro_name.clone(),
                            element_index: 0,
                            kind: FailureKind::DirectivePrefixMismatch {
                                expected: directive.clone(),
                            },
                        });
                    }
                }
            }
        } else if directive.starts_with('$') {
            if !cursor.eat_kind(SyntaxKind::DOLLAR) {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: 0,
                    kind: FailureKind::DirectivePrefixMismatch {
                        expected: directive.clone(),
                    },
                });
            }
        } else if directive.starts_with('&') {
            if !cursor.eat_kind(SyntaxKind::AMPERSAND) {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: 0,
                    kind: FailureKind::DirectivePrefixMismatch {
                        expected: directive.clone(),
                    },
                });
            }
        } else if directive.starts_with('~') {
            if !cursor.eat_kind(SyntaxKind::TILDE) {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: 0,
                    kind: FailureKind::DirectivePrefixMismatch {
                        expected: directive.clone(),
                    },
                });
            }
        } else if let Some(name_part) = directive.strip_prefix('%') {
            if !cursor.eat_kind(SyntaxKind::PERCENT) {
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: 0,
                    kind: FailureKind::DirectivePrefixMismatch {
                        expected: directive.clone(),
                    },
                });
            }
            // Match the meta-directive name
            if !name_part.is_empty() {
                cursor.skip_trivia();
                if !cursor.eat_text(source, name_part) {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index: 0,
                        kind: FailureKind::DirectivePrefixMismatch {
                            expected: directive.clone(),
                        },
                    });
                }
            }
        }

        Ok(namespace_qualifier)
    }

    /// Match a single FormInlineElement against the token cursor.
    fn match_inline_element(
        &self,
        element: &FormInlineElement,
        cursor: &mut TokenCursor,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        capture_spans: &mut HashMap<String, SourceSpan>,
        element_index: usize,
    ) -> Result<(), MatchFailure> {
        cursor.skip_trivia();

        match element {
            FormInlineElement::Literal(text) => {
                if !cursor.eat_text(source, text) {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index,
                        kind: FailureKind::LiteralMismatch {
                            expected: text.clone(),
                            got: cursor.peek_text(source).map(String::from),
                        },
                    });
                }
            }

            FormInlineElement::Capture(cap, default) => {
                self.match_capture(
                    &cap.var_name,
                    &cap.capture_type,
                    cap.modifier,
                    default.as_ref(),
                    cursor,
                    source,
                    extractors,
                    captures,
                    capture_spans,
                    element_index,
                )?;

                // Handle alias capture: `$source:binding as $item:ident`
                // The alias is stored in cap.alias_capture and extracted after
                // the "as" keyword.
                if let Some(alias) = &cap.alias_capture {
                    cursor.skip_trivia();
                    // Skip the "as" keyword
                    if cursor.peek_text(source) == Some("as") {
                        cursor.advance(1);
                        cursor.skip_trivia();
                    }
                    self.match_capture(
                        &alias.var_name,
                        &alias.capture_type,
                        alias.modifier,
                        None,
                        cursor,
                        source,
                        extractors,
                        captures,
                        capture_spans,
                        element_index,
                    )?;
                }
            }

            FormInlineElement::Group { elements, modifier } => {
                // An inline GROUP tried as a unit. For OPTIONAL, a failure to
                // match the group's FIRST element (or any later element) rolls
                // back the cursor and skips the whole clause — a genuinely-
                // optional trailing clause (`( timeout : $t:time = 5000 )?`)
                // is absent, so the group contributes nothing. Captures the
                // partial group added are removed on rollback. Required / +
                // groups propagate the failing element's error.
                //
                // A REPEATED group (`( "->" $next:score_step )*` / `+`) LOOPS:
                // the group's elements are matched over and over until they stop
                // matching, and a capture that fires on more than one iteration
                // AGGREGATES into an Array (score_line's `$next` collects every
                // clip after the first). Before this, a repeated inline group
                // matched exactly once and each iteration overwrote the capture,
                // so `&a -> &b -> &c` silently kept only `&a` and dropped the
                // arrow chain entirely (the golden §9 sequencing gap).
                let repeated = matches!(
                    modifier,
                    CaptureModifier::ZeroOrMore | CaptureModifier::OneOrMore
                );
                let mut iterations = 0usize;
                loop {
                    let iter_start = cursor.clone();
                    let pre_keys: Vec<String> = captures.keys().cloned().collect();
                    let mut group_ok = true;
                    // Snapshot captures present before THIS iteration so we can
                    // detect which vars the iteration produced (to aggregate).
                    let before: std::collections::HashMap<String, CapturedValue> = captures
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for elem in elements {
                        cursor.skip_trivia();
                        if let Err(f) = self.match_inline_element(
                            elem,
                            cursor,
                            source,
                            extractors,
                            captures,
                            capture_spans,
                            element_index,
                        ) {
                            if *modifier == CaptureModifier::Optional
                                || *modifier == CaptureModifier::ZeroOrMore
                                || (repeated && iterations > 0)
                            {
                                // Roll back this iteration and stop. On the
                                // FIRST iteration of an optional/zero-or-more
                                // group this means "absent"; on a LATER iteration
                                // it means "the repetition ended here".
                                *cursor = iter_start;
                                captures.retain(|k, _| pre_keys.contains(k));
                                if iterations == 0 {
                                    for el in elements {
                                        if let FormInlineElement::Capture(cap, Some(def)) = el {
                                            captures
                                                .entry(cap.var_name.clone())
                                                .or_insert_with(|| CapturedValue::from_default(def));
                                        }
                                    }
                                }
                                group_ok = false;
                                break;
                            }
                            // Required / OneOrMore first iteration: a partial
                            // group match is a genuine failure.
                            return Err(f);
                        }
                    }
                    if !group_ok {
                        break;
                    }
                    // Aggregate: any capture this iteration produced or changed
                    // that fires again on a later iteration folds into an Array.
                    if repeated {
                        for el in elements {
                            collect_group_capture_vars(el, &mut |var| {
                                if let Some(new_val) = captures.get(var).cloned() {
                                    match before.get(var) {
                                        Some(CapturedValue::Array(prev)) => {
                                            let mut arr = prev.clone();
                                            arr.push(new_val);
                                            captures.insert(
                                                var.to_string(),
                                                CapturedValue::Array(arr),
                                            );
                                        }
                                        Some(prev) if iterations > 0 => {
                                            captures.insert(
                                                var.to_string(),
                                                CapturedValue::Array(vec![
                                                    prev.clone(),
                                                    new_val,
                                                ]),
                                            );
                                        }
                                        _ => {}
                                    }
                                }
                            });
                        }
                    }
                    iterations += 1;
                    if !repeated {
                        break;
                    }
                    // Guard against a zero-width match looping forever.
                    if cursor.remaining().is_empty() {
                        break;
                    }
                }
            }

            FormInlineElement::Comparison { operator, capture } => {
                // Match the operator first
                if !cursor.eat_text(source, operator) {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index,
                        kind: FailureKind::LiteralMismatch {
                            expected: operator.clone(),
                            got: cursor.peek_text(source).map(String::from),
                        },
                    });
                }
                cursor.skip_trivia();
                // Then extract the capture
                self.match_capture(
                    &capture.var_name,
                    &capture.capture_type,
                    capture.modifier,
                    None,
                    cursor,
                    source,
                    extractors,
                    captures,
                    capture_spans,
                    element_index,
                )?;
            }

            FormInlineElement::KeywordBlock {
                keyword,
                body_params,
                modifier,
            } => {
                // Look for a child node matching this keyword
                if let Some(child) = ctx_child_by_keyword(cursor, source, keyword) {
                    let mut child_cursor = TokenCursor::new(&child);
                    for param in body_params {
                        self.match_param(
                            param,
                            &mut child_cursor,
                            source,
                            extractors,
                            captures,
                            capture_spans,
                        )?;
                    }
                } else if *modifier != CaptureModifier::Optional
                    && *modifier != CaptureModifier::ZeroOrMore
                {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index,
                        kind: FailureKind::MissingKeywordBlock {
                            keyword: keyword.clone(),
                        },
                    });
                }
            }

            FormInlineElement::PseudoSelector { .. } => {
                // PseudoSelectors in body_params are handled by match_pseudo_selectors
                // (step 4b in try_match) which searches SCOPE_BLOCK children.
                // This stub is a no-op because pseudo-selector tokens are structural
                // and don't appear in the flat body cursor.
            }

            FormInlineElement::PseudoClass { .. } => {
                // Similar to PseudoSelector — handled at body level
                // During POC, pseudo-class matching is deferred to INIT-034
            }
        }

        Ok(())
    }

    /// Match a single capture against the token cursor.
    fn match_capture(
        &self,
        var_name: &str,
        capture_type: &CaptureType,
        modifier: CaptureModifier,
        default: Option<&ParamDefault>,
        cursor: &mut TokenCursor,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        capture_spans: &mut HashMap<String, SourceSpan>,
        element_index: usize,
    ) -> Result<(), MatchFailure> {
        // Union types are parameterized, so they can't be pre-registered in the
        // ExtractorRegistry. Create a temporary UnionExtractor on-the-fly.
        // Inline-constructed extractors for parameterized terminals that can't be
        // pre-registered in the static ExtractorRegistry (Union, Balanced). Held here so
        // the borrowed `&dyn` reference outlives the match arms below.
        let union_ext_holder;
        let balanced_ext_holder;
        let skip_block_ext_holder;
        let extractor: Option<&dyn super::extractors::CaptureExtractor> = match capture_type {
            CaptureType::Union(variants) => {
                union_ext_holder = super::extractors::pattern::UnionExtractor {
                    variants: variants.clone(),
                };
                Some(&union_ext_holder)
            }
            CaptureType::Balanced(delim) => {
                balanced_ext_holder =
                    super::extractors::custom::BalancedExtractor { delim: *delim };
                Some(&balanced_ext_holder)
            }
            CaptureType::SkipBlock => {
                skip_block_ext_holder = super::extractors::custom::SkipBlockExtractor;
                Some(&skip_block_ext_holder)
            }
            // Custom (stdlib %capture_type) productions resolve from the compiled custom map
            // (PLAN-023 W2). When a name has no registered %capture_type (e.g. ad-hoc form
            // type words like `array` that historically defaulted to Expr), fall back to the
            // Expr extractor so those forms keep their prior lenient behavior.
            CaptureType::Custom(name) => extractors
                .custom
                .get(name)
                .map(|b| b.as_ref())
                .or_else(|| extractors.get(&CaptureType::Expr)),
            _ => extractors.get(capture_type),
        };

        // Union types are parameterized (CaptureType::Union(variants)), so they
        // can't be pre-registered in the static ExtractorRegistry HashMap.
        // Construct a UnionExtractor inline when needed.
        let union_extractor;
        let extractor: Option<&dyn super::extractors::CaptureExtractor> = if extractor.is_some() {
            extractor
        } else if let CaptureType::Union(variants) = capture_type {
            union_extractor = super::extractors::pattern::UnionExtractor {
                variants: variants.clone(),
            };
            Some(&union_extractor)
        } else {
            None
        };

        // Greedy extractors (selector, expr) must STOP at the next literal/keyword
        // the form expects after this capture. Without this, `@then $target:selector
        // should …` lets the selector swallow `should exit` whole, leaving the
        // `should` literal unmatched (the @then/@when 0-match bug). Compute the
        // boundary = first token whose text equals the next inline Literal, and
        // restrict the slice handed to the extractor to tokens before it.
        // Boundary kinds: a Literal stops the extractor at that keyword's text; a
        // following String capture (e.g. `$message:string?` after `$condition:expr`)
        // stops it at the first STRING token so `(cond) "msg"` splits correctly.
        enum NextBoundary<'a> {
            None,
            Literal(&'a str),
            StringCapture,
            // An `:expr` capture follows another `:expr` capture (e.g. `@assert
            // $condition:expr $message:expr?`). Both are greedy, so without a
            // boundary the FIRST swallows the second and the form fails to match
            // — which, for an optional trailing capture, silently drops the whole
            // directive from the emitted bundle (BUG-216). Two adjacent expressions
            // are only separable if the second is parenthesized, so the boundary is
            // the first top-level `(` that begins a NEW expression: one preceded by
            // whitespace and not itself the start of the first expression.
            ParenExprCapture,
            // The current capture is a SELECTOR and another capture follows (e.g.
            // `@when $target:selector $action:ident …`). A selector is a single
            // contiguous token-run, so it must stop at the first whitespace so the
            // following capture (`click`) is not swallowed. (PLAN-027 W3 — @when.)
            WhitespaceAfterSelector,
            // The last inline capture before a `{ … }` body must stop at the body's
            // opening brace — without this a greedy `:expr` capture (e.g. the signal
            // path in `@when &self.$sig:expr { … }`) swallows the whole body, since
            // ExprExtractor treats `{` as an opening-depth token. (BUG-088.)
            BodyBrace,
        }
        let next_elem = self.form.inline_elements.get(element_index + 1);
        let is_last_inline = element_index + 1 == self.form.inline_elements.len();
        let next_boundary: NextBoundary = match next_elem {
            Some(FormInlineElement::Literal(s)) => NextBoundary::Literal(s.as_str()),
            Some(FormInlineElement::Capture(cap, _))
                if matches!(cap.capture_type, CaptureType::String) =>
            {
                NextBoundary::StringCapture
            }
            // An expression-or-string capture follows a greedy `:expr` (e.g.
            // `@assert $condition:expr $message:assert_message?`). See
            // `ParenExprCapture`: the condition must stop at the `(` that opens
            // the NEXT expression, or it swallows the message and the form fails
            // to match — silently dropping the directive (BUG-216).
            Some(FormInlineElement::Capture(cap, _))
                if matches!(capture_type, CaptureType::Expr)
                    && matches!(
                        &cap.capture_type,
                        CaptureType::Expr | CaptureType::Custom(_)
                    ) =>
            {
                NextBoundary::ParenExprCapture
            }
            Some(FormInlineElement::Capture(_, _))
                if matches!(capture_type, CaptureType::Selector) =>
            {
                NextBoundary::WhitespaceAfterSelector
            }
            // A GROUP follows: bound the current capture at the group's FIRST
            // element, so a greedy `:expr` stops at the group's leading keyword
            // instead of swallowing it (`@wait_until $condition:expr
            // ( timeout : $t:time = 5000 )?` — the expr must stop at `timeout`).
            Some(FormInlineElement::Group { elements, .. }) => match elements.first() {
                Some(FormInlineElement::Literal(s)) => NextBoundary::Literal(s.as_str()),
                _ => NextBoundary::None,
            },
            // No following inline element, but a body capture follows: bound at `{`.
            None if is_last_inline && self.form.body_capture.is_some() => NextBoundary::BodyBrace,
            _ => NextBoundary::None,
        };
        // The boundary must only fire at bracket DEPTH 0 — a keyword or string
        // *inside* parens/braces/brackets belongs to the captured expression, not
        // to the following form element. Without the depth guard, an assertion
        // like `@assert (Math.abs(x - f('#f80')) < 1)` would truncate the
        // condition at the nested `'#f80'` string (regression). (PLAN-027 W0/W2.)
        let bound = |toks: &[super::extractors::TokenData]| -> usize {
            if matches!(next_boundary, NextBoundary::None) {
                return toks.len();
            }
            let mut depth = 0i32;
            for (i, t) in toks.iter().enumerate() {
                // A top-level `{` ends the inline region when the form expects a body:
                // check BEFORE the depth bump so the brace itself is the boundary.
                if depth == 0
                    && matches!(next_boundary, NextBoundary::BodyBrace)
                    && t.kind == SyntaxKind::L_BRACE
                {
                    return i;
                }
                match t.kind {
                    SyntaxKind::L_PAREN | SyntaxKind::L_BRACE | SyntaxKind::L_BRACKET => {
                        depth += 1;
                    }
                    SyntaxKind::R_PAREN | SyntaxKind::R_BRACE | SyntaxKind::R_BRACKET => {
                        depth -= 1;
                    }
                    _ if depth == 0 => match next_boundary {
                        NextBoundary::Literal(lit)
                            if !t.kind.is_trivia() && t.text(source) == lit =>
                        {
                            return i;
                        }
                        NextBoundary::StringCapture if t.kind == SyntaxKind::STRING => {
                            return i;
                        }
                        // A top-level `(` that OPENS a new expression — i.e. one that
                        // is not the very first token of this capture (`(cond)` itself
                        // starts with a paren) and follows whitespace, so
                        // `f(x)` stays one expression while `(cond) (msg)` splits.
                        NextBoundary::ParenExprCapture
                            if t.kind == SyntaxKind::L_PAREN
                                && i > 0
                                && toks[..i].iter().any(|p| !p.kind.is_trivia())
                                && toks[i - 1].kind.is_trivia() =>
                        {
                            return i;
                        }
                        NextBoundary::WhitespaceAfterSelector if t.kind.is_trivia() => {
                            return i;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            toks.len()
        };

        match modifier {
            // `Counted` constrains a char-class run INSIDE a capture type; at the
            // form-parameter level it behaves exactly like `Required` (match once).
            CaptureModifier::Required | CaptureModifier::Counted(_) => {
                let remaining = &cursor.remaining()[..bound(cursor.remaining())];
                if remaining.is_empty() {
                    if let Some(def) = default {
                        captures.insert(var_name.to_string(), CapturedValue::from_default(def));
                        return Ok(());
                    }
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index,
                        kind: FailureKind::CaptureFailedNoTokens {
                            var_name: var_name.to_string(),
                            capture_type: format!("{:?}", capture_type),
                        },
                    });
                }

                if let Some(ext) = extractor
                    && let Some((value, consumed)) = ext.extract(remaining, source)
                {
                    // I3 / gh-37: a CUSTOM capture (e.g. `$params:param_list`)
                    // that consumes ZERO tokens from a non-empty bounded region
                    // has NOT consumed its declared extent — the paren-group
                    // content is left unexamined and the match would silently
                    // accept it with defaults filling the holes. This is the
                    // consume-or-refuse line for a custom capture: `(x number,
                    // y number)` where param_list legally matches zero items.
                    // Only structural closers (`)`, `]`, `}`) remain → clean
                    // empty (`()`); any other content → the capture mismatched.
                    let content_remains = remaining.iter().any(|t| {
                        !t.kind.is_trivia()
                            && !matches!(
                                t.kind,
                                SyntaxKind::R_PAREN
                                    | SyntaxKind::R_BRACE
                                    | SyntaxKind::R_BRACKET
                            )
                    });
                    if matches!(capture_type, CaptureType::Custom(_))
                        && consumed == 0
                        && content_remains
                    {
                        return Err(MatchFailure {
                            form_name: self.macro_name.clone(),
                            element_index,
                            kind: FailureKind::CaptureTypeMismatch {
                                var_name: var_name.to_string(),
                                expected_type: format!("{:?}", capture_type),
                                expected_alternatives: Vec::new(),
                                got_kind: remaining
                                    .iter()
                                    .find(|t| !t.kind.is_trivia())
                                    .map(|t| format!("{:?}", t.kind))
                                    .unwrap_or_else(|| "EOF".to_string()),
                            },
                        });
                    }
                    let span = cursor.span_for(consumed);
                    captures.insert(var_name.to_string(), value);
                    if let Some(s) = span {
                        capture_spans.insert(var_name.to_string(), s);
                    }
                    cursor.advance(consumed);
                    return Ok(());
                }

                if let Some(def) = default {
                    captures.insert(var_name.to_string(), CapturedValue::from_default(def));
                    return Ok(());
                }

                Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index,
                    kind: FailureKind::CaptureTypeMismatch {
                        var_name: var_name.to_string(),
                        expected_type: format!("{:?}", capture_type),
                        expected_alternatives: match capture_type {
                            CaptureType::Union(alts) => alts.clone(),
                            _ => Vec::new(),
                        },
                        got_kind: remaining
                            .first()
                            .map(|t| format!("{:?}", t.kind))
                            .unwrap_or_else(|| "EOF".to_string()),
                    },
                })
            }

            CaptureModifier::Optional => {
                let remaining = &cursor.remaining()[..bound(cursor.remaining())];
                if let Some(ext) = extractor {
                    if let Some((value, consumed)) = ext.extract(remaining, source) {
                        let span = cursor.span_for(consumed);
                        captures.insert(var_name.to_string(), value);
                        if let Some(s) = span {
                            capture_spans.insert(var_name.to_string(), s);
                        }
                        cursor.advance(consumed);
                    } else if let Some(def) = default {
                        captures.insert(var_name.to_string(), CapturedValue::from_default(def));
                    }
                }
                // Optional capture — ok if it doesn't match
                Ok(())
            }

            CaptureModifier::ZeroOrMore | CaptureModifier::OneOrMore => {
                let mut values = Vec::new();
                if let Some(ext) = extractor {
                    loop {
                        cursor.skip_trivia();
                        let remaining = cursor.remaining();
                        if remaining.is_empty() {
                            break;
                        }
                        if let Some((value, consumed)) = ext.extract(remaining, source) {
                            values.push(value);
                            cursor.advance(consumed);
                            // Skip comma separator if present
                            cursor.skip_trivia();
                            cursor.eat_kind(SyntaxKind::COMMA);
                        } else {
                            break;
                        }
                    }
                }

                if modifier == CaptureModifier::OneOrMore && values.is_empty() {
                    return Err(MatchFailure {
                        form_name: self.macro_name.clone(),
                        element_index,
                        kind: FailureKind::CaptureFailedNoTokens {
                            var_name: var_name.to_string(),
                            capture_type: format!("{:?}+", capture_type),
                        },
                    });
                }

                captures.insert(var_name.to_string(), CapturedValue::Array(values));
                Ok(())
            }
        }
    }

    /// Match params (from ARG_LIST).
    fn match_params(
        &self,
        params: &[FormParam],
        cursor: &mut TokenCursor,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        capture_spans: &mut HashMap<String, SourceSpan>,
        // BUG-264: set true when a param group left tokens this form could not bind
        // (a near-miss clause dropped rather than captured) — the match is LENIENT.
        lenient: &mut bool,
    ) -> Result<(), MatchFailure> {
        // BUG-145 fix: named args must bind by NAME, not by call-site position.
        //
        // The previous lockstep walk matched declared params against call tokens
        // IN THE SAME ORDER: the Nth declared param had to be the Nth comma group
        // in the call, or it silently fell back to that param's default with no
        // diagnostic (BUG-145's exact symptom: `@stage(camZ: 5.5, fov: 99)` lost
        // `fov` because %form declares `fov` before `camZ`).
        //
        // Fix:
        //   Pass 1 (named): split the arg-list into top-level comma groups; for
        //     every declared param with a non-empty `name`, search ALL groups
        //     (regardless of position) for the one whose leading token equals
        //     that name, and bind from it. This is the actual fix - named args
        //     become order-independent. A claimed group is EXCISED from the
        //     token stream (not left as an empty placeholder) so nothing
        //     double-binds AND so a variadic/param_list-style unnamed capture
        //     downstream still sees a CONTIGUOUS run of its own tokens (some
        //     %form params, e.g. `param_list`, expect to consume several
        //     comma-separated groups as ONE capture - splitting them into
        //     isolated single-group cursors would break that; excising only the
        //     named groups and reflowing the rest preserves it).
        //   Pass 2 (unnamed/positional): re-run the ORIGINAL lockstep walk
        //     (unchanged semantics: comma-separated, in declaration order)
        //     against the RECONSTRUCTED stream of whatever pass 1 didn't claim.
        //     `@each`'s form interleaves an unnamed literal-prefixed
        //     `when $filter:expr` AFTER a named `key: $key:expr?`
        //     (stdlib/macros/each.st) - reflowing preserves that shape exactly
        //     as the pre-fix lockstep matcher handled it, just skipping the
        //     bytes pass 1 already consumed.
        let groups = Self::split_top_level_groups(cursor.remaining());
        let mut pool: Vec<Option<&[TokenData]>> = groups.iter().map(|g| Some(*g)).collect();

        for param in params.iter().filter(|p| !p.name.is_empty()) {
            let found = pool.iter_mut().find_map(|slot| {
                let group = (*slot)?;
                let mut peek = TokenCursor::new(group);
                peek.skip_trivia();
                if peek.peek_text(source) == Some(param.name.as_str()) {
                    *slot = None;
                    Some(group)
                } else {
                    None
                }
            });
            if let Some(group) = found {
                let mut group_cursor = TokenCursor::new(group);
                self.match_param(
                    param,
                    &mut group_cursor,
                    source,
                    extractors,
                    captures,
                    capture_spans,
                )?;
                // BUG-264: a claimed group with tokens its param could not bind
                // (e.g. `close: close` where `close` is not a `$binding`) is a clause
                // the form silently dropped, not a clean match.
                if Self::has_dropped_tokens(&group_cursor, source) {
                    *lenient = true;
                }
            } else {
                // No group named this param anywhere in the call - same
                // default/optional/error fallback match_param already applies,
                // reached via an empty cursor (name lookup trivially fails).
                let mut empty_cursor = TokenCursor::new(&[]);
                self.match_param(
                    param,
                    &mut empty_cursor,
                    source,
                    extractors,
                    captures,
                    capture_spans,
                )?;
            }
        }

        // Reflow the unclaimed groups back into ONE contiguous token stream,
        // re-joined with COMMA tokens, so the ORIGINAL lockstep matcher runs
        // over it unmodified for the unnamed params. The joining comma must
        // slice to REAL "," text in `source` (not a synthetic zero-width
        // range) - some %capture_type PEGs (e.g. `param_list`'s
        // `("?")? ","?` literal) match a comma by TEXT via `.text(source)`,
        // not just SyntaxKind, so reuse an ACTUAL comma token's range from the
        // original arg-list (any one will do - they all slice to ","). A form
        // with only one group overall never needs a joining comma, so `None`
        // here is fine (the `first` guard below never inserts one).
        let comma: Option<TokenData> = cursor
            .remaining()
            .iter()
            .find(|t| t.kind == SyntaxKind::COMMA)
            .cloned();
        let mut reflowed: Vec<TokenData> = Vec::new();
        let mut first = true;
        for group in pool.iter().flatten() {
            if !first && let Some(c) = &comma {
                reflowed.push(c.clone());
            }
            reflowed.extend_from_slice(group);
            first = false;
        }

        let unnamed_params: Vec<&FormParam> = params.iter().filter(|p| p.name.is_empty()).collect();
        let mut reflow_cursor = TokenCursor::new(&reflowed);
        for (i, param) in unnamed_params.iter().enumerate() {
            reflow_cursor.skip_trivia();
            if i > 0 {
                reflow_cursor.skip_trivia();
                reflow_cursor.eat_kind(SyntaxKind::COMMA);
                reflow_cursor.skip_trivia();
            }
            self.match_param(
                param,
                &mut reflow_cursor,
                source,
                extractors,
                captures,
                capture_spans,
            )?;
        }
        // BUG-264: leftover tokens after ALL unnamed params is a clause the form
        // could not bind — a lenient match (the ARG_LIST trailing-leniency drop).
        if Self::has_dropped_tokens(&reflow_cursor, source) {
            *lenient = true;
        }

        Ok(())
    }

    /// BUG-264: does the cursor hold a token the form's grammar could not bind —
    /// the signal of a DROPPED clause (a lenient match)? The enclosing group's
    /// closing delimiters (`)`, `]`, `}`) are structural terminators, never a
    /// dropped value, so a cursor holding only those is still a clean match.
    fn has_dropped_tokens(cursor: &TokenCursor, source: &str) -> bool {
        cursor.remaining().iter().any(|t| {
            let _ = source;
            !t.kind.is_trivia()
                && !matches!(
                    t.kind,
                    SyntaxKind::R_PAREN | SyntaxKind::R_BRACE | SyntaxKind::R_BRACKET
                )
        })
    }

    /// Split a token slice into top-level (bracket-depth-0) comma-separated
    /// groups, e.g. `camZ: 5.5, camY: 0, fov: 99` -> 3 groups. Depth-aware so a
    /// comma inside `(...)`/`[...]`/`{...}` (a nested call's own arg list, an
    /// array literal, an object literal) does not split its enclosing group.
    /// Trivia-only groups (a stray leading/trailing comma) are dropped. This is
    /// the ONE place %form param matching segments a call's arg list by comma -
    /// BUG-145's fix (name-first, order-independent matching) builds on it.
    fn split_top_level_groups(tokens: &[TokenData]) -> Vec<&[TokenData]> {
        let mut groups = Vec::new();
        let mut depth: i32 = 0;
        let mut start = 0usize;
        for (i, tok) in tokens.iter().enumerate() {
            match tok.kind {
                SyntaxKind::L_PAREN | SyntaxKind::L_BRACE | SyntaxKind::L_BRACKET => depth += 1,
                SyntaxKind::R_PAREN | SyntaxKind::R_BRACE | SyntaxKind::R_BRACKET => depth -= 1,
                SyntaxKind::COMMA if depth == 0 => {
                    let slice = &tokens[start..i];
                    if slice.iter().any(|t| !t.kind.is_trivia()) {
                        groups.push(slice);
                    }
                    start = i + 1;
                }
                _ => {}
            }
        }
        let tail = &tokens[start..];
        if tail.iter().any(|t| !t.kind.is_trivia()) {
            groups.push(tail);
        }
        groups
    }

    /// Match a single FormParam.
    ///
    /// For named params (e.g., `src: $src:string`), the param has `name: "src"` and
    /// `elements: [Capture("src", String)]`. The cursor contains `IDENT("src") COLON ...`
    /// so we need to skip past the `name:` prefix tokens before matching elements.
    ///
    /// When a named param's name is not found in the tokens, we fall back to the
    /// param's default value (if available) or skip if captures are optional.
    fn match_param(
        &self,
        param: &FormParam,
        cursor: &mut TokenCursor,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        capture_spans: &mut HashMap<String, SourceSpan>,
    ) -> Result<(), MatchFailure> {
        if !param.name.is_empty() {
            cursor.skip_trivia();
            if cursor.peek_text(source) == Some(param.name.as_str()) {
                // Name found — eat it and the colon, then match elements below
                cursor.advance(1);
                cursor.skip_trivia();
                cursor.eat_kind(SyntaxKind::COLON);
            } else {
                // Named param not found in tokens — use default or skip
                if let Some(default) = &param.default {
                    for element in &param.elements {
                        if let FormInlineElement::Capture(cap, _) = element {
                            captures
                                .insert(cap.var_name.clone(), CapturedValue::from_default(default));
                        }
                    }
                    return Ok(());
                }
                // Check if all captures are optional
                let all_optional = param.elements.iter().all(|e| match e {
                    FormInlineElement::Capture(cap, _) => {
                        cap.modifier == CaptureModifier::Optional
                            || cap.modifier == CaptureModifier::ZeroOrMore
                    }
                    _ => false,
                });
                if all_optional {
                    return Ok(());
                }
                // Required named param not found and no default
                return Err(MatchFailure {
                    form_name: self.macro_name.clone(),
                    element_index: 0,
                    kind: FailureKind::LiteralMismatch {
                        expected: format!("{}:", param.name),
                        got: cursor.peek_text(source).map(String::from),
                    },
                });
            }
        }

        for (i, element) in param.elements.iter().enumerate() {
            self.match_inline_element(
                element,
                cursor,
                source,
                extractors,
                captures,
                capture_spans,
                i,
            )?;
        }
        Ok(())
    }

    /// Match body params (from BODY child node).
    ///
    /// Body params are matched as CSS property-like syntax:
    /// `name: value;` where each param corresponds to a property.
    fn match_body_params(
        &self,
        body_params: &[FormParam],
        body_node: &ChildNode,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        capture_spans: &mut HashMap<String, SourceSpan>,
        // BUG-264: set true when the body carried clauses no body param could bind.
        lenient: &mut bool,
    ) -> Result<(), MatchFailure> {
        let mut cursor = TokenCursor::new(&body_node.tokens);

        // Skip opening brace
        cursor.skip_trivia();
        cursor.eat_kind(SyntaxKind::L_BRACE);

        for param in body_params {
            cursor.skip_trivia();
            self.match_param(
                param,
                &mut cursor,
                source,
                extractors,
                captures,
                capture_spans,
            )?;
            // Skip semicolons between body params
            cursor.skip_trivia();
            cursor.eat_kind(SyntaxKind::SEMICOLON);
        }
        // BUG-264: tokens a body param could not bind and no body_capture consumes
        // are a clause the form silently dropped — a lenient match, not clean.
        if self.form.body_capture.is_none() && Self::has_dropped_tokens(&cursor, source) {
            *lenient = true;
        }

        Ok(())
    }

    /// Match pseudo-selector captures from SCOPE_BLOCK children in the body.
    ///
    /// When a form has PseudoSelector body_params (e.g., `:entering { $enterAnim:keyframes? }`),
    /// the CST parser creates SCOPE_BLOCK child nodes for each `:name { ... }` block.
    /// SCOPE_BLOCK is structural, so its tokens are NOT in the body's flat token stream.
    /// This method pre-builds a name→node map from SCOPE_BLOCK children (single pass),
    /// then looks up each pseudo-selector by name in O(1).
    fn match_pseudo_selectors(
        &self,
        body_node: &ChildNode,
        source: &str,
        _extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        capture_spans: &mut HashMap<String, SourceSpan>,
    ) -> Result<(), MatchFailure> {
        // Single pass: build name → &ChildNode map for all SCOPE_BLOCK children
        let pseudo_blocks: HashMap<&str, &ChildNode> = body_node
            .children
            .iter()
            .filter(|c| c.kind == SyntaxKind::SCOPE_BLOCK)
            .filter_map(|c| extract_pseudo_name(c, source).map(|name| (name, c)))
            .collect();

        for param in &self.form.body_params {
            for element in &param.elements {
                if let FormInlineElement::PseudoSelector {
                    name,
                    body_params,
                    modifier,
                } = element
                {
                    if let Some(scope_block) = pseudo_blocks.get(name.as_str()) {
                        // Find the BODY child inside this SCOPE_BLOCK
                        if let Some(inner_body) = scope_block.child_by_kind(SyntaxKind::BODY) {
                            let body_start = inner_body.span_start;
                            let body_end = inner_body.span_end;

                            if body_start < body_end && body_end <= source.len() {
                                let body_text = &source[body_start..body_end];
                                let inner = body_text
                                    .trim()
                                    .trim_start_matches('{')
                                    .trim_end_matches('}')
                                    .trim();

                                if !inner.is_empty() {
                                    for inner_param in body_params {
                                        for inner_element in &inner_param.elements {
                                            if let FormInlineElement::Capture(cap, _) =
                                                inner_element
                                            {
                                                captures.insert(
                                                    cap.var_name.clone(),
                                                    CapturedValue::String(inner.to_string()),
                                                );
                                                capture_spans.insert(
                                                    cap.var_name.clone(),
                                                    SourceSpan::new(body_start, body_end),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if *modifier == CaptureModifier::Optional
                        || *modifier == CaptureModifier::ZeroOrMore
                    {
                        // An OPTIONAL pseudo-selector that is ABSENT still binds
                        // its inner captures — to Null — so a `%binds` that
                        // references them (`rest: $rest`) receives a defined
                        // value instead of failing E0804 "missing parameter".
                        // Without this, an optional slot could only ever be used
                        // if the consumer never named its captures, which defeats
                        // the point of an optional slot (PLAN-150 W5 @scatter).
                        for inner_param in body_params {
                            for inner_element in &inner_param.elements {
                                if let FormInlineElement::Capture(cap, _) = inner_element {
                                    captures
                                        .entry(cap.var_name.clone())
                                        .or_insert(CapturedValue::Array(Vec::new()));
                                }
                            }
                        }
                    } else {
                        return Err(MatchFailure {
                            form_name: self.macro_name.clone(),
                            element_index: 0,
                            kind: FailureKind::MissingPseudoSelector { name: name.clone() },
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Match leading body GROUPS (FEAT-103) against the BODY token prefix.
    ///
    /// Each group in `self.form.body_groups` is a `CapturePatternAst` compiled by
    /// the SAME `compile_pattern` machinery that powers `%capture_type`. The BODY
    /// node's inner tokens (brace-stripped) are matched in sequence; matched
    /// captures are merged into `captures`. Returns the source BYTE offset just past
    /// the last consumed group token — the start of the remaining body run handed to
    /// the body capture. `None` means no group consumed anything (all optional and
    /// absent), so the body capture scans from its normal start.
    ///
    /// A REQUIRED group that fails to match fails the whole form (lets a sibling
    /// form win, consistent with required body-capture semantics).
    fn match_body_groups(
        &self,
        body_node: &ChildNode,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
    ) -> Result<Option<usize>, MatchFailure> {
        use crate::parser::meta_ast::CapturePatternAst;

        // Re-lex the brace-inner body text to clean tokens (mirrors the Properties /
        // Custom inner-relex path: collect_descendant_tokens overlaps parent+child).
        let body_start = body_node.span_start;
        let body_end = body_node.span_end;
        if body_start >= body_end || body_end > source.len() {
            return Ok(None);
        }
        let body_text = &source[body_start..body_end];
        let inner = body_text
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}');
        // Byte offset of `inner` within source (inner is a sub-slice of body_text).
        let inner_offset = inner.as_ptr() as usize - source.as_ptr() as usize;
        let lexer = crate::syntax::cst::Lexer::new(inner);
        let raw_tokens = lexer.tokenize();
        let tokens: Vec<TokenData> = raw_tokens
            .iter()
            .filter(|t| t.kind != SyntaxKind::EOF)
            .map(|t| TokenData {
                kind: t.kind,
                text_range: (inner_offset + t.offset, inner_offset + t.offset + t.len()),
            })
            .collect();

        // Resolve the registered stdlib %capture_type definitions so a body group
        // that references a Custom capture type (e.g. `( $send:send_clause )`) compiles
        // its REAL grammar. Without the defs map, `compile_pattern` resolves an unknown
        // Custom to the greedy `ExprExtractor`, which swallows the WHOLE body (it would
        // consume every clause, starving the sibling groups). PLAN-038 W1.
        let group_defs: std::collections::HashMap<
            String,
            crate::parser::meta_ast::CaptureTypeDefAst,
        > = crate::syntax::stdlib_registry::STDLIB_REGISTRY
            .capture_types()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();

        let mut cursor = 0usize; // index into `tokens`
        for group in &self.form.body_groups {
            let ext = super::extractors::custom::compile_pattern_with_defs(
                group,
                extractors,
                &group_defs,
                &mut Vec::new(),
            );
            // A Group extractor (Optional/Repeat/Required) consumes a token prefix.
            match ext.extract(&tokens[cursor..], source) {
                Some((value, consumed)) => {
                    merge_group_captures(group, value, captures);
                    cursor += consumed;
                }
                None => {
                    // Optional/ZeroOrMore groups compile to extractors that succeed
                    // with consumed=0; a None here is a REQUIRED group that didn't
                    // match → fail the form so a sibling can win.
                    if matches!(
                        group,
                        CapturePatternAst::Group {
                            modifier: None
                                | Some(crate::parser::meta_ast::CaptureModifier::Required),
                            ..
                        }
                    ) {
                        return Err(MatchFailure {
                            form_name: self.macro_name.clone(),
                            element_index: BODY_ELEMENT_INDEX,
                            kind: FailureKind::MissingBody,
                        });
                    }
                }
            }
        }

        if cursor == 0 {
            return Ok(None);
        }
        // Byte offset just past the last consumed group token.
        let last = &tokens[cursor - 1];
        Ok(Some(last.text_range.1))
    }

    /// Extract body captures — the body_capture spec may contain multiple
    /// `$name:type` entries (e.g., `{ $html:html_block $animations:keyframes? }`).
    /// Also handles property-style captures (e.g., `{ src: $src:string? }`).
    ///
    /// Each capture is parsed and inserted into the captures map.
    /// Optional captures (ending with `?`) are skipped if extraction fails.
    fn extract_body_captures(
        &self,
        capture_spec: &str,
        body_node: &ChildNode,
        source: &str,
        extractors: &ExtractorRegistry,
        captures: &mut HashMap<String, CapturedValue>,
        // FEAT-103: when leading body GROUPS consumed a head-line, this is the source
        // byte offset where the REMAINING body run begins. The body capture scans
        // from here instead of the brace start, so a `$body:component_body` after
        // `( shortcut: ... )?` sees only the projection, not the consumed metadata.
        body_inner_start: Option<usize>,
    ) -> Result<(), MatchFailure> {
        let spec = capture_spec
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();

        // Parse individual capture specs from the body_capture string.
        // Each spec is like "$name:type" or "$name:type?" or "$children*".
        // Property-style specs have a prop_key (e.g., "src: $src:string?").
        let capture_specs = parse_body_capture_specs(spec);

        if capture_specs.is_empty() {
            return Ok(());
        }

        // Check for the simple $children* case (no type annotation)
        if capture_specs.len() == 1 && capture_specs[0].is_children_collector {
            captures.insert(
                capture_specs[0].name.clone(),
                CapturedValue::Block(body_node.matched_children.clone()),
            );
            return Ok(());
        }

        // Extract body text (between braces). When leading body groups consumed a
        // head-line (FEAT-103), start the body run at `body_inner_start` instead of
        // the brace, so the body capture sees only the remaining projection.
        let body_start = body_node.span_start;
        let body_end = body_node.span_end;
        if body_start >= body_end || body_end > source.len() {
            return Ok(());
        }
        let inner = match body_inner_start {
            Some(start) if start >= body_start && start <= body_end => {
                // From the group boundary to the body end; strip a trailing `}`.
                source[start..body_end].trim().trim_end_matches('}').trim()
            }
            _ => {
                let body_text = &source[body_start..body_end];
                body_text
                    .trim()
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .trim()
            }
        };

        // Check if any specs are property-style
        let has_property_specs = capture_specs.iter().any(|s| !s.prop_key.is_empty());

        if has_property_specs {
            // Property-style capture: parse body as `key: value;` pairs
            // and match against property-key specs.
            let prop_map = parse_body_properties(inner);

            for cap_spec in &capture_specs {
                if cap_spec.prop_key.is_empty() {
                    continue;
                }

                if let Some(value_str) = prop_map.get(cap_spec.prop_key.as_str()) {
                    // Convert based on capture type
                    let capture_type = match parse_capture_type_name(&cap_spec.type_name) {
                        Some(ct) => ct,
                        None => continue,
                    };
                    // BUG-264 strict-capture: a PRESENT clause whose value is the
                    // wrong KIND for its declared capture type rejects the productive
                    // form — `refresh: 5` must not match `$refresh:duration?` by
                    // falling back to Expr (the near-miss silently vanished). A
                    // reportable CaptureTypeMismatch lets an error sibling win, or
                    // the committed-then-failed report fire, instead of a silent drop.
                    if is_bare_number_duration_reject(value_str, &capture_type) {
                        return Err(MatchFailure {
                            form_name: self.macro_name.clone(),
                            element_index: BODY_ELEMENT_INDEX,
                            kind: FailureKind::CaptureTypeMismatch {
                                var_name: cap_spec.name.clone(),
                                expected_type: format!("{:?}", capture_type),
                                expected_alternatives: Vec::new(),
                                got_kind: "bare number (missing duration unit)".to_string(),
                            },
                        });
                    }
                    let value = convert_property_value(value_str, &capture_type);
                    captures.insert(cap_spec.name.clone(), value);
                } else if let Some(ref default) = cap_spec.default_value {
                    // Use default value
                    let capture_type = match parse_capture_type_name(&cap_spec.type_name) {
                        Some(ct) => ct,
                        None => continue,
                    };
                    let value = convert_property_value(default, &capture_type);
                    captures.insert(cap_spec.name.clone(), value);
                }
                // Optional specs with no value get a null default so that
                // downstream resolve/bind never sees them as "missing required".
                if matches!(cap_spec.modifier, CaptureModifier::Optional)
                    && !captures.contains_key(&cap_spec.name)
                {
                    captures.insert(
                        cap_spec.name.clone(),
                        CapturedValue::Expr("null".to_string()),
                    );
                }
            }

            // A body may MIX property specs (`accepts: $a;`) with NON-property
            // captures (`$customStates:states?`, `( $lifecycle:drag_lifecycle_arm )*`)
            // — e.g. dnd's `@drop-zone` and `@drag` both do. The property specs are
            // now bound; DON'T early-return — fall through to the general
            // capture loop below, which extracts the remaining non-property specs
            // through their real extractors (states/custom-type/modifier-aware),
            // skipping the property specs already handled here. Without this the
            // non-property captures were dropped, so a caller's `over { … }` /
            // lifecycle arms never reached the macro.
        }

        // Simple capture specs — extract from body tokens/text. Track whether every
        // REQUIRED capture (non-optional, non-zero-or-more) actually produced a value:
        // a required body capture that extracts NOTHING from a NON-EMPTY body means this
        // form does not fit the body shape, so try_match must reject it (lets a sibling
        // form — e.g. `$body:html_block` vs `$invocations:template_invocation+` — win
        // instead of deferring a confusing error to resolve). PLAN-023 W4.
        let body_is_nonempty = !inner.is_empty();
        let mut unsatisfied_required: Option<String> = None;
        for cap_spec in &capture_specs {
            // Property specs (`key: $var`) are bound in the property branch above;
            // skip them here so a mixed body (property + non-property captures)
            // doesn't re-bind them from the wrong slice.
            if !cap_spec.prop_key.is_empty() {
                continue;
            }
            if cap_spec.is_children_collector {
                captures.insert(
                    cap_spec.name.clone(),
                    CapturedValue::Block(body_node.matched_children.clone()),
                );
                continue;
            }

            let capture_type = match parse_capture_type_name(&cap_spec.type_name) {
                Some(ct) => ct,
                None => continue,
            };

            // Extract value based on capture type
            let value = if inner.is_empty() {
                None
            } else {
                match capture_type {
                    CaptureType::Keyframes => Some(CapturedValue::String(inner.to_string())),
                    CaptureType::MutationActions => {
                        // String-aware: a `;` inside a quoted value is content,
                        // not a statement separator (BUG-225).
                        let stmts: Vec<CapturedValue> = split_statements_outside_strings(inner)
                            .into_iter()
                            .map(|s| CapturedValue::String(s.to_string()))
                            .collect();
                        let mut map = HashMap::new();
                        map.insert("js_statements".to_string(), CapturedValue::Array(stmts));
                        map.insert("macro_calls".to_string(), CapturedValue::Array(vec![]));
                        Some(CapturedValue::Named(map))
                    }
                    CaptureType::HtmlBlock => Some(CapturedValue::String(inner.to_string())),
                    CaptureType::Properties => {
                        // Re-lex the inner text to get clean tokens for PropertiesExtractor.
                        // Using collect_descendant_tokens from body_node produces duplicates
                        // (parent + child tokens overlap), which confuses the extractor.
                        //
                        // Compute inner's byte offset within source via pointer arithmetic:
                        // inner is a slice of source (through body_text), so this is safe.
                        let inner_offset = inner.as_ptr() as usize - source.as_ptr() as usize;
                        let lexer = crate::syntax::cst::Lexer::new(inner);
                        let raw_tokens = lexer.tokenize();
                        let tokens: Vec<super::extractors::TokenData> = raw_tokens
                            .iter()
                            .filter(|t| {
                                t.kind != SyntaxKind::L_BRACE
                                    && t.kind != SyntaxKind::R_BRACE
                                    && t.kind != SyntaxKind::EOF
                            })
                            .map(|t| super::extractors::TokenData {
                                kind: t.kind,
                                text_range: (
                                    inner_offset + t.offset,
                                    inner_offset + t.offset + t.len(),
                                ),
                            })
                            .collect();
                        if tokens.is_empty() {
                            None
                        } else if let Some(ext) = extractors.get(&capture_type) {
                            ext.extract(&tokens, source).map(|(v, _)| v)
                        } else {
                            None
                        }
                    }
                    CaptureType::ComponentBody => {
                        // FEAT-120: a bare presence-marker. The body's payload
                        // (html/states/exports/refs) is produced on the World-A path in
                        // `cst_to_stfile`, and its validation diagnostics (E0900/E0904/
                        // E0906/W0700) are emitted there too — `validate_component_body`
                        // runs in the scope-building loop, where the template name + body
                        // span are in hand. This extractor only marks the capture present.
                        Some(CapturedValue::ComponentBody)
                    }
                    // Custom (stdlib %capture_type) body capture: resolve from the compiled
                    // custom map and run over the re-lexed inner text (PLAN-023 W2). Mirrors
                    // the Properties inner-relex path to avoid duplicate parent/child tokens.
                    CaptureType::Custom(ref name) => {
                        let inner_offset = inner.as_ptr() as usize - source.as_ptr() as usize;
                        let lexer = crate::syntax::cst::Lexer::new(inner);
                        let raw_tokens = lexer.tokenize();
                        // Keep brace tokens: a stdlib production may need them (e.g.
                        // `properties` uses `skip_block` to step over nested `@dir { ... }`
                        // blocks and continue — PLAN-023 W2). Only EOF is dropped. Re-lexing
                        // `inner` (vs collect_descendant_tokens) avoids duplicate
                        // parent/child token overlap.
                        let tokens: Vec<super::extractors::TokenData> = raw_tokens
                            .iter()
                            .filter(|t| t.kind != SyntaxKind::EOF)
                            .map(|t| super::extractors::TokenData {
                                kind: t.kind,
                                text_range: (
                                    inner_offset + t.offset,
                                    inner_offset + t.offset + t.len(),
                                ),
                            })
                            .collect();
                        if tokens.is_empty() {
                            None
                        } else if let Some(ext) = extractors.custom.get(name) {
                            // Honor the repeat modifier so `match_arm+` (and any repeated
                            // CUSTOM %capture_type) accumulates into an Array, exactly like
                            // a repeated builtin (template_invocation+).
                            extract_with_modifier(ext.as_ref(), &tokens, source, cap_spec.modifier)
                        } else {
                            // BUG-229 (B): NO lenient fallback. Passing the raw inner
                            // text through here made a body that CANNOT match its
                            // declared grammar indistinguishable from one that does:
                            // the capture "succeeded" with unparsed text, the form
                            // matched, and `check` went green on malformed source.
                            // An unresolvable capture type is a failure to match, and
                            // is reported as such by the caller.
                            None
                        }
                    }
                    _ => {
                        let mut tokens = Vec::new();
                        collect_descendant_tokens(body_node, &mut tokens);
                        tokens.retain(|t| {
                            t.kind != SyntaxKind::L_BRACE
                                && t.kind != SyntaxKind::R_BRACE
                                && t.kind != SyntaxKind::EOF
                        });
                        tokens.sort_by_key(|t| t.text_range.0);

                        if tokens.is_empty() {
                            None
                        } else if let Some(ext) = extractors.get(&capture_type) {
                            extract_with_modifier(ext, &tokens, source, cap_spec.modifier)
                        } else {
                            None
                        }
                    }
                }
            };

            // A repeated capture (`+`/`*`) yielding ZERO items from a NON-EMPTY
            // body does not fit the body shape — the PLAN-023 W4 rule (a
            // required capture extracting nothing rejects the form) extended to
            // collections. PLAN-126: this is what lets the `@on` arms/body
            // sibling forms discriminate `{ true => --x; }` (arms) from
            // `{ font-size: 14px -> 24px; }` (motion body) — before this, the
            // loser's tokens were silently swallowed as an empty Array.
            let body_has_content = !inner.trim().is_empty();
            let mut satisfied = value.is_some();
            // Modifier-independent: the EMPTY Array is the misfit signal,
            // whether the repeat is at the spec (`$arms:on_arm+`) or inside
            // the capture type (`$body:on_motion_body`, Required). Scoped to
            // SINGLE-SPEC bodies: with one capture the empty result is an
            // unambiguous "wrong form" signal, while a MIXED body (dnd's
            // states/lifecycle/on-drop) legitimately leaves sub-captures
            // empty — there the historical satisfied-empty masking stands.
            let zero_items_on_content = body_has_content
                && capture_specs.len() == 1
                && matches!(&value, Some(CapturedValue::Array(items)) if items.is_empty());
            if zero_items_on_content {
                satisfied = false;
            }
            if let Some(val) = value {
                captures.insert(cap_spec.name.clone(), val);
            } else if matches!(capture_type, CaptureType::HtmlBlock) && !body_is_nonempty {
                // An EMPTY body satisfies a required `html_block` capture as empty HTML
                // (renders nothing) — e.g. an empty `@each { }` is valid, not an error
                // (PLAN-023 W4, dissolves BUG-030). Other required captures stay unmet.
                captures.insert(cap_spec.name.clone(), CapturedValue::String(String::new()));
                satisfied = true;
            }
            // OneOrMore (`+`) demands ≥1 element: an empty result is ALWAYS a mismatch
            // (even on an empty body) so e.g. `$invocations:template_invocation+` rejects
            // both empty and HTML bodies, letting the html_block form win. A plain
            // Required capture only mismatches when the body is non-empty (an empty body
            // is handled above for html_block).
            let rejects_when_unmet = match cap_spec.modifier {
                // `*` tolerates an empty result ONLY on an empty body — zero
                // items from a NON-EMPTY body means the sibling form fits
                // better (PLAN-126 arms/body discrimination).
                CaptureModifier::Optional => false,
                CaptureModifier::ZeroOrMore => zero_items_on_content,
                // A Required capture whose value is an empty Array on a
                // NON-EMPTY body is the same misfit (the repeat lives
                // inside the capture type).
                CaptureModifier::OneOrMore => true,
                // Counted behaves as Required here (see the capture-level match).
                CaptureModifier::Required | CaptureModifier::Counted(_) => {
                    body_is_nonempty || zero_items_on_content
                }
            };
            if rejects_when_unmet && !satisfied && unsatisfied_required.is_none() {
                unsatisfied_required = Some(cap_spec.name.clone());
            }
        }

        if let Some(name) = unsatisfied_required {
            return Err(MatchFailure {
                form_name: self.macro_name.clone(),
                element_index: BODY_ELEMENT_INDEX,
                kind: FailureKind::CaptureFailedNoTokens {
                    var_name: name,
                    capture_type: "body".to_string(),
                },
            });
        }
        Ok(())
    }

    /// Apply defaults for missing optional params.
    fn apply_param_defaults(
        &self,
        params: &[FormParam],
        captures: &mut HashMap<String, CapturedValue>,
    ) {
        for param in params {
            // Check param-level default first
            if let Some(default) = &param.default
                && let Some(cap) = param.capture()
                && !captures.contains_key(&cap.var_name)
            {
                captures.insert(cap.var_name.clone(), CapturedValue::from_default(default));
            }
            // Then check inline-element-level defaults
            for element in &param.elements {
                if let FormInlineElement::Capture(cap, Some(default)) = element
                    && !captures.contains_key(&cap.var_name)
                {
                    captures.insert(cap.var_name.clone(), CapturedValue::from_default(default));
                }
            }
        }
    }
}

/// Parse body text as `key: value;` property pairs.
/// Returns a map of key → value (trimmed, with quotes stripped from strings).
///
/// Splitting is bracket- and string-aware: a property value runs from its
/// `:` to the next `;` at bracket depth 0, so values may span multiple lines
/// and contain nested `[]`/`{}`/`()` literals (e.g. `value: [ {...}, {...} ];`).
/// The `key` is the identifier immediately before the `:` (the last bare
/// identifier run), which lets a value's trailing text not bleed into the
/// next key.
/// PLAN-150 W8: walk a form-inline element and call `f` with every capture var
/// name it contains (recursing into nested groups). Used to aggregate the
/// captures a REPEATED group produces across iterations into an Array.
fn collect_group_capture_vars(el: &FormInlineElement, f: &mut impl FnMut(&str)) {
    match el {
        FormInlineElement::Capture(cap, _) => f(&cap.var_name),
        FormInlineElement::Comparison { capture, .. } => f(&capture.var_name),
        FormInlineElement::Group { elements, .. } => {
            for e in elements {
                collect_group_capture_vars(e, f);
            }
        }
        _ => {}
    }
}

fn parse_body_properties(inner: &str) -> HashMap<&str, &str> {
    let mut props = HashMap::new();
    let bytes = inner.as_bytes();
    let mut depth: i32 = 0;
    let mut i = 0usize;
    // Start of the current statement (after the previous `;` at depth 0).
    let mut stmt_start = 0usize;
    // String state.
    let mut in_str = false;
    let mut str_quote = 0u8;

    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == str_quote {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'\'' | b'`' => {
                in_str = true;
                str_quote = c;
            }
            b'[' | b'{' | b'(' => depth += 1,
            b']' | b')' => depth -= 1,
            b'}' => {
                depth -= 1;
                // A nested block that closes back to depth 0 (e.g. a `dragging { ... }`
                // states sub-block preceding `on-drop:`) is an implicit statement
                // boundary — the macro body uses no `;` between a block and the next
                // property. Without this, `dragging {...} on-drop: $move(...)` parses
                // as ONE statement keyed `dragging {...} on-drop`, so the `on-drop`
                // property is never extracted (PLAN-053). Flush the block as its own
                // statement (insert_body_property ignores a `key {...}` with no
                // depth-0 `:`), and start the next statement after the brace.
                if depth == 0 {
                    insert_body_property(&mut props, &inner[stmt_start..=i]);
                    stmt_start = i + 1;
                }
            }
            b';' if depth == 0 => {
                insert_body_property(&mut props, &inner[stmt_start..i]);
                stmt_start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    // Trailing statement without a closing `;`.
    if stmt_start < inner.len() {
        insert_body_property(&mut props, &inner[stmt_start..]);
    }
    props
}

/// Insert a single `key: value` statement into the property map, splitting at
/// the FIRST `:` at bracket depth 0 (so `value: [ { "a": 1 } ]` keeps the
/// nested object colons inside the value).
fn insert_body_property<'a>(props: &mut HashMap<&'a str, &'a str>, stmt: &'a str) {
    let trimmed = stmt.trim();
    if trimmed.is_empty() {
        return;
    }
    let bytes = trimmed.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut str_quote = 0u8;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == str_quote {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'\'' | b'`' => {
                in_str = true;
                str_quote = c;
            }
            b'[' | b'{' | b'(' => depth += 1,
            b']' | b'}' | b')' => depth -= 1,
            b':' if depth == 0 => {
                let key = trimmed[..i].trim();
                let value = trimmed[i + 1..].trim();
                if !key.is_empty() {
                    props.insert(key, value);
                }
                return;
            }
            _ => {}
        }
        i += 1;
    }
}

/// Strip a pair of matching outer quotes ONLY when they wrap the ENTIRE value
/// (a whole string literal like `"todo"` → `todo`). An expression that merely
/// starts/ends with a quote (`$.col == "todo"`, `"a" + b`) keeps both quotes —
/// blindly `trim_matches('"')` would drop the closing quote and corrupt the
/// expression into invalid JS (BUG-127).
fn strip_wrapping_quotes(s: &str) -> &str {
    let t = s.trim();
    let bytes = t.as_bytes();
    if t.len() >= 2 {
        let first = bytes[0];
        if (first == b'"' || first == b'\'' || first == b'`') && bytes[t.len() - 1] == first {
            // Confirm the opening quote is not closed BEFORE the end (i.e. the
            // whole value is one string literal, not `"a" == "b"`). Scan the
            // interior for an unescaped matching quote before the last byte.
            let inner = &t[1..t.len() - 1];
            let ib = inner.as_bytes();
            let mut i = 0;
            while i < ib.len() {
                if ib[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if ib[i] == first {
                    // A matching quote closes the literal early → NOT a single
                    // wrapping pair; leave the value untouched.
                    return t;
                }
                i += 1;
            }
            return inner;
        }
    }
    t
}

/// Split a statement list on `;` **outside string literals**.
///
/// A naive `text.split(';')` cuts inside a quoted value, so a mutation whose
/// string contains a semicolon — ordinary in prose: `"false for nobody; every
/// owner wants control"` — is severed mid-literal. The fragments then reach the
/// runtime as separate statements and the eval throws
/// `SyntaxError: Invalid or unexpected token`, at click time, with a green
/// compile (BUG-225).
///
/// Tracks `"`, `'` and `` ` `` (honouring backslash escapes); a `;` separates
/// only at quote depth zero.
///
/// `raw = false` → segments trimmed, empties dropped (statement lists).
/// `raw = true`  → segments verbatim, empties kept, so `join(";")` round-trips
/// byte-identically (rewrite-and-rejoin callers).
fn split_semicolons(text: &str, raw: bool) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) => {
                if b == b'\\' {
                    i += 2;
                    continue;
                }
                if b == q {
                    quote = None;
                }
            }
            None => {
                if b == b'"' || b == b'\'' || b == b'`' {
                    quote = Some(b);
                } else if b == b';' {
                    let seg = &text[start..i];
                    if raw {
                        out.push(seg);
                    } else {
                        let t = seg.trim();
                        if !t.is_empty() {
                            out.push(t);
                        }
                    }
                    start = i + 1;
                }
            }
        }
        i += 1;
    }

    let seg = &text[start..];
    if raw {
        out.push(seg);
    } else {
        let t = seg.trim();
        if !t.is_empty() {
            out.push(t);
        }
    }
    out
}

/// Statement segments, trimmed with empties dropped. See [`split_semicolons`].
pub(crate) fn split_statements_outside_strings(text: &str) -> Vec<&str> {
    split_semicolons(text, false)
}

/// Lossless segments for rewrite-and-rejoin. See [`split_semicolons`].
pub(crate) fn split_statements_outside_strings_raw(text: &str) -> Vec<&str> {
    split_semicolons(text, true)
}

/// Convert a property value string to a CapturedValue based on capture type.
/// BUG-264 strict-capture: is `value` the WRONG KIND for a `:duration`/`:time`
/// property capture? A bare number (`refresh: 5`) is a near-miss for
/// `refresh: 5s`, not a valid millisecond value — the productive form must REFUSE
/// it (so an error sibling can fire) rather than silently thread a raw number
/// through the Expr fallback. A binding/expr (`refresh: $userInterval`) is still
/// legitimate and keeps the Expr fallback, so only a near-miss value rejects —
/// a bare number, a wrong-unit length, a spaced number, or a bare ident that is
/// not a `$` signal.
///
/// Is this a bare numeric ZERO (`0`, `0.0`)? ZERO needs no unit under the
/// CSS convention, and is the stdlib's own no-auto-refresh sentinel for
/// `refresh:` (data-kind.st:82,482) — so the strict-capture rejection must
/// let it through, not treat it as a unit-less number near-miss.
fn is_zero_duration_sentinel(clean: &str) -> bool {
    clean.parse::<f64>().map(|n| n == 0.0).unwrap_or(false)
}

/// A `:duration`/`:time` capture's strict near-miss guard (BUG-264): a
/// PRESENT clause whose value cannot be a duration AND is not a signal/
/// binding reference rejects the productive form so the error sibling can
/// report it (E0955) instead of the value converting to Expr, binding
/// cleanly, and silently dropping (the timer never starts).
fn is_bare_number_duration_reject(value: &str, capture_type: &CaptureType) -> bool {
    match capture_type {
        CaptureType::Duration | CaptureType::Time => {
            let clean = strip_wrapping_quotes(value);
            let clean = clean.trim();
            // ZERO needs no unit (CSS convention): `refresh: 0` is the stdlib's
            // own "never auto-refresh" sentinel (data-kind.st:82,482), so it must
            // bind cleanly rather than trip the strict rejection. Same for the
            // `0.0` spelling a generated bind may emit.
            if is_zero_duration_sentinel(clean) {
                return false;
            }
            // A real duration literal (`5s`, `300ms`, `1m`, `60fps`) passes
            // through unchanged.
            if crate::syntax::conversions::parse_duration_with_unit(clean).is_some() {
                return false;
            }
            // A signal/binding reference (`$every`, `$userInterval`) is an
            // expression the runtime resolves at run time — passes. `foo` (a
            // bare ident, no `$`) is a NEAR-MISS: it converts to Expr, binds
            // cleanly, and silently drops (the timer never starts). The `$`
            // sigil is the contract's marker for "a signal" (data-kind.st's
            // invalid-refresh sibling says `refresh:` takes "a duration … or a
            // `$signal`").
            if clean.starts_with('$') {
                return false;
            }
            // Everything else — a bare number (`5`), a wrong-unit length
            // (`5px`), a spaced number (`5 ms`), a bare ident (`foo`) — is a
            // near-miss for a `:duration` capture. Reject so the error sibling
            // can report it (E0955) instead of the value silently dropping.
            true
        }
        _ => false,
    }
}

fn convert_property_value(value_str: &str, capture_type: &CaptureType) -> CapturedValue {
    let clean = strip_wrapping_quotes(value_str);
    match capture_type {
        CaptureType::String => CapturedValue::String(clean.to_string()),
        CaptureType::Number => {
            if let Ok(n) = clean.parse::<f64>() {
                CapturedValue::Number(n)
            } else {
                CapturedValue::String(clean.to_string())
            }
        }
        CaptureType::Time | CaptureType::Duration => {
            // Parse a literal time/duration token ("5m", "300ms", "2s") to a number
            // of milliseconds, matching the token-stream path (extractors/simple.rs)
            // and param-default path (resolve.rs) which both emit `Time(ms)`. A
            // non-literal value (a binding/expr like `$userInterval`) won't parse —
            // fall back to `Expr` so it still threads through unchanged.
            match crate::syntax::conversions::parse_duration_with_unit(clean) {
                Some((ms, _unit)) => CapturedValue::Time(ms),
                None => CapturedValue::Expr(clean.to_string()),
            }
        }
        CaptureType::Ident => CapturedValue::Ident(clean.to_string()),
        // A dashed-ident must still BE dashed in property-value position. This
        // path converts an already-sliced value string rather than running the
        // extractor, so without an explicit arm it fell to `_ => Expr` and
        // accepted anything — `form: rise` would bind happily, which is exactly
        // the discrimination the type exists to enforce. Returning None-shaped
        // Expr is not an option here (the fn is infallible), so an undashed value
        // is preserved as Expr and the form's own grammar rejects it downstream;
        // a dashed one becomes a proper Ident.
        CaptureType::DashedIdent => {
            if clean.starts_with("--") && clean.len() > 2 {
                CapturedValue::Ident(clean.to_string())
            } else {
                CapturedValue::Expr(clean.to_string())
            }
        }
        CaptureType::Binding => CapturedValue::Binding(clean.to_string()),
        CaptureType::Color => CapturedValue::Color(clean.to_string()),
        // FEAT-109 Tier 1 — the keystone. Everything above is DECLARATION-driven:
        // the value's type is known only because a form said `:color`, `:time`,
        // `:number`. This arm is where a literal whose type nobody wrote down
        // used to become an opaque `Expr`, and that discard is why four
        // independent "is this a colour?" detectors exist downstream — at the
        // LSP, in visual lint, in the sync layer — each guessing back from the
        // string what the compiler had in its hand and dropped.
        //
        // So before falling to `Expr`, ASK. `infer_value_type` runs the same
        // grammars the `%scalar_type` table names, with recognition polarity: it
        // answers only on a positive, unambiguous match and abstains otherwise
        // (src/types/value_infer.rs explains why that is the opposite default
        // from the validator's, and why both are correct).
        //
        // Three properties make this a strict addition rather than a re-typing
        // of the corpus, and each is a gate in tests/tier1_capture_inference_test.rs:
        //
        //   1. it runs ONLY here, so a declared type always wins — inference
        //      never second-guesses an author who wrote `:string`;
        //   2. an abstention (compound value, reference, tie, nonsense) reaches
        //      the exact `Expr` it reached before, unchanged;
        //   3. a bare `600` infers as a NUMBER and never a time. The unit lives
        //      at the declaration site, and a blanket number→time widening once
        //      made every stagger in the corpus 1000x too slow with a green
        //      build. `Number` is deliberately NOT mapped below for that reason:
        //      re-typing bare numerics is a behaviour change, not a gap-fill.
        // FEAT-109 Tier 2 threads the DECLARED type in as context. A capture the
        // parser could not map to a builtin arrives as `Custom(name)`, and that
        // name is frequently a real stdlib scalar (`duration`, `length`,
        // `angle`) — so the slot is known here even though no arm above matched
        // it. Passing it lets `600` in a `$reveal:duration` become a time, while
        // a bare `600` with no slot stays a number.
        CaptureType::Custom(name) => infer_captured_value(clean, Some(name.as_str())),
        _ => infer_captured_value(clean, None),
    }
}

/// The Tier-1 fallback: recover a `CapturedValue` variant from the literal's own
/// shape when no declaration supplied one.
///
/// `declared` is the capture type's own name when the parser mapped it to a
/// `Custom(...)` — i.e. a stdlib scalar like `duration` that has no builtin
/// `CaptureType` arm above. That is the Tier-2 context: `$reveal:duration` with
/// a value of `600` means 600 MILLISECONDS, and nothing in `600` says so. The
/// unit belongs to the declaration site, not the spelling.
///
/// Only the variants whose meaning is unambiguous are mapped. `Color`, `Time`
/// and `Length` already exist on `CapturedValue` — they were simply unreachable
/// without a declaration, which is the whole defect. Any scalar without a
/// variant here (`angle`, `percentage`, `easing`) falls through to `Expr`
/// exactly as before: the inference is REAL for them, but there is nowhere
/// lossless to put it yet, and inventing a variant per scalar is the closed-enum
/// rot this codebase keeps deleting. When the captured value grows a general
/// `Scalar { ty, text }` shape, this function becomes one line.
fn infer_captured_value(clean: &str, declared: Option<&str>) -> CapturedValue {
    use crate::types::value_infer::{Inferred, infer_value_type_in_context};

    match infer_value_type_in_context(clean, declared) {
        Inferred::Scalar(ty) => match ty.as_str() {
            "color" => CapturedValue::Color(clean.to_string()),
            // A duration's payload is milliseconds, matching the declared
            // `Time`/`Duration` path above and the param-default path in
            // resolve.rs.
            //
            // NB this is where Tier 2 pays: a bare `600` in a `duration` slot
            // arrives here as a duration, and `parse_duration_with_unit` must
            // therefore accept a unitless number as milliseconds. Where it
            // cannot, we degrade to `Expr` rather than guess a magnitude — an
            // animation running at the wrong speed is worse than one that does
            // not run, because only the second is visible.
            "duration" | "time" => {
                match crate::syntax::conversions::parse_duration_with_unit(clean) {
                    Some((ms, _unit)) => CapturedValue::Time(ms),
                    None => match clean.parse::<f64>() {
                        Ok(n) if n.is_finite() && n >= 0.0 => CapturedValue::Time(n as u32),
                        _ => CapturedValue::Expr(clean.to_string()),
                    },
                }
            }
            _ => CapturedValue::Expr(clean.to_string()),
        },
        // Ambiguous, Reference and Unknown all mean "recognition has no single
        // answer" — which is exactly the condition `Expr` already described.
        _ => CapturedValue::Expr(clean.to_string()),
    }
}

/// Test-only handle on the capture converter (FEAT-109 Tier 1 gates).
///
/// The converter itself stays private: it is an internal step of form
/// compilation and nothing outside this module should be reaching into it. The
/// Tier-1 gates need to assert the CONTRACT at exactly this boundary, though —
/// "an undeclared literal keeps its type, a declared one is never re-inferred" —
/// and asserting that through a whole compile would test six other things at
/// once and fail for the wrong reasons.
pub fn convert_property_value_public(capture_type: &CaptureType, value_str: &str) -> CapturedValue {
    convert_property_value(value_str, capture_type)
}

/// Parse a capture type name string to CaptureType enum.
fn parse_capture_type_name(name: &str) -> Option<CaptureType> {
    Some(match name {
        "ident" => CaptureType::Ident,
        "dashed_ident" => CaptureType::DashedIdent,
        "event_name" => CaptureType::EventName,
        "string" => CaptureType::String,
        "number" => CaptureType::Number,
        "bool" => CaptureType::Bool,
        // color/length/duration/time/easing migrated to stdlib (PLAN-122 W1.2):
        // they fall through to Custom and resolve against the grammars in
        // stdlib/capture-types/css-values.st, exactly as they do in
        // `parse_capture_type` (src/parser/mod.rs).
        //
        // This table is a DUPLICATE of that one (FUP-151: "duplicated
        // capture-type name table fails silently"), which is why the migration
        // has to be made twice and why forgetting the second copy is invisible:
        // the name simply keeps resolving to the Rust arm on this path while the
        // stdlib production sits unused.
        "typeref" => CaptureType::Typeref,
        "binding" => CaptureType::Binding,
        "element" => CaptureType::Element,
        "preset" => CaptureType::Preset,
        "event" => CaptureType::Event,
        "expr" => CaptureType::Expr,
        "selector" => CaptureType::Selector,
        // properties migrated to stdlib (PLAN-023 W2): resolves via Custom + reify_properties.
        // keyframes migrated to stdlib (PLAN-023 W2): Custom + reify_keyframes.
        // params migrated to stdlib (PLAN-023 W2): resolves via Custom + reify_params.
        // fields migrated to stdlib (PLAN-023 W2): resolves via Custom + reify_properties.
        "states" => CaptureType::States,
        "transitions" => CaptureType::Transitions,
        "mutation_actions" => CaptureType::MutationActions,
        "template" => CaptureType::Template,
        // param_list now lives in stdlib (capture-types/param_list.st) and resolves via the
        // Custom path + reify_param_list (PLAN-023 W2). No longer hardcoded here.
        "skip_block" => CaptureType::SkipBlock,
        "html_block" => CaptureType::HtmlBlock,
        "js_block" => CaptureType::JsBlock,
        "template_invocation" => CaptureType::TemplateInvocation,
        // "color" migrated to stdlib with the other scalars above (PLAN-122 W1.2).
        "block" => CaptureType::HtmlBlock,
        "component_body" => CaptureType::ComponentBody,
        // Unknown names resolve as a Custom capture type — a stdlib %capture_type production
        // (PLAN-023 W2), matched via the compiled custom extractors.
        other => CaptureType::Custom(other.to_string()),
    })
}

/// A parsed body capture specification.
struct BodyCaptureSpec {
    name: String,
    type_name: String,
    modifier: CaptureModifier,
    is_children_collector: bool,
    /// For property-style captures like `src: $src:string?`, this is "src".
    /// Empty for simple captures like `$html:html_block`.
    prop_key: String,
    /// Default value (e.g., "0" for `cache: $cache:duration = 0`)
    default_value: Option<String>,
}

/// Parse a body_capture string into individual capture specs.
///
/// Handles both simple captures and property-style captures:
/// - Simple: `"$html:html_block $animations:keyframes?"` → two specs with empty prop_key
/// - Property: `"src: $src:string?\ncache: $cache:duration = 0"` → specs with prop_key
fn parse_body_capture_specs(spec: &str) -> Vec<BodyCaptureSpec> {
    let mut specs = Vec::new();

    // Split by lines to handle property-style captures, then by whitespace for
    // inline captures. Property-style lines have `key: $var:type` patterns.
    for line in spec.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Check if this line is a property-style capture: "key: $var:type"
        // Detect by looking for `identifier: $` pattern
        if let Some(colon_pos) = line.find(':') {
            let before = line[..colon_pos].trim();
            let after = line[colon_pos + 1..].trim();

            // Property-style: key is a plain identifier, value starts with $
            if !before.is_empty()
                && !before.starts_with('$')
                && !before.starts_with('&')
                && before
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                && after.starts_with('$')
            {
                // Parse: "key: $var:type? = default"
                let prop_key = before.to_string();

                // Extract the $var:type part and optional default
                let rest = after.trim_start_matches('$');
                let (capture_part, default_value) = if let Some(eq_pos) = rest.find('=') {
                    let cap = rest[..eq_pos].trim();
                    let def = rest[eq_pos + 1..].trim().to_string();
                    (cap, Some(def))
                } else {
                    (rest, None)
                };

                // Parse optional/required modifier
                let (modifier, capture_part) = if capture_part.ends_with('?') {
                    (
                        CaptureModifier::Optional,
                        &capture_part[..capture_part.len() - 1],
                    )
                } else if capture_part.ends_with('+') {
                    (
                        CaptureModifier::OneOrMore,
                        &capture_part[..capture_part.len() - 1],
                    )
                } else if capture_part.ends_with('*') {
                    (
                        CaptureModifier::ZeroOrMore,
                        &capture_part[..capture_part.len() - 1],
                    )
                } else {
                    (CaptureModifier::Required, capture_part)
                };

                if let Some((name, type_name)) = capture_part.split_once(':') {
                    specs.push(BodyCaptureSpec {
                        name: name.to_string(),
                        type_name: type_name.to_string(),
                        modifier,
                        is_children_collector: false,
                        prop_key,
                        default_value,
                    });
                }
                continue;
            }
        }

        // Simple capture(s) on this line — split by whitespace
        for token in line.split_whitespace() {
            let token = token.trim_start_matches('$').trim_start_matches('&');
            if token.is_empty() {
                continue;
            }

            // Children collector: bare `children*` (no `:type`). A typed
            // `name:type*` (e.g. `transitions:transitions*`) is NOT a children
            // collector — it must fall through to the typed branch below so the
            // name/type split happens (else the capture key becomes
            // "name:type" and `%for $x in $name` can't find it). (PLAN-027 W3.)
            if let Some(name) = token.strip_suffix('*')
                && !name.contains(':')
            {
                specs.push(BodyCaptureSpec {
                    name: name.to_string(),
                    type_name: String::new(),
                    modifier: CaptureModifier::ZeroOrMore,
                    is_children_collector: true,
                    prop_key: String::new(),
                    default_value: None,
                });
                continue;
            }

            // Regular capture: "name:type", "name:type?", or "name:type+"
            let (modifier, token) = if token.ends_with('?') {
                (CaptureModifier::Optional, &token[..token.len() - 1])
            } else if token.ends_with('+') {
                (CaptureModifier::OneOrMore, &token[..token.len() - 1])
            } else if token.ends_with('*') {
                (CaptureModifier::ZeroOrMore, &token[..token.len() - 1])
            } else {
                (CaptureModifier::Required, token)
            };

            if let Some((name, type_name)) = token.split_once(':') {
                specs.push(BodyCaptureSpec {
                    name: name.to_string(),
                    type_name: type_name.to_string(),
                    modifier,
                    is_children_collector: false,
                    prop_key: String::new(),
                    default_value: None,
                });
            }
        }
    }

    specs
}

fn collect_descendant_tokens(node: &ChildNode, out: &mut Vec<TokenData>) {
    out.extend(node.tokens.iter().cloned());
    for child in &node.children {
        collect_descendant_tokens(child, out);
    }
}

/// Parse a component body into structured sections.
///
/// Detects content type by leading characters at the start of lines:
/// - `<tag` → HTML block
/// - `$name type:` → state declaration
/// - `@exports` → export declaration
/// - `.selector {` or `[attr] {` or `#id {` → CSS rule / behavior block
/// - `@directive` → behavioral directive
/// - `.class: $` → reactive class toggle
/// - `text <-` or `[slot] <-` → content injection
///
/// For backward compatibility: if no non-HTML sections are detected,
/// A logical segment of a component body — one construct, regardless of how many
/// source lines it spans. Produced by `segment_component_body`, which scans the
/// raw body char-by-char tracking HTML tag depth, brace depth, and string state
/// so that constructs are split at their true boundaries (not at newlines).
///
/// This replaces the previous line-atomic classifier, which could neither split
/// a single line holding `$decl; <html>` (HTML or decl was silently dropped) nor
/// keep a `.sel { @on &.click { $x <- v } }` behavior block from being misread as a
/// content injection (the inner `<-` matched the injection probe). See BUG-059.
#[derive(Debug, Clone)]
struct BodySegment<'a> {
    /// The verbatim slice of the body for this construct (trimmed of outer ws).
    text: &'a str,
    /// True when the segment is an HTML run (passed through to the `html` field).
    is_html: bool,
}

/// Scan a component body into `BodySegment`s. Non-HTML constructs are recognized
/// only at HTML-tag-depth 0 AND brace-depth 0, so markup containing `<`, `>`,
/// `{`, `}`, or `<-` in text/attributes never desyncs the scanner.
///
/// Construct starts (at depth 0), by leading token after trivia:
///   `$name <type>...;`         state declaration (terminated by `;`)
///   `@exports { ... }`         exports block (brace-balanced)
///   `@name ... { ... }`        directive block (brace-balanced)  e.g. `@on`, `@each`
///   `@name ...;`               single-line directive (terminated by `;`)
///   `.sel { ... }` / `[..] {`  CSS-or-behavior block (brace-balanced)
///   `#id { ... }`              CSS block (brace-balanced)
///   `&...`                     template/selector ref (block if it has `{`, else `;`/line)
///   `target <- expr;`          content injection (terminated by `;`)
///   `.cls: $sig;`              reactive class toggle (terminated by `;`)
/// Anything else at depth 0 is HTML and accumulates until the next construct.
fn segment_component_body(inner: &str) -> Vec<BodySegment<'_>> {
    let bytes = inner.as_bytes();
    let n = bytes.len();
    let mut segments: Vec<BodySegment> = Vec::new();
    let mut html_start: Option<usize> = None;
    let mut i = 0usize;
    let mut tag_depth: i32 = 0;

    // Helper: push any accumulated HTML run ending at `end`.
    macro_rules! flush_html {
        ($end:expr) => {{
            if let Some(s) = html_start.take() {
                let slice = inner[s..$end].trim();
                if !slice.is_empty() {
                    segments.push(BodySegment {
                        text: slice,
                        is_html: true,
                    });
                }
            }
        }};
    }

    while i < n {
        let b = bytes[i];
        // Whitespace at depth 0 with no open HTML run: skip (don't start an HTML run on ws).
        if tag_depth == 0 && html_start.is_none() && b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Inside HTML (tag depth > 0) we only track tag open/close to find depth 0 again.
        if tag_depth > 0 {
            if b == b'<' {
                i = scan_html_tag(inner, bytes, i, &mut tag_depth);
            } else {
                i += 1;
            }
            continue;
        }

        // At tag depth 0: an HTML tag continues/starts an HTML run.
        if b == b'<' {
            if html_start.is_none() {
                html_start = Some(i);
            }
            i = scan_html_tag(inner, bytes, i, &mut tag_depth);
            continue;
        }

        // At tag depth 0 a construct may begin even if an HTML run is open (e.g. a
        // behavior block or a state decl AFTER the root element, or BEFORE it). We are
        // between top-level elements here (depth 0), so a recognized construct-start
        // token genuinely begins a construct; inline interpolation like `$x` lives at
        // depth > 0 (inside a tag) and is never reached by this branch. When a construct
        // is recognized we FLUSH the pending HTML run (trimmed) before emitting it.
        if let Some(kind) = classify_construct_start(inner, i) {
            flush_html!(i);
            let end = match kind {
                ConstructEnd::Semicolon => scan_to_semicolon(bytes, i, n),
                ConstructEnd::SemicolonOrLine => scan_to_semicolon_or_line(bytes, i, n),
                ConstructEnd::BraceBlock => scan_brace_block(bytes, i, n),
                ConstructEnd::RefLine => scan_ref_end(inner, bytes, i, n),
            };
            let slice = inner[i..end].trim();
            if !slice.is_empty() {
                segments.push(BodySegment {
                    text: slice,
                    is_html: false,
                });
            }
            i = end;
            continue;
        }

        // Default: HTML content (text, interpolation, entities, etc.).
        if html_start.is_none() {
            html_start = Some(i);
        }
        i += 1;
    }
    flush_html!(n);
    segments
}

/// How a construct terminates.
#[derive(Debug, Clone, Copy)]
enum ConstructEnd {
    /// Terminated by the next `;` at brace depth 0.
    Semicolon,
    /// Terminated by the next `;` at brace depth 0 OR end of line, whichever first.
    /// Used for state declarations (always single-line) so a broken decl without `;`
    /// does not swallow the following construct.
    SemicolonOrLine,
    /// Terminated by the matching `}` of the first `{`.
    BraceBlock,
    /// A `&`-ref: a brace block if it contains `{` before `;`/newline, else to `;`/newline.
    RefLine,
}

/// Decide whether a construct begins at byte offset `i` (which is at tag depth 0
/// and at the start of a depth-0 region). Returns how it terminates, or None for HTML.
fn classify_construct_start(inner: &str, i: usize) -> Option<ConstructEnd> {
    // A construct never begins mid-multibyte-char: every trigger ($ @ . # [ & and the `<-`
    // arrow) is ASCII, and UTF-8 continuation bytes (>=0x80) are never a trigger. If `i` (a
    // byte index the segmenter advances byte-by-byte through HTML content) lands inside a
    // multibyte char, slicing `&inner[i..]` would panic -- so bail to HTML content and let the
    // byte loop step to the next boundary (BUG-062).
    if !inner.is_char_boundary(i) {
        return None;
    }
    let rest = &inner[i..];
    let b = rest.as_bytes()[0];
    match b {
        b'$' => {
            // `${` is HTML interpolation, not a state decl.
            if rest.as_bytes().get(1) == Some(&b'{') {
                return None;
            }
            // `$item.prop` (dotted, no type) is HTML interpolation, not a construct.
            // A bare `$ident` that is neither interpolation nor a valid `$name type:`
            // decl is an ATTEMPTED-but-broken construct: surface it as a Semicolon
            // segment so the classifier can flag E0900 (don't silently absorb as HTML).
            // A state decl is single-line, so bound `head` at the first `;` OR newline
            // (whichever is first) — otherwise a bare `$broken` would swallow the next
            // line's `$count number: 0;` and mis-read its `$` as an operator value.
            let head = {
                let semi = rest.find(';').unwrap_or(rest.len());
                let nl = rest.find('\n').unwrap_or(rest.len());
                &rest[..semi.min(nl)]
            };
            let first_token = head
                .trim()
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("");
            // Interpolation like `$item.name` or bare `$x` embedded in text: a dotted
            // path is interpolation. A lone `$x` with following type is a decl; a lone
            // `$x` alone is a broken decl (E0900). Treat dotted-only as HTML.
            if first_token.contains('.') {
                return None;
            }
            // A `$ident` that is the LHS of an expression value (e.g. `$count * 2`, the
            // RHS of `text: $count * 2`) is NOT a construct start — only `$name <type>`
            // (decl) or a bare `$name` (broken decl -> E0900) is. Distinguish by what
            // follows the first token: a type word (alnum) or `:`/`;`/end means decl;
            // an operator (`*`, `+`, `-`, `<`, `(`, etc.) means it's an expression value
            // that belongs to the surrounding HTML/property, so leave it as HTML.
            let after_first = head.trim()[first_token.len()..].trim_start();
            let next_ch = after_first.chars().next();
            match next_ch {
                // Decl with a type word, or a `:` form, or a bare `$name` (broken decl).
                None => Some(ConstructEnd::SemicolonOrLine),
                Some(c) if c.is_ascii_alphabetic() => Some(ConstructEnd::SemicolonOrLine),
                Some(':') => Some(ConstructEnd::SemicolonOrLine),
                // Anything else (operator, paren, etc.) -> expression value, stays HTML.
                _ => None,
            }
        }
        b'@' => {
            // Directives, but NOT @if/@else (template control-flow handled elsewhere). This is
            // pure TERMINATOR detection (block vs `;`-line) — construct-agnostic: no directive is
            // NAMED here. `@exports`/`@on`/`@match`/`@media` all fall through to the generic
            // brace-vs-semicolon rule below (e.g. `@exports { }` has a `{` before any `;` ->
            // BraceBlock). The construct's identity is decided later by the stdlib grammar.
            if rest.starts_with("@if") || rest.starts_with("@else") {
                return None;
            }
            // @name... : block if a `{` appears before a `;`; else a `;`-terminated line.
            if rest.len() > 1 && rest.as_bytes()[1].is_ascii_alphabetic() {
                if brace_before_semicolon(rest) {
                    Some(ConstructEnd::BraceBlock)
                } else {
                    Some(ConstructEnd::Semicolon)
                }
            } else {
                None
            }
        }
        b'.' | b'#' | b'[' => {
            // `.cls: $sig;` reactive class toggle (no brace) vs `.sel { ... }` block.
            // `[slot] <- $x;` injection vs `[attr] { ... }` block.
            if brace_before_semicolon(rest) {
                Some(ConstructEnd::BraceBlock)
            } else {
                // No brace: a `;`-terminated construct (class toggle, injection, etc.).
                Some(ConstructEnd::Semicolon)
            }
        }
        b'&' => {
            // Ref — but `&lt;` / `&#...` are HTML entities, treat as HTML.
            if rest.starts_with("&lt;")
                || rest.starts_with("&#")
                || rest.starts_with("&amp;")
                || rest.starts_with("&gt;")
                || rest.starts_with("&quot;")
            {
                return None;
            }
            Some(ConstructEnd::RefLine)
        }
        _ => {
            // A bare `target <- expr;` injection where target is an identifier (e.g. `text`,
            // `src`, `value`). Only treat as injection if a ` <- ` occurs before any `<`
            // (so HTML like `a < b` stays HTML) and before the next `;`.
            if arrow_before_tag_or_semicolon(rest) {
                Some(ConstructEnd::Semicolon)
            } else {
                None
            }
        }
    }
}

/// True if a `{` occurs before the next `;` (both at the top level of `rest`).
fn brace_before_semicolon(rest: &str) -> bool {
    for &c in rest.as_bytes() {
        match c {
            b'{' => return true,
            b';' => return false,
            _ => {}
        }
    }
    false
}

/// True if ` <- ` occurs before the next `<` (HTML tag) or `;` or `{`.
fn arrow_before_tag_or_semicolon(rest: &str) -> bool {
    let b = rest.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b';' | b'{' => return false,
            b'<' if i + 1 < b.len() && b[i + 1] == b'-' => return true,
            b'<' => return false, // an HTML tag starts before any arrow
            _ => {}
        }
        i += 1;
    }
    false
}

/// Scan a single HTML tag starting at `i` (byte is `<`). Updates `tag_depth`:
/// opening tag → +1, closing tag → -1, void/self-closing → 0. Returns index past `>`.
fn scan_html_tag(inner: &str, bytes: &[u8], i: usize, tag_depth: &mut i32) -> usize {
    let n = bytes.len();
    // Comments / doctype: `<!-- ... -->` or `<!...>`
    if i + 1 < n && bytes[i + 1] == b'!' {
        if let Some(end) = inner[i..].find("-->") {
            return i + end + 3;
        }
        if let Some(end) = inner[i..].find('>') {
            return i + end + 1;
        }
        return n;
    }
    let is_close = i + 1 < n && bytes[i + 1] == b'/';
    // Find the matching `>` (HTML tags don't contain `>` except inside attr quotes).
    let mut j = i + 1;
    let mut in_q: Option<u8> = None;
    while j < n {
        let c = bytes[j];
        match in_q {
            Some(q) => {
                if c == q {
                    in_q = None;
                }
            }
            None => match c {
                b'"' | b'\'' => in_q = Some(c),
                b'>' => break,
                _ => {}
            },
        }
        j += 1;
    }
    let gt = j.min(n);
    if gt >= n {
        return n;
    }
    // tag text between `<` and `>` (exclusive)
    let tag_inner = &inner[i + 1..gt];
    let self_closing = tag_inner.trim_end().ends_with('/');
    if is_close {
        *tag_depth = (*tag_depth - 1).max(0);
    } else if !self_closing {
        // Extract tag name to skip void elements.
        let name = tag_inner
            .trim_start_matches('/')
            .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        const VOID: &[&str] = &[
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];
        if !VOID.contains(&name.as_str()) {
            *tag_depth += 1;
        }
    }
    gt + 1
}

/// Scan from `i` to just past the next `;` at brace depth 0 (or end).
fn scan_to_semicolon(bytes: &[u8], i: usize, n: usize) -> usize {
    let mut j = i;
    let mut depth: i32 = 0;
    let mut in_q: Option<u8> = None;
    while j < n {
        let c = bytes[j];
        match in_q {
            Some(q) => {
                if c == q {
                    in_q = None;
                }
            }
            None => match c {
                b'"' | b'\'' => in_q = Some(c),
                b'{' | b'(' | b'[' => depth += 1,
                b'}' | b')' | b']' => depth -= 1,
                b';' if depth <= 0 => return j + 1,
                _ => {}
            },
        }
        j += 1;
    }
    n
}

/// Scan from `i` to the next `;` at brace depth 0 OR the end of the current line
/// (newline), whichever comes first. Used for single-line constructs (state decls)
/// so a malformed one without `;` cannot swallow the following construct/HTML.
fn scan_to_semicolon_or_line(bytes: &[u8], i: usize, n: usize) -> usize {
    let mut j = i;
    let mut depth: i32 = 0;
    let mut in_q: Option<u8> = None;
    while j < n {
        let c = bytes[j];
        match in_q {
            Some(q) => {
                if c == q {
                    in_q = None;
                }
            }
            None => match c {
                b'"' | b'\'' => in_q = Some(c),
                b'{' | b'(' | b'[' => depth += 1,
                b'}' | b')' | b']' => depth -= 1,
                b';' if depth <= 0 => return j + 1,
                b'\n' if depth <= 0 => return j,
                _ => {}
            },
        }
        j += 1;
    }
    n
}

/// Scan from `i` (which precedes a `{` somewhere ahead) to just past the matching `}`.
fn scan_brace_block(bytes: &[u8], i: usize, n: usize) -> usize {
    let mut j = i;
    let mut depth: i32 = 0;
    let mut seen_open = false;
    let mut in_q: Option<u8> = None;
    while j < n {
        let c = bytes[j];
        match in_q {
            Some(q) => {
                if c == q {
                    in_q = None;
                }
            }
            None => match c {
                b'"' | b'\'' => in_q = Some(c),
                b'{' => {
                    depth += 1;
                    seen_open = true;
                }
                b'}' => {
                    depth -= 1;
                    if seen_open && depth <= 0 {
                        return j + 1;
                    }
                }
                _ => {}
            },
        }
        j += 1;
    }
    n
}

/// Scan a `&`-ref construct: a brace block if a `{` precedes the next `;`/newline,
/// otherwise to the next `;` or end of line.
fn scan_ref_end(inner: &str, bytes: &[u8], i: usize, n: usize) -> usize {
    // Determine if a brace precedes a newline or `;`.
    let mut j = i;
    while j < n {
        match bytes[j] {
            b'{' => return scan_brace_block(bytes, i, n),
            b';' => return j + 1,
            b'\n' => break,
            _ => {}
        }
        j += 1;
    }
    // Single-line ref without brace or `;`: consume to end of line.
    let _ = inner;
    let mut k = i;
    while k < n && bytes[k] != b'\n' {
        k += 1;
    }
    k
}

/// the entire content is returned as the `html` field, identical to HtmlBlock.

/// Phase B (FEAT-079): the SELF-DESCRIBING component-body producer. Replaces the per-construct
/// Rust classifier in `parse_component_body` with the stdlib `component_item` %capture_type
/// grammar + a generic reifier. The generic `segment_component_body` scanner (KEEP) splits the
/// body into HTML runs + construct segments; each construct segment is classified by the
/// stdlib grammar (which branch matched) and its coarse fields extracted by the engine; this
/// reifier maps each branch record into the typed `ComponentBodyDef` IR using value-semantics
/// adapters (typed initial conversion, @on action-kind detection, template-call arg split) that
/// operate ONLY on the grammar's pre-split fields — NO construct grammar lives in Rust.
///
/// Differential-tested byte-identical against `parse_component_body` over the Phase B corpus
/// (`component_body_corpus::CORPUS`) before the old path is deleted (B3f).
/// Validate a component body, emitting canonical [`Diagnostic`]s (FEAT-120).
///
/// Body validation is the one lint pass that runs at PARSE time over the raw body
/// text (the 11 pipeline `check_*` passes read World A at compile time). FEAT-120
/// retired the `ComponentBodyDef` capture envelope that used to smuggle these lints
/// forward: the validator now returns `Vec<Diagnostic>` directly, the caller
/// (`cst_to_stfile`) pushes them into `StFile.diagnostics`, and the pipeline drains
/// that into `output.diagnostics` in ONE extend.
///
/// `base_offset` is the absolute byte offset of `inner` within the source file, so
/// each diagnostic carries a real span (each segment is a subslice of `inner`).
/// `name` is the enclosing template's name — needed for the W0700 interleave warning,
/// which is now emitted HERE (it has the transition count + the name locally) rather
/// than re-derived in a pipeline drain.
///
/// The body's structured payload (html/states/exports/refs) is produced on the
/// World-A path in `cst_to_stfile`; this function emits ONLY diagnostics.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn validate_component_body(
    inner: &str,
    base_offset: usize,
    name: &str,
) -> Vec<crate::diagnostics::Diagnostic> {
    validate_component_body_with_registry(inner, base_offset, name, None)
}

/// Like [`validate_component_body`], but registry-aware: when `registry` is
/// `Some`, an unmatched root `@name` directive that ISN'T a known form in the
/// registry at all (zero forms for `@name`, mirroring the top-level `W0714`
/// "not a known directive" check in `src/parser/mod.rs`) surfaces E0900
/// instead of being silently consumed (BUG-167). A directive the registry DOES
/// know (`@effect`, `@portal`, `@fn`, any macro not modeled as a
/// `component_item` arm) is legitimate body content matched by the wider
/// macro-form system elsewhere — NOT an error here, so it stays silent exactly
/// as before. `registry: None` (bootstrap / test callers with no registry in
/// hand) preserves the OLD fully-permissive behavior — this is an additive
/// tightening, not a behavior change for callers that can't supply a registry.
pub(crate) fn validate_component_body_with_registry(
    inner: &str,
    base_offset: usize,
    name: &str,
    registry: Option<&crate::syntax::registry::SyntaxRegistry>,
) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode as DC};
    use crate::syntax::form_match::CapturedValue as CV;

    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Fast path: pure HTML (no construct sections at all) — nothing to validate.
    if !has_component_body_sections(inner) {
        return diagnostics;
    }

    // Span of a segment slice (a subslice of `inner`) in absolute source bytes.
    let span_of = |s: &str| -> crate::diagnostics::SourceSpan {
        let off = (s.as_ptr() as usize).saturating_sub(inner.as_ptr() as usize);
        crate::diagnostics::SourceSpan::new(base_offset + off, base_offset + off + s.len())
    };

    let item_ext = component_item_extractor();
    let segments = segment_component_body(inner);
    // Section-transition tracking for the W0700 interleave lint. The CST reconstruct
    // is the sole html source; this just counts HTML↔CSS interleaves.
    let mut last_section: u8 = 0; // 0=None 1=Html 2=Css
    let mut section_transitions: u32 = 0;

    for seg in segments.iter() {
        if seg.is_html {
            if last_section == 2 {
                section_transitions += 1;
            }
            last_section = 1;
            continue;
        }
        let stext = seg.text;
        let branch = item_ext
            .as_ref()
            .and_then(|ext| classify_named_item(ext.as_ref(), stext));
        match branch {
            Some((kind, rec)) => match kind.as_str() {
                // PURE-CONSUME arms — each is produced on the World-A path, so the
                // validator builds no payload and emits no lint for them. (See the
                // FEAT-115 notes: state/exports/self_prop/inject/toggle/match/selector_ref
                // all surface as World-A scopes/matches.) `media` (BUG-167) joins this
                // list: its CSS hoist happens in src/parser/mod.rs's template-scope
                // construction loop (file.raw_css_blocks), not here — this validator is
                // diagnostics-only and never built CSS output for ANY construct.
                "state" | "exports" | "self_prop" | "inject" | "toggle" | "match" | "media"
                | "selector_ref" => {
                    let _ = &rec;
                }
                "root_on" => {
                    // A body-root `@on` surfaces as a World-A `on` match; the parse runs
                    // only for its DIAGNOSTIC side-effects (E0906 malformed-handler).
                    if let Some(CV::Expr(tail)) = rec.get("tail") {
                        let on_text = format!("@on {}", tail);
                        let _ = parse_on_event_block(&on_text, &mut diagnostics, span_of(stext));
                    }
                }
                "block" => {
                    // A `.sel { … }` block. Validate it (E0906 for a malformed scoped
                    // `@on` inside) and track the HTML↔CSS section transition for W0700.
                    let found = reify_block(&rec, &mut diagnostics, span_of(stext));
                    if !found {
                        if last_section == 1 {
                            section_transitions += 1;
                        }
                        last_section = 2;
                    }
                }
                "template_ref" => {
                    // Surface E0900 for a MALFORMED ref (`&broken` with no `(args)`) so a
                    // broken construct is never silently swallowed — UNLESS it is an
                    // ENTITY SCOPE (`&name { @c }`: an ident, no `(args)`, a `{ body }`).
                    // FEAT-142 Wave E: entity scopes are valid nested construct-body
                    // content (they lower to a `.st-entity-<name>` selector scope +
                    // registration), so they must NOT trip E0900.
                    if parse_template_ref(stext.trim()).is_none()
                        && !is_entity_scope_segment(stext.trim())
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                DC::E0900,
                                format!("Unrecognized content in component body: `{}`", stext.trim()),
                            )
                            .with_span(span_of(stext.trim()))
                            .with_hint("Valid content: HTML elements (<tag>), state declarations ($var type: value), CSS rules (.selector { }), directives (@name), content injections (target <- $var), exports (@exports { $var }), entity scopes (&name { @component })"),
                        );
                    }
                }
                _ => {}
            },
            None => {
                // Unclassifiable construct segment: E0900 / HTML-fallback split.
                let ltrim = stext.trim();
                let is_directive = ltrim.starts_with('@')
                    && ltrim.len() > 1
                    && ltrim.as_bytes()[1].is_ascii_alphabetic();
                if is_directive {
                    // BUG-167: previously an UNCONDITIONAL silent drop for ANY `@name`
                    // no component_item arm matched ("parity" with the old reify
                    // classifier). That swallowed real typos with zero feedback. Now:
                    // only surface E0900 when the registry confirms `@name` is not a
                    // known directive AT ALL (mirrors the top-level `W0714` "not a
                    // known directive" check) — a directive the registry KNOWS about
                    // (`@effect`, `@portal`, `@fn`, any macro matched by the wider
                    // macro-form system rather than a component_item arm) stays
                    // legitimately silent here; only a GENUINE unknown name errors.
                    // `registry: None` (bootstrap/tests) keeps the fully-permissive
                    // old behavior — there is nothing to check against.
                    let dir_name = ltrim
                        .trim_start_matches('@')
                        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                        .next()
                        .unwrap_or("");
                    let unknown = registry.is_some_and(|r| {
                        !dir_name.is_empty() && r.get_forms_for_directive(dir_name).is_empty()
                    });
                    if unknown {
                        diagnostics.push(
                            Diagnostic::error(
                                DC::E0900,
                                format!("`@{}` is not a known directive in a @template body", dir_name),
                            )
                            .with_span(span_of(ltrim))
                            .with_hint("check the spelling against the stdlib registry, or import the module that defines it".to_string()),
                        );
                    }
                } else if let Some(bname) = backtick_decl_name(ltrim) {
                    // A state declaration mis-written with the HOLE form (`` `$x` bool: false ``).
                    diagnostics.push(
                        Diagnostic::error(
                            DC::E0900,
                            format!("a `` `${0}` `` hole cannot be a state declaration", bname),
                        )
                        .with_span(span_of(ltrim))
                        .with_hint(format!(
                            "Drop the backticks — a declaration uses the bare name: `${0} <type>: <value>;` (e.g. `${0} bool: false;`). Backtick `` `${0}` `` is the hole/interpolation form, valid only in HTML.",
                            bname
                        )),
                    );
                } else if looks_like_invalid_construct(ltrim) {
                    diagnostics.push(
                        Diagnostic::error(
                            DC::E0900,
                            format!("Unrecognized content in component body: `{}`", ltrim),
                        )
                        .with_span(span_of(ltrim))
                        .with_hint("Valid content: HTML elements (<tag>), state declarations ($var type: value), CSS rules (.selector { }), directives (@name), content injections (target <- $var), exports (@exports { $var })"),
                    );
                } else {
                    // Looks like HTML, not an invalid construct: no diagnostic.
                }
            }
        }
    }

    // W0700 (FEAT-120): emitted HERE — the validator has the transition count AND the
    // template name locally, so the old pipeline drain (check_interleaved_html_css) is
    // retired. More than one HTML↔CSS transition means the sections interleave.
    if section_transitions > 1 {
        diagnostics.push(
            Diagnostic::warning(
                DC::W0700,
                format!("Interleaved HTML and CSS sections in template &{}", name),
            )
            .with_span(crate::diagnostics::SourceSpan::new(
                base_offset,
                base_offset + inner.len(),
            ))
            .with_hint(
                "Group all HTML content together, then all CSS rules. This improves readability.",
            ),
        );
    }

    diagnostics
}

// ===========================================================================================
// Phase B reifier support (FEAT-079): grammar-record -> ComponentBodyDef value-semantics
// adapters. Each operates ONLY on the stdlib `component_item` grammar's pre-split fields. The
// dispatch key is the GRAMMAR branch label (not a hardcoded construct grammar) — the same
// reifier-boundary pattern as reify_params / reify_param_list.
// ===========================================================================================
use std::collections::HashMap as ReifyMap;
type ItemRecord = ReifyMap<String, crate::syntax::form_match::CapturedValue>;

/// Build the `component_item` extractor once from the live stdlib registry. Returns None if the
/// grammar is unavailable (e.g. stdlib not loaded) so the caller can fall back.
fn component_item_extractor() -> Option<Box<dyn super::extractors::CaptureExtractor>> {
    named_choice_extractor("component_item")
}

/// Build the `cb_block_item` extractor (the scoped-construct Choice for `.sel { }` block inners).
fn cb_block_item_extractor() -> Option<Box<dyn super::extractors::CaptureExtractor>> {
    named_choice_extractor("cb_block_item")
}

/// Build an extractor for a stdlib `%capture_type` Choice-of-named-arms grammar by name. Returns
/// None if the grammar is unavailable (stdlib not loaded) so callers can fall back.
fn named_choice_extractor(name: &str) -> Option<Box<dyn super::extractors::CaptureExtractor>> {
    use super::extractors::ExtractorRegistry;
    use super::extractors::custom::{compile_pattern_with_defs, wrap_reifier};
    use crate::parser::meta_ast::CaptureTypeDefAst;
    let reg = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
    let defs: ReifyMap<String, CaptureTypeDefAst> = reg
        .capture_types()
        .map(|c| (c.name.clone(), c.clone()))
        .collect();
    let ct = defs.get(name)?;
    Some(wrap_reifier(
        name,
        compile_pattern_with_defs(
            &ct.pattern,
            &ExtractorRegistry::new(),
            &defs,
            &mut Vec::new(),
        ),
    ))
}

/// Run the `component_item` grammar over a single CONSTRUCT segment. Returns the matched branch
/// label + its field record, ONLY when ALL tokens are consumed (a partial consume means the
/// grammar could not fully classify the segment — treated as unclassified so the caller can
/// surface E0900 / HTML-fallback, never silently truncate).
/// Run a Choice-of-named-arms grammar over a single segment. Returns the matched branch label +
/// its field record, ONLY when ALL tokens are consumed (a partial consume means the grammar
/// could not fully classify the segment — treated as unclassified so the caller can surface
/// E0900 / fallback, never silently truncate). Shared by component_item (top level) and
/// Apply a body-capture extractor honoring its repeat modifier (BUG: repeated CUSTOM
/// %capture_types like `match_arm+` previously single-extracted and dropped the rest).
/// Required/Optional -> one extract; ZeroOrMore/OneOrMore -> accumulate into an Array,
/// skipping inter-item WHITESPACE/SEMICOLON/COMMA separators. ONE repeat implementation
/// shared by the builtin and Custom body-capture branches.
fn extract_with_modifier(
    ext: &dyn super::extractors::CaptureExtractor,
    tokens: &[super::extractors::TokenData],
    source: &str,
    modifier: crate::parser::meta_ast::CaptureModifier,
) -> Option<crate::syntax::form_match::CapturedValue> {
    use crate::parser::meta_ast::CaptureModifier;
    use crate::syntax::cst::SyntaxKind;
    use crate::syntax::form_match::CapturedValue;
    match modifier {
        // Counted is a single extraction — the count lives inside the char-class
        // extractor, which consumes the whole run in one call.
        CaptureModifier::Required | CaptureModifier::Optional | CaptureModifier::Counted(_) => {
            ext.extract(tokens, source).map(|(v, _)| v)
        }
        CaptureModifier::ZeroOrMore | CaptureModifier::OneOrMore => {
            let mut values = Vec::new();
            let mut consumed_total = 0usize;
            while consumed_total < tokens.len() {
                while consumed_total < tokens.len()
                    && (tokens[consumed_total].kind == SyntaxKind::WHITESPACE
                        || tokens[consumed_total].kind == SyntaxKind::SEMICOLON
                        || tokens[consumed_total].kind == SyntaxKind::COMMA)
                {
                    consumed_total += 1;
                }
                if consumed_total >= tokens.len() {
                    break;
                }
                let remaining = &tokens[consumed_total..];
                match ext.extract(remaining, source) {
                    Some((value, consumed)) if consumed > 0 => {
                        values.push(value);
                        consumed_total += consumed;
                    }
                    _ => break,
                }
            }
            // BUG-229 (repeat path). Stopping at the first element that fails to
            // extract is only correct if nothing meaningful is LEFT. Returning the
            // prefix collected so far silently discards the remainder: with
            // `$arms:on_arm+`, a valid arm followed by a MALFORMED one captured just
            // the valid arm, the form matched, and `check` went green while the
            // second arm vanished from the program.
            //
            // That is the same silent-drop class the E0946 gate exists to close, one
            // level down: there the whole directive disappeared, here one element of
            // a repeat does. A repeat that halts with unconsumed non-trivia input has
            // NOT matched its input, so the capture fails and the caller reports it.
            let mut rest = consumed_total;
            while rest < tokens.len()
                && matches!(
                    tokens[rest].kind,
                    SyntaxKind::WHITESPACE | SyntaxKind::SEMICOLON | SyntaxKind::COMMA
                )
            {
                rest += 1;
            }
            if rest < tokens.len() {
                return None;
            }
            if matches!(modifier, CaptureModifier::OneOrMore) && values.is_empty() {
                None
            } else {
                Some(CapturedValue::Array(values))
            }
        }
    }
}

/// cb_block_item (block inner).
fn classify_named_item(
    ext: &dyn super::extractors::CaptureExtractor,
    segment: &str,
) -> Option<(String, ItemRecord)> {
    use super::extractors::TokenData;
    use crate::syntax::form_match::CapturedValue as CV;
    let tokens: Vec<TokenData> = crate::syntax::cst::lexer::Lexer::new(segment)
        .tokenize()
        .into_iter()
        .filter(|t| t.kind != crate::syntax::cst::SyntaxKind::EOF)
        .map(|t| TokenData {
            kind: t.kind,
            text_range: (t.offset, t.offset + t.len()),
        })
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let (value, consumed) = ext.extract(&tokens, segment)?;
    if consumed != tokens.len() {
        return None;
    }
    // A Choice of single named arms -> Named{ <branch> : record }.
    let CV::Named(outer) = value else {
        return None;
    };
    let (branch, inner) = outer.into_iter().next()?;
    let rec = match inner {
        CV::Named(m) => m,
        // A single-field arm whose body is a leaf (rare) -> wrap under its own key.
        other => {
            let mut m = ReifyMap::new();
            m.insert(branch.clone(), other);
            m
        }
    };
    Some((branch, rec))
}

/// Get a field as trimmed text (Binding/Expr/Ident/String), preserving sigils for Binding.
fn rec_text(rec: &ItemRecord, key: &str) -> Option<String> {
    use crate::syntax::form_match::CapturedValue as CV;
    match rec.get(key) {
        Some(CV::Binding(s)) | Some(CV::Expr(s)) | Some(CV::Ident(s)) | Some(CV::Selector(s)) => {
            Some(s.trim().to_string())
        }
        Some(CV::String(s)) => Some(s.trim().to_string()),
        _ => None,
    }
}

/// Reify a `cb_self_prop` record { prop:Ident, value:Expr } -> ContentBinding (legacy &self form).

/// Reify a `cb_toggle` record { class:Ident, var:Binding } -> ClassToggle.
/// Reify a `cb_toggle` record { class:Ident, var:Expr (RHS after the literal `$`) } ->
/// ClassToggle. The grammar already consumed the leading `$`, so `var` is the bare condition
/// (`isActive`, `count > 0`, `.featured`) — kept verbatim to match parse_class_toggle's
/// `trim_start_matches('$')` semantics (BUG-063 #3).

/// Reify a `cb_block` record { sel:Selector, body:Expr }: re-segment the block body and dispatch
/// each inner construct (@on / class toggle / self-prop / content binding). Returns true when at
/// least one behavioral directive was found (so the caller does NOT emit it as a css_rule).
/// Mirrors parse_component_body's behavior-block branch (R-wave P1: mixed @on + siblings).
/// Reify a `cb_block` record { sel:Selector, body:Expr }: re-segment the block body and dispatch
/// each inner construct (@on / class toggle / self-prop / content binding). Returns true when at
/// least one behavioral directive was found (so the caller does NOT emit it as a css_rule).
/// Mirrors parse_component_body's behavior-block branch (R-wave P1: mixed @on + siblings).
/// Diagnostics (e.g. E0906 from a non-mutating scoped @on) flow into the REAL body diagnostics
/// vec for parity with parse_component_body (BUG-063 #1).
/// Reify a `cb_block` record { sel:Selector, body:Expr }: classify each block-inner construct
/// via the stdlib `cb_block_item` grammar (NOT Rust string-prefix dispatch — FEAT-080) and reify
/// scoped directives (@on / class toggle / self-prop / content binding). Returns true when at
/// least one behavioral directive was found (so the caller does NOT emit it as a css_rule).
/// Mirrors parse_component_body's behavior-block branch (R-wave P1: mixed @on + siblings).
/// Diagnostics (e.g. E0906 from a non-mutating scoped @on) flow into the REAL body diagnostics
/// vec for parity with parse_component_body (BUG-063 #1).
/// Split a `.sel { ... }` block inner into individual construct statements. A statement ends at
/// a depth-0 `;`, OR — for a brace-bearing construct like `@on &.click { ... }` — at the matching
/// `}` of its first `{`. Respects `()[]{}` nesting + double/single-quoted strings, so a `$signal`
/// inside an expression value (`text: currency($count);`, `text: 5 + $count;`) is NOT mistaken
/// for a new construct (the top-level segment_component_body treats `$x` as a construct start,
/// which is wrong INSIDE a block value — BUG-065). Returns trimmed, non-empty statements.
fn split_block_statements(inner: &str) -> Vec<&str> {
    let b = inner.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut start = 0usize;
    let mut depth: i32 = 0;
    let mut in_q: Option<u8> = None;
    let mut saw_brace = false;
    while i < n {
        let c = b[i];
        match in_q {
            Some(q) => {
                if c == q {
                    in_q = None;
                }
            }
            None => match c {
                b'"' | b'\'' => in_q = Some(c),
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                b'{' => {
                    depth += 1;
                    saw_brace = true;
                }
                b'}' => {
                    depth -= 1;
                    // The matching close of a construct's brace block (e.g. `@on { }`) at depth 0
                    // ends the statement.
                    if depth <= 0 && saw_brace {
                        let seg = inner[start..i + 1].trim();
                        if !seg.is_empty() {
                            out.push(seg);
                        }
                        i += 1;
                        start = i;
                        depth = 0;
                        saw_brace = false;
                        continue;
                    }
                }
                b';' if depth <= 0 => {
                    let seg = inner[start..i].trim();
                    if !seg.is_empty() {
                        out.push(seg);
                    }
                    i += 1;
                    start = i;
                    saw_brace = false;
                    continue;
                }
                _ => {}
            },
        }
        i += 1;
    }
    let tail = inner[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

fn reify_block(
    rec: &ItemRecord,
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
    span: crate::diagnostics::SourceSpan,
) -> bool {
    use crate::syntax::form_match::CapturedValue as CV;
    let Some(sel) = rec_text(rec, "sel") else {
        return false;
    };
    let Some(binner) = rec_text(rec, "body") else {
        return false;
    };
    let item_ext = cb_block_item_extractor();
    let mut found = false;
    for stmt in split_block_statements(&binner) {
        let it = stmt.trim();
        if it.is_empty() {
            continue;
        }
        // Classify the inner segment via the stdlib grammar (which scoped construct?).
        let branch = item_ext
            .as_ref()
            .and_then(|ext| classify_named_item(ext.as_ref(), it));
        // FEAT-115 S5c/FEAT-116: a `.sel{}` block's behavioral inner constructs are
        // World-A (the nested scope from convert_cst_scope renders them). reify no
        // longer BUILDS the `directives` payload; it only still CLASSIFIES each inner
        // segment to decide whether the block is BEHAVIORAL (`found` -> consumed, not a
        // css_rule) vs plain CSS (-> css_rule + section transition), and to run the
        // @on DIAGNOSTIC side-effects (E0906 malformed-handler).
        match branch.as_ref().map(|(k, _)| k.as_str()) {
            Some("on") => {
                if let Some(CV::Expr(tail)) = branch.as_ref().unwrap().1.get("tail") {
                    let on_text = format!("@on {}", tail);
                    if parse_on_event_block_scoped(&on_text, &sel, diagnostics, span) {
                        found = true;
                    }
                }
            }
            Some("toggle") | Some("self_prop") => {
                found = true;
            }
            Some("content") => {
                let rec = &branch.as_ref().unwrap().1;
                if let Some(val) = rec_text(rec, "val") {
                    // cb_content_bind matches ANY `prop: rhs` (incl. plain CSS). The original
                    // gate: a `$` ANYWHERE in the RHS makes it a reactive binding (behavioral,
                    // `found`); a plain decl (`color: red`, no `$`) stays a css_rule (BUG-065).
                    if val.contains('$') {
                        found = true;
                    }
                }
            }
            _ => {} // a non-behavioral inner segment (plain CSS decl) -> not a directive.
        }
    }
    found
}

/// Parse an `@match` segment via the STDLIB `match_block` %capture_type grammar (NOT a Rust
/// parser) and reify the engine record into a `ComponentMatch` (FEAT-073 Phase C). The arm
/// `inv` captures as a raw `&name(args)` expr, parsed by the existing `parse_template_ref`.
/// Returns None if the grammar is unavailable or the segment doesn't match.

/// Detect a state declaration mis-written with the HOLE form: `` `$name` <type>: <value> ``.
/// Backtick is the one hole/interpolation form and is never a declaration site
/// (docs §2), so this shape is always an authoring error — we recognize it to emit a
/// precise fix-it instead of the generic "unrecognized content" E0900. Returns the bare
/// name (no `$`) on a match. Does NOT match a plain HTML hole (`` `$title` `` with no
/// trailing `type:`), which stays literal text.
fn backtick_decl_name(line: &str) -> Option<String> {
    // The segmenter strips the OPENING backtick of a hole, so a body-root
    // `` `$navOpen` bool: false; `` arrives here as `$navOpen` bool: false;` —
    // a `$name` followed by a CLOSING backtick, then ` <type>:` (the decl signature).
    let s = line.trim();
    let rest = s.strip_prefix('$')?;
    let close = rest.find('`')?;
    let name = &rest[..close];
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let tail = &rest[close + 1..];
    let after = tail.trim_start();
    if after.len() == tail.len() {
        return None; // no whitespace separator after the hole — not a decl shape
    }
    let ty: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if ty.is_empty() {
        return None;
    }
    if after[ty.len()..].trim_start().starts_with(':') {
        Some(name.to_string())
    } else {
        None
    }
}

/// Check if a line at tag depth 0 looks like an invalid/broken Spacetime construct.
/// Returns true for lines that appear to be malformed state declarations,
/// broken directives, or other non-HTML content that the parser couldn't classify.
fn looks_like_invalid_construct(line: &str) -> bool {
    // Bare $identifier without type — likely broken state declaration
    // (valid state decls are caught earlier by is_state_declaration)
    if line.starts_with('$') && !line.starts_with("${") {
        return true;
    }
    // Bare & without matching template ref pattern — likely broken ref
    if line.starts_with('&') && !line.starts_with("&lt;") && !line.starts_with("&#") {
        return true;
    }
    false
}

/// Parse a state declaration string like "$open bool: false" into a ComponentStateDecl.

/// Extract the CSS selector from a block opening line.
/// e.g., `.child {` → Some(".child"), `[data-x] {` → Some("[data-x]")

/// Extract the inner content of a braced block (between first `{` and last `}`).

/// Parse a reactive class toggle: `.nav--open: $menuOpen;`
/// Optional `selector` targets a child element (e.g., `.child`) instead of root.

/// Parse a content injection: `selector <- $var` or `text <- $var | filter`
/// Split a `target <- value` statement on the arrow, tolerant of surrounding
/// whitespace (`a <- b`, `a<-b`, `a <-b` all split identically). Returns trimmed
/// (target, value). Whitespace tolerance matters because the CST emits a bare
/// LEFT_ARROW token regardless of spacing, so a no-space `[slot]<-$x` must route to
/// an injection rather than silently falling through to the HTML field (R-wave P3).

/// Parse a property binding: `&self.property <- $expr [| filter];`
///
/// Maps property names to DOM properties:
/// - `content` → `textContent`
/// - all others pass through as-is (href, src, alt, value, etc.)

/// Build a ContentBinding from a property name and a right-hand-side that is
/// either a bare `$signal` (optionally `| filter`) or an arbitrary expression
/// over signals (e.g. `$count * 2`, `$first + ' ' + $last`).
///
/// Bare single-signal bindings use the lightweight `var_name` fast path (no
/// `apply` arrow). Anything else is carried as an expression with its full
/// dependency set so the runtime watches every referenced signal.

/// As `make_content_binding`, but `allow_dotted_ref` controls whether a dotted
/// path like `$item.name` is treated as a single-signal reference (legacy
/// `&self.prop <- $item.name` content-injection form, where the leading
/// segment is an @each row binding) or as an expression (block form
/// `.sel { text: $signal.length }`, where the leading segment is a signal and
/// the `.length` is property access that must be evaluated).

/// Validate an `@on` event block: `@on &.click { $open <- !$open; }`. Returns `true`
/// when it is a well-formed @on directive (so the caller marks the block behavioral),
/// pushing any action-parse diagnostic (E0906) as a side-effect. FEAT-115 S5c: reify
/// no longer BUILDS a `ComponentDirective` — the @on renders via the World-A `on`
/// match — so this is now a validity probe + diagnostic pass, not a data producer.
fn parse_on_event_block(
    block: &str,
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
    span: crate::diagnostics::SourceSpan,
) -> bool {
    let block = block.trim();
    let Some(rest) = block.strip_prefix("@on") else {
        return false;
    };
    let rest = rest.trim();

    // Parse: "click { ... }" or "click(.selector) { ... }"
    let body = if let Some(brace_pos) = rest.find('{') {
        let body_end = rest.rfind('}').unwrap_or(rest.len());
        rest[brace_pos + 1..body_end].trim()
    } else {
        return false;
    };

    // Validate the action purely for its diagnostic side-effect (E0906 — missing
    // `$var <- expr`); the @on renders via the World-A `on` match.
    if let Some(d) = validate_on_action(body, span) {
        diagnostics.push(d);
    }
    true
}

/// Validate a scoped `@on` (inside a `.sel { }` block). FEAT-115 S5c: like
/// `parse_on_event_block`, a validity probe + diagnostic pass (no directive built;
/// the enclosing block selector + render are World-A). `block_selector` is unused
/// for data now but kept in the signature for call-site symmetry/diagnostics intent.
fn parse_on_event_block_scoped(
    on_text: &str,
    _block_selector: &str,
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
    span: crate::diagnostics::SourceSpan,
) -> bool {
    parse_on_event_block(on_text, diagnostics, span)
}

/// Validate an `@on` action body: `$open <- !$open;` OR a keyframe/transition
/// statement (`opacity: 0 -> 1;`, incl. an `& { ... }` scoped group), OR an
/// `@emit`/signal-method-call action. FEAT-115 S5c/S6: the action is rendered
/// by the World-A `on` match (the runtime evaluates its js_statements) — this
/// function is a DIAGNOSTIC-ONLY probe: it never builds the render payload,
/// it only decides whether the body is EMPTY of any recognized action.
///
/// BUG-166 fix: the body is legal iff it contains >=1 statement matching the
/// stdlib `on_action` grammar (mutation `$var <- expr`, keyframe `prop: a -> b`,
/// `@emit name`, or a `$signal.method(args)` call) — an assignment is only ONE
/// of several valid action kinds, not the only one. An animation-only body
/// (`@on visible { opacity: 0 -> 1; }`, or the same wrapped in a scoped
/// `& { ... }` group) is exactly as valid as a mutation-only body: both
/// already render correctly via the World-A path (verified: the js_statements
/// payload for a keyframe-only body is built independently of this function,
/// see the CaptureType::MutationActions arm above). This reuses the SAME
/// `stdlib/capture-types/on-actions.st::on_action` grammar the render path's
/// capture parsing is built from — no second bespoke Rust classifier.
fn validate_on_action(
    body: &str,
    span: crate::diagnostics::SourceSpan,
) -> Option<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::{Diagnostic, DiagnosticCode as DC};

    if on_body_has_action(body) {
        return None;
    }

    Some(
        Diagnostic::error(
            DC::E0906,
            format!("no state assignment found in @on body: {}", body),
        )
        .with_span(span)
        .with_hint("expected: $var <- expression; (or a keyframe: prop: a -> b;, or @emit name;)"),
    )
}

/// Does `body` contain at least one recognized `@on` action statement?
/// Tries the stdlib `on_action` Choice grammar (mutation / emit / signal-call /
/// keyframe) against each `;`-split statement; a statement that matches ANY
/// arm counts. A bare `& { ... }` scoped wrapper (BUG-166's original repro) is
/// unwrapped first — `on_action`/`keyframes` describe the INNER property
/// lines, not the wrapper syntax itself, so its contents are checked
/// directly. Falls back to the OLD strict (`<-`-only) scan if the stdlib
/// grammar is unavailable (defensive — should not happen once stdlib loads).
fn on_body_has_action(body: &str) -> bool {
    let trimmed = body.trim();
    // Unwrap a single top-level `& { ... }` scoped group (a self-referencing
    // element scope wrapping keyframe lines — stdlib/macros/template.st's own
    // documented "scoped animations" shape). Only strip when the WHOLE body is
    // one such wrapper so a mixed body (`$x <- 1; & { ... }`) still reaches the
    // per-statement scan below via its own `&`-prefixed segment.
    let unwrapped: &str = if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        if let Some(inner) = rest.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            inner
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    let Some(on_action_ext) = named_choice_extractor("on_action") else {
        // Defensive fallback — stdlib grammar unavailable. Preserve the OLD
        // strict behavior rather than silently accepting everything.
        return split_statements_outside_strings(unwrapped)
            .into_iter()
            .any(|stmt| stmt.contains("<-"));
    };

    split_statements_outside_strings(unwrapped)
        .into_iter()
        .any(|stmt| {
            use super::extractors::TokenData;
            let tokens: Vec<TokenData> = crate::syntax::cst::lexer::Lexer::new(stmt)
                .tokenize()
                .into_iter()
                .filter(|t| t.kind != crate::syntax::cst::SyntaxKind::EOF)
                .map(|t| TokenData {
                    kind: t.kind,
                    text_range: (t.offset, t.offset + t.len()),
                })
                .collect();
            if tokens.is_empty() {
                return false;
            }
            // A match is valid even if it doesn't consume every token (a keyframe
            // chain `opacity: 0 -> 1` fully consumes; a statement match at all
            // signals "this is a recognized action", which is all E0906 gates on).
            on_action_ext.extract(&tokens, stmt).is_some()
        })
}

/// Parse @exports block: `@exports { $count: mut; $label }`
pub(crate) fn parse_exports_block(
    block: &str,
    exports: &mut Vec<ExportDecl>,
    diagnostics: &mut Vec<crate::diagnostics::Diagnostic>,
    span: crate::diagnostics::SourceSpan,
) {
    use crate::diagnostics::{Diagnostic, DiagnosticCode as DC};
    // Strip @exports and braces
    let inner = block
        .trim()
        .strip_prefix("@exports")
        .unwrap_or(block)
        .trim();
    let inner = inner.strip_prefix('{').unwrap_or(inner).trim();
    let inner = inner.strip_suffix('}').unwrap_or(inner).trim();

    // Entries separate on EITHER `;` or `,` (BUG-296). The block is a LIST of
    // cells, and both spellings are taught: this fn's own doc-comment shows
    // `;`, while `@template`'s `@exports` (FEAT-115), the module-system docs and
    // every PLAN-117 fixture use `,`. Splitting on `;` alone did not reject a
    // comma list — it swallowed it: `{ $host, $session }` parsed as ONE entry
    // named `host, session`, which then failed the identifier check as
    // "Invalid export variable name" pointing at the whole line. A single-cell
    // block (`{ $host }`) has no separator at all, so it kept working and hid
    // this for the entire multi-export surface.
    for part in inner.split([';', ',']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // Valid export entries must start with $
        if !part.starts_with('$') {
            diagnostics.push(
                Diagnostic::error(DC::E0904, format!("Malformed export declaration: `{}`", part))
                    .with_span(span)
                    .with_hint("Export entries must start with $. Correct syntax: @exports { $varName } or @exports { $varName: mut }"),
            );
            continue;
        }
        let var_part = part.trim_start_matches('$');
        let (var_name, mutable) = if let Some((name, rest)) = var_part.split_once(':') {
            (name.trim().to_string(), rest.trim() == "mut")
        } else {
            (var_part.trim().to_string(), false)
        };
        // Validate variable name is a valid identifier
        if var_name.is_empty() || !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            diagnostics.push(
                Diagnostic::error(DC::E0904, format!("Invalid export variable name: `${}`", var_part.trim()))
                    .with_span(span)
                    .with_hint("Export variable names must be valid identifiers (alphanumeric and underscore). Example: @exports { $myVar }"),
            );
            continue;
        }
        exports.push(ExportDecl { var_name, mutable });
    }
}

/// Check if a line looks like a template ref invocation.
/// Patterns:
/// - `&counter("Likes");` — anonymous invocation (& followed by ident and parens)
/// - `&likeCounter &counter("Likes");` — named ref (& ident & ident(...))
/// - `&cards[] &counter($item);` — collection ref (& ident[] & ident(...))

/// Parse a template ref invocation line into a TemplateRef.
/// Handles:
/// - `&counter("Likes");` → anonymous (ref_name=None, template_name="counter")
/// - `&likeCounter &counter("Likes");` → named (ref_name=Some("likeCounter"))
/// - `&cards[] &counter($item);` → collection (ref_name=Some("cards"), is_collection=true)
/// Whether a construct-body segment is an ENTITY SCOPE (`&name { @component... }`):
/// a `&`, an identifier, NO `(args)` before the brace, then a `{ body }`. This is
/// the nested-body twin of the top-level entity-scope recognition (FEAT-142 Wave E),
/// so the component-body validator accepts it instead of raising E0900.
fn is_entity_scope_segment(line: &str) -> bool {
    let s = line.trim();
    let Some(rest) = s.strip_prefix('&') else {
        return false;
    };
    let rest = rest.trim_start();
    // Read the identifier.
    let name_end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(rest.len());
    if name_end == 0 {
        return false; // no name
    }
    let after = rest[name_end..].trim_start();
    // An entity scope has a `{` body next, NOT a `(` arg list (that's a template
    // invocation) and NOT a `.`/selector (that's an element-ref with a selector).
    after.starts_with('{')
}

fn parse_template_ref(line: &str) -> Option<crate::syntax::form_match::TemplateRef> {
    let s = line.trim().trim_end_matches(';').trim();
    let s = s.strip_prefix('&')?;

    // Check for second & — this means first part is ref name, second is template call
    if let Some(second_amp_pos) = s.find('&') {
        // Before the second & is the ref name (possibly with [] suffix)
        let ref_part = s[..second_amp_pos].trim();
        let call_part = s[second_amp_pos + 1..].trim();

        // Parse ref name: could be "cards[]" or "likeCounter"
        let (ref_name, is_collection) = if ref_part.ends_with("[]") {
            (ref_part[..ref_part.len() - 2].to_string(), true)
        } else {
            (ref_part.to_string(), false)
        };

        // Validate ref name is a valid identifier
        if ref_name.is_empty() || !ref_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }

        // Parse template call: template_name(args)
        let (template_name, raw_args) = parse_template_call(call_part)?;
        let (args, arg_names) = crate::syntax::normalize_invocation_args(&raw_args);

        Some(crate::syntax::form_match::TemplateRef {
            ref_name: Some(ref_name),
            template_name,
            args,
            is_collection,
            arg_names,
            // String-parser context (no source offsets); the span-bearing harvest
            // is scope_refs/harvest_dynamic_refs. Zero span = unknown (G2 skips it).
            span: Default::default(),
        })
    } else {
        // Anonymous invocation: &template(args)
        let (template_name, raw_args) = parse_template_call(s)?;
        let (args, arg_names) = crate::syntax::normalize_invocation_args(&raw_args);
        Some(crate::syntax::form_match::TemplateRef {
            ref_name: None,
            template_name,
            args,
            is_collection: false,
            arg_names,
            span: Default::default(),
        })
    }
}

/// Parse a template call expression: `template_name(arg1, arg2, ...)`
/// Returns (template_name, args_vec)
fn parse_template_call(s: &str) -> Option<(String, Vec<String>)> {
    let paren_pos = s.find('(')?;
    let template_name = s[..paren_pos].trim();
    if template_name.is_empty() {
        return None;
    }
    // DYNAMIC DISPATCH (FEAT-073): `&$w(...)` / `&$node.widget(...)` — the template NAME is a
    // runtime value. Extension of the template_invocation grammar; keep the `$`-name
    // verbatim, the stdlib runtime (template.st) resolves it against `data` before
    // templates.get(). Dotted path allowed for field access.
    if let Some(dyn_name) = template_name.strip_prefix('$') {
        if dyn_name.is_empty()
            || !dyn_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        {
            return None;
        }
        let close_paren = s.rfind(')')?;
        let args_str = &s[paren_pos + 1..close_paren];
        let args: Vec<String> = if args_str.trim().is_empty() {
            vec![]
        } else {
            args_str.split(',').map(|a| a.trim().to_string()).collect()
        };
        return Some((template_name.to_string(), args));
    }
    // Validate template name: may contain hyphens (e.g., "product-card")
    if !template_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let close_paren = s.rfind(')')?;
    let args_str = &s[paren_pos + 1..close_paren];
    let args: Vec<String> = if args_str.trim().is_empty() {
        vec![]
    } else {
        args_str.split(',').map(|a| a.trim().to_string()).collect()
    };
    Some((template_name.to_string(), args))
}

/// Check if a component body contains any non-HTML sections.
///
/// Returns false for pure HTML content (backward compat fast path).
/// Check if a component body contains any non-HTML sections.
///
/// Returns false for pure HTML content (backward-compat fast path that returns the
/// body verbatim as `html`). Delegates to `segment_component_body` so the fast-path
/// gate and the full parse share ONE segmentation truth — the previous line-atomic
/// gate diverged from the parser (e.g. `<div/> $n number: 0;` on one line was seen as
/// pure HTML by the gate but had a real state decl), silently dropping constructs.
fn has_component_body_sections(content: &str) -> bool {
    segment_component_body(content)
        .iter()
        .any(|seg| !seg.is_html)
}

/// Check if a line is a state declaration pattern: $name type: [default]
/// Distinguished from HTML interpolation ($item.prop) by requiring a type word.

/// Check if a body_capture spec has any required (non-optional) captures.
/// Body capture specs look like `"{ $html:html_block $animations:keyframes? }"` or
/// `"{ $children* }"`. Captures ending with `?` or `*` are optional; all others
/// are required. If any capture is required, the body itself is required.
fn body_capture_has_required(spec: &str) -> bool {
    for token in spec.split_whitespace() {
        if (token.starts_with('$') || token.starts_with('&'))
            && !token.ends_with('?')
            && !token.ends_with('*')
        {
            return true;
        }
    }
    false
}

/// Try to find tokens matching a keyword block in the remaining cursor.
///
/// Scans ahead in the cursor for an IDENT matching the keyword, then
/// collects the following brace-delimited block's tokens.
fn ctx_child_by_keyword(
    cursor: &mut TokenCursor,
    source: &str,
    keyword: &str,
) -> Option<Vec<TokenData>> {
    // Save current position for potential backtrack
    let saved = cursor.pos;

    // Scan ahead looking for the keyword identifier
    while !cursor.at_end() {
        cursor.skip_trivia();
        if cursor.at_end() {
            break;
        }
        let kind = cursor.current_kind();
        if kind == SyntaxKind::IDENT || kind.is_keyword() {
            let text = cursor.current_text(source);
            if text == keyword {
                cursor.advance(1); // consume keyword
                cursor.skip_trivia();

                // Expect L_BRACE
                if cursor.at_end() || cursor.current_kind() != SyntaxKind::L_BRACE {
                    cursor.pos = saved;
                    return None;
                }

                // Collect tokens inside the braces
                let mut block_tokens = Vec::new();
                let mut depth = 0i32;

                while !cursor.at_end() {
                    let k = cursor.current_kind();
                    if k == SyntaxKind::L_BRACE {
                        depth += 1;
                    } else if k == SyntaxKind::R_BRACE {
                        depth -= 1;
                        if depth == 0 {
                            cursor.advance(1); // consume closing brace
                            return Some(block_tokens);
                        }
                    }
                    block_tokens.push(cursor.current_token().clone());
                    cursor.advance(1);
                }

                // Unbalanced braces
                cursor.pos = saved;
                return None;
            }
        }
        cursor.advance(1);
    }

    // Keyword not found
    cursor.pos = saved;
    None
}

/// Check if a SCOPE_BLOCK's inline_tokens match a pseudo-selector name.
///
/// SCOPE_BLOCK.tokens contains the SELECTOR child's tokens flattened in
/// (since SELECTOR is non-structural). We look for COLON followed by IDENT(name).
/// Extract the pseudo-selector name from a SCOPE_BLOCK node's inline tokens.
/// Looks for COLON followed by IDENT/keyword pattern, returns the name as a
/// zero-copy `&str` borrowed from `source`.
fn extract_pseudo_name<'a>(scope_block: &ChildNode, source: &'a str) -> Option<&'a str> {
    let mut found_colon = false;
    for tok in &scope_block.tokens {
        if tok.kind.is_trivia() {
            continue;
        }
        if tok.kind == SyntaxKind::COLON {
            found_colon = true;
        } else if found_colon && (tok.kind == SyntaxKind::IDENT || tok.kind.is_keyword()) {
            return Some(tok.text(source));
        } else {
            found_colon = false;
        }
    }
    None
}

// =============================================================================
// MatchFailure
// =============================================================================

/// Records WHY a form match failed — which element, position, expected vs got.
/// This feeds the diagnostic system.
#[derive(Debug, Clone)]
pub struct MatchFailure {
    /// Which form was being tried. NB this is the MACRO name
    /// (`wait-for-appears`), NOT the authored directive surface (`@wait-for`) —
    /// dispatch keys on the `%form`'s first token, not the macro name
    /// (`stdlib/testing/automation.st:59-61`). Do not use it to decide whether a
    /// failure belongs to the directive the author wrote; that is what the
    /// failure KIND is for (see match_sink's reporting rule, BUG-229).
    pub form_name: String,
    /// Index of the element in the FormClause that failed
    pub element_index: usize,
    /// What went wrong
    pub kind: FailureKind,
}

/// Detailed failure reason.
#[derive(Debug, Clone)]
pub enum FailureKind {
    /// Directive prefix didn't match (e.g., expected "@on" but got "@data")
    DirectivePrefixMismatch { expected: String },
    /// Literal text didn't match
    LiteralMismatch {
        expected: String,
        got: Option<String>,
    },
    /// Capture extraction failed — no tokens remaining
    CaptureFailedNoTokens {
        var_name: String,
        capture_type: String,
    },
    /// Capture extraction failed — wrong token type
    CaptureTypeMismatch {
        var_name: String,
        expected_type: String,
        /// When the expected capture is a UNION (`$kind:("style"|"motion")`), its
        /// alternatives verbatim; empty for every other capture type.
        ///
        /// Sibling overloads that discriminate on a ONE-ALTERNATIVE union — the
        /// kind-is-the-macro shape (`@form style …` / `@form motion …`, see
        /// `stdlib/macros/form.st`) — each contribute a single word here. Carrying
        /// them as DATA (rather than only inside the `{:?}` rendering of the
        /// capture type) lets the reporting layer merge the siblings' failures
        /// into the FULL vocabulary, instead of naming whichever sibling happened
        /// to rank first and leaving the author to guess the rest.
        expected_alternatives: Vec<String>,
        got_kind: String,
    },
    /// Expected an ARG_LIST child node but none found
    MissingArgList,
    /// Expected a BODY child node but none found
    MissingBody,
    /// Expected a keyword block but none found
    MissingKeywordBlock { keyword: String },
    /// Expected a pseudo-selector but none found
    MissingPseudoSelector { name: String },
    /// Unconsumed inline tokens after matching all inline elements
    ExtraInlineTokens { remaining: usize },
    /// The input carries a `{ … }` body block, but this form declares no body
    /// consumer (no `body_capture`, no `body_groups`). A paren-args / inline-only
    /// form (e.g. the 3D `@light(type, …)`) must REJECT a brace-body invocation
    /// (e.g. `@light { color: red }`) so a sibling overload that DOES take a body
    /// (the responsive `light-mode`'s `@light { $styles }`) is selected instead.
    /// Without this, an all-defaulted inline form silently swallows `@name { … }`,
    /// ignoring the body, and wins on specificity — dropping the body capture.
    UnexpectedBody,
}

impl MatchFailure {
    /// Get a human-readable error message.
    pub fn message(&self) -> String {
        match &self.kind {
            FailureKind::DirectivePrefixMismatch { expected } => {
                format!("Expected directive `{}`", expected)
            }
            FailureKind::LiteralMismatch { expected, got } => {
                format!(
                    "Expected `{}`, got {}",
                    expected,
                    got.as_deref().unwrap_or("end of input")
                )
            }
            FailureKind::CaptureFailedNoTokens {
                var_name,
                capture_type,
            } => {
                format!(
                    "Expected capture `${}:{}` but no tokens remaining",
                    var_name, capture_type
                )
            }
            FailureKind::CaptureTypeMismatch {
                var_name,
                expected_type,
                expected_alternatives,
                got_kind,
            } => {
                // A union capture states a VOCABULARY, so say the words. Rendering
                // `{:?}` of the capture type here would print Rust's
                // `Union(["style"])` at an author, and — worse — name only the one
                // sibling whose failure ranked first.
                if expected_alternatives.is_empty() {
                    format!(
                        "Expected `${}:{}`, got token {:?}",
                        var_name, expected_type, got_kind
                    )
                } else {
                    format!(
                        "Expected `${}` to be one of {}, got token {:?}",
                        var_name,
                        expected_alternatives
                            .iter()
                            .map(|alt| format!("`{alt}`"))
                            .collect::<Vec<_>>()
                            .join(" | "),
                        got_kind
                    )
                }
            }
            FailureKind::MissingArgList => "Expected parenthesized arguments".to_string(),
            FailureKind::MissingBody => "Expected body block `{ ... }`".to_string(),
            FailureKind::MissingKeywordBlock { keyword } => {
                format!("Expected keyword block `{} {{ ... }}`", keyword)
            }
            FailureKind::MissingPseudoSelector { name } => {
                format!("Expected pseudo-selector `:{}`", name)
            }
            FailureKind::ExtraInlineTokens { remaining } => {
                format!(
                    "Unexpected {} extra inline token(s) after directive",
                    remaining
                )
            }
            FailureKind::UnexpectedBody => "This form takes no `{ ... }` body block".to_string(),
        }
    }
}

// =============================================================================
// TokenCursor
// =============================================================================

/// Cursor over a token slice for sequential matching.
#[derive(Clone)]
pub struct TokenCursor<'a> {
    tokens: &'a [TokenData],
    pub pos: usize,
}

impl<'a> TokenCursor<'a> {
    pub fn new(tokens: &'a [TokenData]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Get remaining tokens from current position.
    pub fn remaining(&self) -> &'a [TokenData] {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos..]
        } else {
            &[]
        }
    }

    /// Advance the cursor by `n` tokens.
    pub fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    /// Skip whitespace and comment tokens.
    pub fn skip_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind.is_trivia() {
            self.pos += 1;
        }
    }

    /// Count remaining non-trivia tokens.
    pub fn remaining_non_trivia(&self) -> usize {
        self.tokens[self.pos..]
            .iter()
            .filter(|t| !t.kind.is_trivia())
            .count()
    }

    /// Eat a token of the given kind, advancing if matched.
    pub fn eat_kind(&mut self, kind: SyntaxKind) -> bool {
        if self.pos < self.tokens.len() && self.tokens[self.pos].kind == kind {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Skip a token of the given kind (eat without caring about result).
    pub fn skip_kind(&mut self, kind: SyntaxKind) {
        self.eat_kind(kind);
    }

    /// FEAT-118: skip a leading namespace QUALIFIER on a directive invocation —
    /// the `scene/` (or alias `s/`) in `@scene/camera` — so a qualified call
    /// still matches the unqualified `@camera` `%form`. Consumes `(IDENT '/')+`
    /// runs ONLY when the segment is immediately followed by `/` (a real
    /// qualifier), leaving the final leaf name for the form's own name match.
    /// Returns the consumed qualifier segments (namespace path / alias), or
    /// empty when there was none. Trivia-free by construction (the parser only
    /// emits tight `/` qualifiers).
    pub fn skip_namespace_qualifier(&mut self, source: &str) -> Vec<String> {
        let mut segments = Vec::new();
        loop {
            // Need IDENT then SLASH at the current raw position (no trivia).
            if self.pos + 1 >= self.tokens.len() {
                break;
            }
            let first = &self.tokens[self.pos];
            let second = &self.tokens[self.pos + 1];
            let first_is_name = first.kind == SyntaxKind::IDENT || first.kind.is_keyword();
            if first_is_name && second.kind == SyntaxKind::SLASH {
                segments.push(first.text(source).to_string());
                self.pos += 2; // consume IDENT and '/'
            } else {
                break;
            }
        }
        segments
    }

    /// Eat a token whose text matches the given string.
    pub fn eat_text(&mut self, source: &str, text: &str) -> bool {
        self.skip_trivia();
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            if tok.text(source) == text {
                self.pos += 1;
                return true;
            }
            // Multi-token literal: a form literal like `&self.` lexes as several
            // tokens (`&`, `self`, `.`). Greedily consume consecutive tokens whose
            // concatenated text equals the expected literal (trivia between them is
            // NOT allowed — the literal is a contiguous run). (BUG-088 @when guard.)
            let mut acc = String::new();
            let mut lookahead = self.pos;
            while lookahead < self.tokens.len() {
                acc.push_str(self.tokens[lookahead].text(source));
                lookahead += 1;
                if acc == text {
                    self.pos = lookahead;
                    return true;
                }
                if !text.starts_with(acc.as_str()) {
                    break;
                }
            }
        }
        false
    }

    /// Peek at the current token's text.
    pub fn peek_text<'s>(&self, source: &'s str) -> Option<&'s str> {
        if self.pos < self.tokens.len() {
            Some(self.tokens[self.pos].text(source))
        } else {
            None
        }
    }

    /// Check if the cursor is at the end.
    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Get the current token's SyntaxKind.
    pub fn current_kind(&self) -> SyntaxKind {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].kind
        } else {
            SyntaxKind::EOF
        }
    }

    /// Get the current token's text.
    pub fn current_text<'s>(&self, source: &'s str) -> &'s str {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].text(source)
        } else {
            ""
        }
    }

    /// Get a reference to the current token.
    pub fn current_token(&self) -> &TokenData {
        &self.tokens[self.pos]
    }

    /// Get span for the next `n` tokens from current position.
    pub fn span_for(&self, n: usize) -> Option<SourceSpan> {
        if n == 0 || self.pos >= self.tokens.len() {
            return None;
        }
        let start = self.tokens[self.pos].text_range.0;
        let end_idx = (self.pos + n - 1).min(self.tokens.len() - 1);
        let end = self.tokens[end_idx].text_range.1;
        Some(SourceSpan { start, end })
    }
}

// =============================================================================
// NodeContext — collected data for an in-progress node
// =============================================================================

/// Data collected for a CST node during sink processing.
///
/// When the MatchSink receives `start_node`, it pushes a new NodeContext.
/// Tokens are collected into `inline_tokens`. Child nodes are stored as `ChildNode`.
/// On `finish_node`, the context is popped and passed to the FormCompiler.
#[derive(Debug, Clone)]
pub struct NodeContext {
    /// The SyntaxKind of this node.
    pub kind: SyntaxKind,
    /// Tokens directly inside this node (not in child nodes).
    pub inline_tokens: Vec<TokenData>,
    /// Completed child nodes.
    pub children: Vec<ChildNode>,
    /// FormMatches from directives that matched inside this node's BODY.
    /// These are used by `$children*` body captures to pass nested directives
    /// to the parent macro's pipeline resolution.
    pub matched_children: Vec<FormMatch>,
    /// Byte offset where this node starts in source.
    pub span_start: usize,
    /// Byte offset where this node ends in source.
    pub span_end: usize,
}

impl NodeContext {
    pub fn new(kind: SyntaxKind) -> Self {
        Self {
            kind,
            inline_tokens: Vec::new(),
            children: Vec::new(),
            matched_children: Vec::new(),
            span_start: 0,
            span_end: 0,
        }
    }

    /// Push a token into this context.
    pub fn push_token(&mut self, kind: SyntaxKind, range: (usize, usize)) {
        self.inline_tokens.push(TokenData {
            kind,
            text_range: range,
        });
    }

    /// Push a completed child node.
    ///
    /// For non-structural child nodes (ARG, CSS_PROPERTY, CSS_VALUE, etc.),
    /// the child's tokens are also flattened into the parent's inline_tokens
    /// so that the FormCompiler can access them sequentially during matching.
    /// Structural nodes (ARG_LIST, BODY) are kept separate and accessed via
    /// `child_by_kind()`.
    pub fn push_child(&mut self, child: NodeContext) {
        // Flatten tokens from non-structural children into parent
        let is_structural = matches!(
            child.kind,
            SyntaxKind::ARG_LIST | SyntaxKind::BODY | SyntaxKind::SCOPE_BLOCK
        );
        if !is_structural {
            // Copy child tokens into parent's inline_tokens for flat matching
            self.inline_tokens
                .extend(child.inline_tokens.iter().cloned());
        }

        self.children.push(ChildNode {
            kind: child.kind,
            tokens: child.inline_tokens,
            children: child.children,
            matched_children: child.matched_children,
            span_start: child.span_start,
            span_end: child.span_end,
        });
    }

    /// Push a matched child node into children for token-based body capture
    /// (e.g., `$invocations:template_invocation+`) WITHOUT extending
    /// `inline_tokens`. This avoids polluting the parent's inline tokens
    /// which would interfere with the parent directive's form matching.
    pub fn push_child_tokens_only(&mut self, child: NodeContext) {
        self.children.push(ChildNode {
            kind: child.kind,
            tokens: child.inline_tokens,
            children: child.children,
            matched_children: child.matched_children,
            span_start: child.span_start,
            span_end: child.span_end,
        });
    }

    /// Find a child node by SyntaxKind.
    pub fn child_by_kind(&self, kind: SyntaxKind) -> Option<&ChildNode> {
        self.children.iter().find(|c| c.kind == kind)
    }

    /// Check if this node is a matchable directive/reference/meta.
    pub fn is_matchable(&self) -> bool {
        matches!(
            self.kind,
            SyntaxKind::DIRECTIVE
                | SyntaxKind::VARIABLE_REF
                | SyntaxKind::ELEMENT_REF
                | SyntaxKind::META_DEF
        )
    }

    /// Extract the prefix character from the first non-trivia token.
    pub fn extract_prefix(&self, source: &str) -> Option<char> {
        for tok in &self.inline_tokens {
            if !tok.kind.is_trivia() {
                let text = tok.text(source);
                return text.chars().next();
            }
        }
        None
    }

    /// True when this DIRECTIVE context introduces a body-bearing construct whose
    /// `$body:component_body` is treated as a SCOPE (PLAN-039 / FEAT-116): its inner
    /// constructs match at selector scope and surface FLAT (to be span-assigned to
    /// the construct's Construct scope), rather than being absorbed opaquely into
    /// the body capture. Covers `@template` + the `@editable-mark`/`@editable-block`
    /// definition forms — every directive form carrying a `component_body`.
    pub fn is_template_directive(&self, source: &str, registry: &CompiledRegistry) -> bool {
        if self.kind != SyntaxKind::DIRECTIVE {
            return false;
        }
        let mut seen_at = false;
        for tok in &self.inline_tokens {
            if tok.kind.is_trivia() {
                continue;
            }
            let text = tok.text(source);
            // Registry-DERIVED (Q3): a directive is body-bearing iff its `%form`
            // declares a `$body:component_body` capture. Read from the SAME compiled
            // registry driving this parse (NOT the global STDLIB_REGISTRY, which may
            // still be initializing during bootstrap — touching it here re-enters its
            // LazyLock and deadlocks). No hardcoded name set, so a new `%macro` with a
            // `component_body` body is picked up here automatically.
            let is_body_bearing = |n: &str| registry.is_body_bearing(n);
            if !seen_at {
                if text == "@" {
                    seen_at = true;
                    continue;
                }
                return is_body_bearing(text.trim_start_matches('@'));
            }
            return is_body_bearing(text);
        }
        false
    }
}

/// A completed child node with its collected tokens and sub-children.
#[derive(Debug, Clone)]
pub struct ChildNode {
    pub kind: SyntaxKind,
    pub tokens: Vec<TokenData>,
    pub children: Vec<ChildNode>,
    /// FormMatches from directives that matched inside this node's body.
    pub matched_children: Vec<FormMatch>,
    pub span_start: usize,
    pub span_end: usize,
}

impl ChildNode {
    /// Find a sub-child node by SyntaxKind.
    pub fn child_by_kind(&self, kind: SyntaxKind) -> Option<&ChildNode> {
        self.children.iter().find(|c| c.kind == kind)
    }
}

// =============================================================================
// CompiledRegistry
// =============================================================================

/// Merge a matched body-group's value into the form's capture map (FEAT-103).
///
/// A group compiles to a `SequenceExtractor`/`ChoiceExtractor` whose value is a
/// `CapturedValue::Named` record of the inner captures (e.g. `{ shortcut -> "mod+k" }`).
/// An absent optional group yields an empty `String` with `consumed = 0` (handled by
/// the caller); a present group's named keys are lifted into `captures` so downstream
/// `%registers` / `%binds` see `$shortcut` exactly as if it were a top-level capture.
fn merge_group_captures(
    group: &crate::parser::meta_ast::CapturePatternAst,
    value: CapturedValue,
    captures: &mut HashMap<String, CapturedValue>,
) {
    match value {
        // A Sequence/Choice group yields a Named record of its inner captures — lift
        // each key to a top-level capture.
        CapturedValue::Named(map) => {
            for (k, v) in map {
                captures.insert(k, v);
            }
        }
        // A non-Named value (a SINGLE-capture group → scalar, or a REPEAT group →
        // Array) carries one binding, named by the group's inner capture. The
        // name→Named threading lives only in Sequence/Choice extractors, so a bare
        // `( $s:string )?` or `( $row:ident )*` compiles to a scalar/Array; without
        // this branch the tokens would be consumed but the value dropped (FEAT-104).
        other => {
            if let Some(name) = super::extractors::custom::pattern_capture_name(group) {
                captures.insert(name, other);
            }
        }
    }
}

/// Pre-compiled registry mapping prefix → compiled forms (sorted by specificity).
pub struct CompiledRegistry {
    forms: HashMap<char, Vec<CompiledForm>>,
}

impl CompiledRegistry {
    /// Build a CompiledRegistry from a SyntaxRegistry.
    ///
    /// Each registered form is compiled into a CompiledForm for matching.
    pub fn from_registry(registry: &SyntaxRegistry, _extractors: &ExtractorRegistry) -> Self {
        let mut forms: HashMap<char, Vec<CompiledForm>> = HashMap::new();

        for registered in registry.all_forms() {
            let prefix = registered.form.directive_name.chars().next();
            if let Some(prefix) = prefix {
                // For @-prefixed directives, use the directive name
                // (e.g., "data" from "@data"). For $ and & prefixes, use macro_name.
                let creates_name = if prefix == '@' {
                    registered
                        .form
                        .directive_name
                        .trim_start_matches('@')
                        .to_string()
                } else {
                    registered.macro_name.clone()
                };
                let compiled = CompiledForm {
                    macro_name: registered.macro_name.clone(),
                    creates_name,
                    form: registered.form.clone(),
                    specificity: registered.specificity,
                    scopes: registered.macro_def.scopes.clone(),
                    registers_form: registered
                        .macro_def
                        .registers
                        .as_ref()
                        .is_some_and(|r| r.name == "form"),
                    drops: registered.macro_def.drops.clone(),
                };
                forms.entry(prefix).or_default().push(compiled);
            }
        }

        // Sort each prefix group by specificity (descending)
        for group in forms.values_mut() {
            group.sort_by(|a, b| b.specificity.cmp(&a.specificity));
        }

        Self { forms }
    }

    /// Get compiled forms for a given prefix character.
    pub fn forms_for_prefix(&self, prefix: char) -> &[CompiledForm] {
        self.forms.get(&prefix).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Registry-DERIVED (Q3): whether a construct is body-bearing — its `%form`
    /// declares a `$body:component_body` capture. Read from THIS compiled registry
    /// (the one driving the current parse), never the global `STDLIB_REGISTRY`, so
    /// it is safe to call while that global is still initializing (bootstrap parses
    /// stdlib macros through a MatchSink; touching the global there would re-enter
    /// its LazyLock and deadlock). Mirrors `SyntaxRegistry::is_body_bearing`.
    pub fn is_body_bearing(&self, name: &str) -> bool {
        self.forms.values().flatten().any(|f| {
            f.macro_name == name
                && f.form
                    .body_capture
                    .as_deref()
                    .is_some_and(|b| b.contains(":component_body"))
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::{CaptureModifier, CaptureType, FormCapture};
    use crate::syntax::events::test_support::{driver_member, register_driver_expr};

    // === BUG-145: named args must bind by name, not call-site position ===

    /// Build a `FormParam` for a simple named numeric param with a default
    /// (mirrors `@stage`'s `fov: $fov:number = 50` shape).
    fn named_number_param(name: &str, var: &str, default: f64) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: var.to_string(),
                    capture_type: CaptureType::Number,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            default: Some(ParamDefault::Number(default)),
        }
    }

    #[test]
    fn bug145_named_args_bind_by_name_regardless_of_call_order() {
        // %form declares: a (default 1), b (default 2) — mirrors @stage's
        // `fov: ... = 50, camZ: ... = 5` (fov declared BEFORE camZ).
        let params = vec![
            named_number_param("a", "a", 1.0),
            named_number_param("b", "b", 2.0),
        ];

        // Call site: "b: 9, a: 5" — REVERSED vs declaration order. Before the
        // fix, lockstep matching expected "a" first, saw "b", and silently
        // defaulted BOTH: a=1 (default, wrong), b=2 (default, wrong) is not
        // quite what happened (the old code actually mis-assigned differently
        // depending on shape) — the invariant that matters is the ASSERTION
        // below: whatever the old mechanics, the caller's explicit b=9/a=5
        // must land, not silently fall back to defaults.
        let source = "b: 9, a: 5";
        let tokens = vec![
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (0, 1),
            }, // b
            TokenData {
                kind: SyntaxKind::COLON,
                text_range: (1, 2),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (2, 3),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (3, 4),
            }, // 9
            TokenData {
                kind: SyntaxKind::COMMA,
                text_range: (4, 5),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (5, 6),
            },
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (6, 7),
            }, // a
            TokenData {
                kind: SyntaxKind::COLON,
                text_range: (7, 8),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (8, 9),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (9, 10),
            }, // 5
        ];

        let compiled = CompiledForm {
            macro_name: "test-reorder".to_string(),
            creates_name: "test-reorder".to_string(),
            form: FormClause {
                directive_name: "@test-reorder".to_string(),
                inline_elements: vec![],
                params,
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
            specificity: 10,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut captures = HashMap::new();
        let mut capture_spans = HashMap::new();
        let mut cursor = TokenCursor::new(&tokens);
        let extractors = ExtractorRegistry::new();
        compiled
            .match_params(
                &compiled.form.params,
                &mut cursor,
                source,
                &extractors,
                &mut captures,
                &mut capture_spans,
                &mut false,
            )
            .expect("reordered named args must still match");

        assert_eq!(
            captures.get("a"),
            Some(&CapturedValue::Number(5.0)),
            "a: 5 must bind to a, not fall back to its default 1"
        );
        assert_eq!(
            captures.get("b"),
            Some(&CapturedValue::Number(9.0)),
            "b: 9 must bind to b, not fall back to its default 2"
        );
    }

    #[test]
    fn bug145_stage_repro_fov_after_camz_binds_correctly() {
        // Exact BUG-145 repro shape: %form declares fov BEFORE camZ; call site
        // supplies camZ first, THEN fov — fov must still bind to the caller's
        // value (99), not silently fall back to the primitive default (50).
        let params = vec![
            named_number_param("fov", "fov", 50.0),
            named_number_param("camZ", "camZ", 5.0),
        ];

        let source = "camZ: 42, fov: 77";
        let tokens = vec![
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (0, 4),
            }, // camZ
            TokenData {
                kind: SyntaxKind::COLON,
                text_range: (4, 5),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (5, 6),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (6, 8),
            }, // 42
            TokenData {
                kind: SyntaxKind::COMMA,
                text_range: (8, 9),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (9, 10),
            },
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (10, 13),
            }, // fov
            TokenData {
                kind: SyntaxKind::COLON,
                text_range: (13, 14),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (14, 15),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (15, 17),
            }, // 77
        ];

        let compiled = CompiledForm {
            macro_name: "stage".to_string(),
            creates_name: "stage".to_string(),
            form: FormClause {
                directive_name: "@stage".to_string(),
                inline_elements: vec![],
                params,
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
            specificity: 10,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut captures = HashMap::new();
        let mut capture_spans = HashMap::new();
        let mut cursor = TokenCursor::new(&tokens);
        let extractors = ExtractorRegistry::new();
        compiled
            .match_params(
                &compiled.form.params,
                &mut cursor,
                source,
                &extractors,
                &mut captures,
                &mut capture_spans,
                &mut false,
            )
            .expect("reordered @stage-shaped args must still match");

        assert_eq!(
            captures.get("fov"),
            Some(&CapturedValue::Number(77.0)),
            "BUG-145: fov must bind to the caller's 77, not default to 50"
        );
        assert_eq!(captures.get("camZ"), Some(&CapturedValue::Number(42.0)));
    }

    #[test]
    fn bug145_missing_named_param_still_falls_back_to_default() {
        // A named param genuinely absent from the call must still default —
        // the fix must not regress the existing default-fallback contract.
        let params = vec![
            named_number_param("a", "a", 1.0),
            named_number_param("b", "b", 2.0),
        ];
        let source = "a: 5";
        let tokens = vec![
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (0, 1),
            },
            TokenData {
                kind: SyntaxKind::COLON,
                text_range: (1, 2),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (2, 3),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (3, 4),
            },
        ];
        let compiled = CompiledForm {
            macro_name: "test-default".to_string(),
            creates_name: "test-default".to_string(),
            form: FormClause {
                directive_name: "@test-default".to_string(),
                inline_elements: vec![],
                params,
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
            specificity: 10,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };
        let mut captures = HashMap::new();
        let mut capture_spans = HashMap::new();
        let mut cursor = TokenCursor::new(&tokens);
        let extractors = ExtractorRegistry::new();
        compiled
            .match_params(
                &compiled.form.params,
                &mut cursor,
                source,
                &extractors,
                &mut captures,
                &mut capture_spans,
                &mut false,
            )
            .expect("partial named args must still match, defaulting the rest");

        assert_eq!(captures.get("a"), Some(&CapturedValue::Number(5.0)));
        assert_eq!(
            captures.get("b"),
            Some(&CapturedValue::Number(2.0)),
            "absent b must fall back to its default"
        );
    }

    #[test]
    fn bug145_split_top_level_groups_respects_nesting() {
        // "a(1, 2), b: 3" must split into 2 groups ("a(1, 2)" and "b: 3"), not 3
        // — the comma inside a(...) must NOT split its enclosing group.
        let source = "a(1, 2), b: 3";
        let tokens = vec![
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (0, 1),
            }, // a
            TokenData {
                kind: SyntaxKind::L_PAREN,
                text_range: (1, 2),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (2, 3),
            }, // 1
            TokenData {
                kind: SyntaxKind::COMMA,
                text_range: (3, 4),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (4, 5),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (5, 6),
            }, // 2
            TokenData {
                kind: SyntaxKind::R_PAREN,
                text_range: (6, 7),
            },
            TokenData {
                kind: SyntaxKind::COMMA,
                text_range: (7, 8),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (8, 9),
            },
            TokenData {
                kind: SyntaxKind::IDENT,
                text_range: (9, 10),
            }, // b
            TokenData {
                kind: SyntaxKind::COLON,
                text_range: (10, 11),
            },
            TokenData {
                kind: SyntaxKind::WHITESPACE,
                text_range: (11, 12),
            },
            TokenData {
                kind: SyntaxKind::NUMBER,
                text_range: (12, 13),
            }, // 3
        ];
        let groups = CompiledForm::split_top_level_groups(&tokens);
        assert_eq!(
            groups.len(),
            2,
            "nested comma must not split the outer group"
        );
        assert_eq!(
            groups[0].iter().map(|t| t.text(source)).collect::<Vec<_>>(),
            vec!["a", "(", "1", ",", " ", "2", ")"]
        );
        assert_eq!(
            groups[1].iter().map(|t| t.text(source)).collect::<Vec<_>>(),
            vec![" ", "b", ":", " ", "3"]
        );
    }

    /// Test shim: Phase B (FEAT-079) deleted the Rust `parse_component_body`; the production path
    /// is now `validate_component_body` (stdlib grammar + generic reifier). The end-to-end
    /// `test_component_body_*` / `bug059_*` / `feat073_*` integration tests below assert the
    /// produced `ComponentBodyDef`, which is identical across both producers (proven by the
    /// corpus + fleet differential before deletion), so they target the live producer via this
    /// alias — no behavioral coverage lost.
    fn parse_component_body(inner: &str) -> Vec<crate::diagnostics::Diagnostic> {
        validate_component_body(inner, 0, "test")
    }

    #[test]
    fn test_backtick_hole_state_decl_emits_precise_e0900() {
        // L3: `` `$x` <type>: <value> `` mis-writes a state DECLARATION with the HOLE
        // form. Backtick is the one hole form, never a declaration site (docs §2), so
        // emit a PRECISE fix-it (“drop the backticks”), not the generic “unrecognized
        // content”. Covers the live pulsar mis-authoring (`` `$navOpen` bool: false ``).
        let body = parse_component_body("<nav class=\"n\"></nav>\n`$navOpen` bool: false;");
        let d = body
            .iter()
            .find(|d| d.code.as_str() == "E0900")
            .expect("backtick-hole state decl must emit E0900");
        assert!(
            d.message.contains("hole cannot be a state declaration"),
            "precise message, got: {}",
            d.message
        );
        assert!(
            d.hint
                .as_deref()
                .unwrap_or("")
                .contains("$navOpen bool: false"),
            "hint shows the bare-form fix, got: {:?}",
            d.hint
        );
        // A plain HTML hole (no `type:` after) must NOT trigger it.
        let clean = parse_component_body("<span>`$title`</span>");
        assert!(
            !clean.iter().any(|d| d.code.as_str() == "E0900"),
            "plain HTML hole must stay literal, got: {:?}",
            clean
        );
    }

    /// FEAT-115 S3c: collect the synthesized content-injection bindings
    /// `(selector, property, value)` from a parsed file's template scopes — the
    /// World-A representation a body-root `target <- $x` now lowers to
    /// (injection_scopes_from_body). Used by the injection unit tests.
    fn find_injection_binding(f: &crate::parser::StFile) -> Vec<(String, String, String)> {
        use crate::parser::ScopeKind;
        let mut out = Vec::new();
        for s in &f.scopes {
            if matches!(s.kind, ScopeKind::Construct(_)) {
                for n in &s.nested_scopes {
                    for d in &n.css_declarations {
                        out.push((n.selector.clone(), d.property.clone(), d.value.clone()));
                    }
                }
            }
        }
        out
    }

    fn tok(kind: SyntaxKind, start: usize, end: usize) -> TokenData {
        TokenData {
            kind,
            text_range: (start, end),
        }
    }

    #[test]
    fn token_cursor_basics() {
        let source = "@ on hover";
        let tokens = vec![
            tok(SyntaxKind::AT_SIGN, 0, 1),
            tok(SyntaxKind::WHITESPACE, 1, 2),
            tok(SyntaxKind::IDENT, 2, 4), // "on"
            tok(SyntaxKind::WHITESPACE, 4, 5),
            tok(SyntaxKind::IDENT, 5, 10), // "hover"
        ];
        let mut cursor = TokenCursor::new(&tokens);

        assert!(cursor.eat_kind(SyntaxKind::AT_SIGN));
        cursor.skip_trivia();
        assert!(cursor.eat_text(source, "on"));
        cursor.skip_trivia();
        assert_eq!(cursor.peek_text(source), Some("hover"));
        assert_eq!(cursor.remaining().len(), 1);
    }

    #[test]
    fn node_context_child_by_kind() {
        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        let child = NodeContext::new(SyntaxKind::ARG_LIST);
        ctx.push_child(child);

        assert!(ctx.child_by_kind(SyntaxKind::ARG_LIST).is_some());
        assert!(ctx.child_by_kind(SyntaxKind::BODY).is_none());
    }

    #[test]
    fn node_context_extract_prefix() {
        let source = "@on";
        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 3));

        assert_eq!(ctx.extract_prefix(source), Some('@'));
    }

    #[test]
    fn compiled_form_simple_directive() {
        // Test matching "@on $driver:driver_expr" against tokens for "@on &.hover".
        // The driver is an ELEMENT_REF (`&.hover`), so the capture is the custom
        // `driver_expr` type, not a bare Ident.
        let source = "@on &.hover";
        let form = FormClause {
            directive_name: "@on".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "driver".to_string(),
                    capture_type: CaptureType::Custom("driver_expr".to_string()),
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        };

        let compiled = CompiledForm {
            macro_name: "on-event".to_string(),
            creates_name: "on".to_string(),
            form,
            specificity: 12,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 3)); // "on"
        ctx.push_token(SyntaxKind::WHITESPACE, (3, 4));
        // The driver is an ELEMENT_REF child (non-structural, flattened into
        // the DIRECTIVE's inline tokens): `&.hover` at (4, 11).
        let mut ref_ctx = NodeContext::new(SyntaxKind::ELEMENT_REF);
        ref_ctx.push_token(SyntaxKind::AMPERSAND, (4, 5));
        ref_ctx.push_token(SyntaxKind::DOT, (5, 6));
        ref_ctx.push_token(SyntaxKind::IDENT, (6, 11)); // "hover"
        ctx.push_child(ref_ctx);

        let mut extractors = ExtractorRegistry::new();
        register_driver_expr(&mut extractors);
        let result = compiled.try_match(&ctx, source, &extractors);

        assert!(result.is_ok(), "Expected Ok but got: {:?}", result.err());
        let fm = result.unwrap().0;
        assert_eq!(fm.macro_name, "on");
        assert_eq!(
            driver_member(&fm, "driver").as_deref(),
            Some("hover"),
            "driver capture should carry member=hover"
        );
    }

    #[test]
    fn compiled_form_with_optional_param() {
        // "@on $event:ident ($duration:time?)" matching "@on hover"
        // with no parenthesized args — duration should be absent
        let source = "@on &.hover";
        let form = FormClause {
            directive_name: "@on".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "event".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![FormParam {
                name: "duration".to_string(),
                elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "duration".to_string(),
                        capture_type: CaptureType::Time,
                        modifier: CaptureModifier::Optional,
                        alias_capture: None,
                    },
                    None,
                )],
                default: None,
            }],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        };

        let compiled = CompiledForm {
            macro_name: "on-event".to_string(),
            creates_name: "on".to_string(),
            form,
            specificity: 14,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 3));
        ctx.push_token(SyntaxKind::WHITESPACE, (3, 4));
        ctx.push_token(SyntaxKind::IDENT, (4, 9));
        // No ARG_LIST child → optional params are OK

        let extractors = ExtractorRegistry::new();
        let result = compiled.try_match(&ctx, source, &extractors);

        assert!(result.is_ok());
        let fm = result.unwrap().0;
        assert!(!fm.captures.contains_key("duration"));
    }

    #[test]
    fn compiled_form_directive_prefix_mismatch() {
        // Try to match "@data" form against "@on" tokens → should fail
        let source = "@on &.hover";
        let form = FormClause {
            directive_name: "@data".to_string(),
            inline_elements: vec![],
            params: vec![],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        };

        let compiled = CompiledForm {
            macro_name: "data-fetch".to_string(),
            creates_name: "data".to_string(),
            form,
            specificity: 10,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 3)); // "on" not "data"

        let extractors = ExtractorRegistry::new();
        let result = compiled.try_match(&ctx, source, &extractors);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err.kind,
            FailureKind::DirectivePrefixMismatch { .. }
        ));
    }

    #[test]
    fn compiled_form_with_args() {
        // "@on $driver:driver_expr ($duration:time)" with arg list present.
        // The driver is an ELEMENT_REF (`&.hover`), so the capture is the custom
        // `driver_expr` type, not a bare Ident.
        let form = FormClause {
            directive_name: "@on".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "driver".to_string(),
                    capture_type: CaptureType::Custom("driver_expr".to_string()),
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![FormParam {
                name: String::new(),
                elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "duration".to_string(),
                        capture_type: CaptureType::Time,
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                )],
                default: None,
            }],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        };

        let compiled = CompiledForm {
            macro_name: "on-event".to_string(),
            creates_name: "on".to_string(),
            form,
            specificity: 14,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        // Context: ELEMENT_REF driver + ARG_LIST child
        // Source: "@on &.hover(300ms)" — note parens are part of ARG_LIST
        let source_with_parens = "@on &.hover(300ms)";
        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 3)); // "on"
        ctx.push_token(SyntaxKind::WHITESPACE, (3, 4));
        // The driver is an ELEMENT_REF child (non-structural, flattened into
        // the DIRECTIVE's inline tokens): `&.hover` at (4, 11).
        let mut ref_ctx = NodeContext::new(SyntaxKind::ELEMENT_REF);
        ref_ctx.push_token(SyntaxKind::AMPERSAND, (4, 5));
        ref_ctx.push_token(SyntaxKind::DOT, (5, 6));
        ref_ctx.push_token(SyntaxKind::IDENT, (6, 11)); // "hover"
        ctx.push_child(ref_ctx);

        // ARG_LIST child with (300ms)
        let mut arg_ctx = NodeContext::new(SyntaxKind::ARG_LIST);
        arg_ctx.push_token(SyntaxKind::L_PAREN, (11, 12));
        arg_ctx.push_token(SyntaxKind::NUMBER_WITH_UNIT, (12, 17)); // "300ms"
        arg_ctx.push_token(SyntaxKind::R_PAREN, (17, 18));
        ctx.push_child(arg_ctx);

        let mut extractors = ExtractorRegistry::new();
        register_driver_expr(&mut extractors);
        let result = compiled.try_match(&ctx, source_with_parens, &extractors);

        assert!(result.is_ok(), "Expected Ok but got: {:?}", result.err());
        let fm = result.unwrap().0;
        assert_eq!(
            driver_member(&fm, "driver").as_deref(),
            Some("hover"),
            "driver capture should carry member=hover"
        );
        assert_eq!(fm.captures.get("duration"), Some(&CapturedValue::Time(300)));
    }

    #[test]
    fn match_failure_message() {
        let failure = MatchFailure {
            form_name: "on-event".to_string(),
            element_index: 0,
            kind: FailureKind::DirectivePrefixMismatch {
                expected: "@on".to_string(),
            },
        };
        assert!(failure.message().contains("@on"));
    }

    fn make_test_macro_def(name: &str, form: FormClause) -> crate::parser::meta_ast::MacroDefAst {
        crate::parser::meta_ast::MacroDefAst {
            retired: None,
            name: name.to_string(),
            form: Some(form),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            requires: vec![],
            body: vec![],
            span: SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        }
    }
    

    #[test]
    fn compiled_registry_groups_by_prefix() {
        let mut syntax_reg = SyntaxRegistry::new();

        let macro_def = make_test_macro_def(
            "test-on",
            FormClause {
                directive_name: "@on".to_string(),
                inline_elements: vec![],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
        );
        syntax_reg.register(&macro_def);

        let extractors = ExtractorRegistry::new();
        let compiled = CompiledRegistry::from_registry(&syntax_reg, &extractors);

        assert_eq!(compiled.forms_for_prefix('@').len(), 1);
        assert_eq!(compiled.forms_for_prefix('$').len(), 0);
    }

    #[test]
    fn body_capture_required_detection() {
        // Required body: has a capture without ? or * modifier
        assert!(body_capture_has_required("{ $html:html_block }"));
        assert!(body_capture_has_required(
            "{ $html:html_block $animations:keyframes? }"
        ));

        // Optional body: all captures have ? or * modifiers
        assert!(!body_capture_has_required("{ $children* }"));
        assert!(!body_capture_has_required("{ $body:html_block? }"));
        assert!(!body_capture_has_required("{ $a:ident? $b:expr* }"));

        // No captures — body not required
        assert!(!body_capture_has_required("{ }"));
    }

    #[test]
    fn match_fails_when_required_body_missing() {
        let source = "&card";
        let extractors = ExtractorRegistry::new();

        // Form: &$name:ident { $body:html_block } — body required
        let form = CompiledForm {
            macro_name: "test".to_string(),
            creates_name: "test".to_string(),
            form: FormClause {
                directive_name: "&".to_string(),
                inline_elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "name".to_string(),
                        capture_type: CaptureType::Ident,
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                )],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: Some("{ $body:html_block }".to_string()),
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
            specificity: 1,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        // Node context with & + "card" but NO body child
        let ctx = NodeContext {
            kind: SyntaxKind::ELEMENT_REF,
            inline_tokens: vec![
                tok(SyntaxKind::AMPERSAND, 0, 1),
                tok(SyntaxKind::IDENT, 1, 5), // "card"
            ],
            children: vec![], // No BODY child
            matched_children: vec![],
            span_start: 0,
            span_end: 5,
        };

        let result = form.try_match(&ctx, source, &extractors);
        assert!(result.is_err(), "Should fail when required body is missing");
        if let Err(failure) = result {
            assert!(
                matches!(failure.kind, FailureKind::MissingBody),
                "Expected MissingBody, got {:?}",
                failure.kind
            );
        }
    }

    #[test]
    fn html_block_body_capture_strips_braces() {
        //                  01234567890123456789012345678
        let source = "&nav {\n  <a href=\"#\">Home</a>\n}";
        let extractors = ExtractorRegistry::new();

        let form = CompiledForm {
            macro_name: "test".to_string(),
            creates_name: "test".to_string(),
            form: FormClause {
                directive_name: "&".to_string(),
                inline_elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "name".to_string(),
                        capture_type: CaptureType::Ident,
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                )],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: Some("{ $body:html_block }".to_string()),
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
            specificity: 1,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        // Body child spans the "{ ... }" portion of source (offset 5..end)
        let body_child = ChildNode {
            kind: SyntaxKind::BODY,
            tokens: vec![],
            children: vec![],
            matched_children: vec![],
            span_start: 5,
            span_end: source.len(),
        };

        let ctx = NodeContext {
            kind: SyntaxKind::ELEMENT_REF,
            inline_tokens: vec![
                tok(SyntaxKind::AMPERSAND, 0, 1),
                tok(SyntaxKind::IDENT, 1, 4), // "nav"
            ],
            children: vec![body_child],
            matched_children: vec![],
            span_start: 0,
            span_end: source.len(),
        };

        let result = form.try_match(&ctx, source, &extractors);
        assert!(
            result.is_ok(),
            "Should match successfully: {:?}",
            result.err()
        );
        let fm = result.unwrap().0;
        let body_val = fm.captures.get("body").expect("body capture should exist");
        match body_val {
            CapturedValue::String(s) => {
                assert!(
                    !s.starts_with('{'),
                    "HtmlBlock should not start with '{{': got {:?}",
                    s
                );
                assert!(
                    !s.ends_with('}'),
                    "HtmlBlock should not end with '}}': got {:?}",
                    s
                );
                assert!(
                    s.contains("<a href="),
                    "Should preserve HTML content: got {:?}",
                    s
                );
            }
            other => panic!("Expected CapturedValue::String, got {:?}", other),
        }
    }

    #[test]
    fn template_invocation_body_capture_one_or_more_keeps_args() {
        let source = "@each { &ado-project($p); }";
        let extractors = ExtractorRegistry::new();

        let form = CompiledForm {
            macro_name: "each".to_string(),
            creates_name: "each".to_string(),
            form: FormClause {
                directive_name: "@each".to_string(),
                inline_elements: vec![],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: Some("{ $invocations:template_invocation+ }".to_string()),
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
            specificity: 1,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };
        let arg_list_child = ChildNode {
            kind: SyntaxKind::ARG_LIST,
            tokens: vec![
                tok(SyntaxKind::L_PAREN, 20, 21),
                tok(SyntaxKind::DOLLAR, 21, 22),
                tok(SyntaxKind::IDENT, 22, 23),
                tok(SyntaxKind::R_PAREN, 23, 24),
            ],
            children: vec![],
            matched_children: vec![],
            span_start: 20,
            span_end: 24,
        };

        let element_ref_child = ChildNode {
            kind: SyntaxKind::ELEMENT_REF,
            tokens: vec![
                tok(SyntaxKind::AMPERSAND, 8, 9),
                tok(SyntaxKind::IDENT, 9, 12),
                tok(SyntaxKind::MINUS, 12, 13),
                tok(SyntaxKind::IDENT, 13, 20),
            ],
            children: vec![arg_list_child],
            matched_children: vec![],
            span_start: 8,
            span_end: 24,
        };

        let body_child = ChildNode {
            kind: SyntaxKind::BODY,
            tokens: vec![
                tok(SyntaxKind::L_BRACE, 6, 7),
                tok(SyntaxKind::WHITESPACE, 7, 8),
                tok(SyntaxKind::SEMICOLON, 24, 25),
                tok(SyntaxKind::WHITESPACE, 25, 26),
                tok(SyntaxKind::R_BRACE, 26, 27),
            ],
            children: vec![element_ref_child],
            matched_children: vec![],
            span_start: 6,
            span_end: 27,
        };

        let ctx = NodeContext {
            kind: SyntaxKind::DIRECTIVE,
            inline_tokens: vec![tok(SyntaxKind::AT_SIGN, 0, 1), tok(SyntaxKind::IDENT, 1, 5)],
            children: vec![body_child],
            matched_children: vec![],
            span_start: 0,
            span_end: source.len(),
        };

        let result = form.try_match(&ctx, source, &extractors);
        assert!(
            result.is_ok(),
            "Expected @each form to match: {:?}",
            result.err()
        );

        let fm = result.unwrap().0;
        let invocations = fm
            .captures
            .get("invocations")
            .expect("invocations capture should exist");

        match invocations {
            CapturedValue::Array(items) => {
                assert_eq!(items.len(), 1, "Expected one template invocation");
                match &items[0] {
                    CapturedValue::Named(map) => {
                        assert_eq!(
                            map.get("name"),
                            Some(&CapturedValue::String("ado-project".to_string()))
                        );
                        assert_eq!(
                            map.get("args"),
                            Some(&CapturedValue::Array(vec![CapturedValue::String(
                                "$p".to_string()
                            )]))
                        );
                    }
                    other => panic!("Expected first invocation to be Named map, got {:?}", other),
                }
            }
            other => panic!("Expected invocations to be an Array, got {:?}", other),
        }
    }

    #[test]
    fn union_capture_type_matches_variant() {
        // Test that CaptureType::Union (parameterized) works in named params.
        // This is the root cause of @fill glow(...) failing: Union types had no
        // extractor registered, causing cursor advancement failures.
        //
        // Form: @fill $name:ident($radius:number, $center:("mouse" | "screen"))
        // Input: @fill glow(radius: 200, center: mouse)
        let source = "@fill glow(radius: 200, center: mouse)";
        let extractors = ExtractorRegistry::new();

        let form = FormClause {
            directive_name: "@fill".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "name".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![
                FormParam {
                    name: "radius".to_string(),
                    elements: vec![FormInlineElement::Capture(
                        FormCapture {
                            var_name: "radius".to_string(),
                            capture_type: CaptureType::Number,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    )],
                    default: None,
                },
                FormParam {
                    name: "center".to_string(),
                    elements: vec![FormInlineElement::Capture(
                        FormCapture {
                            var_name: "center".to_string(),
                            capture_type: CaptureType::Union(vec![
                                "mouse".to_string(),
                                "screen".to_string(),
                            ]),
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    )],
                    default: None,
                },
            ],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        };

        let compiled = CompiledForm {
            macro_name: "fill-glow".to_string(),
            creates_name: "fill".to_string(),
            form,
            specificity: 10,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 5)); // "fill"
        ctx.push_token(SyntaxKind::WHITESPACE, (5, 6));
        ctx.push_token(SyntaxKind::IDENT, (6, 10)); // "glow"

        // ARG_LIST child with named params: (radius: 200, center: mouse)
        let mut arg_ctx = NodeContext::new(SyntaxKind::ARG_LIST);
        arg_ctx.push_token(SyntaxKind::L_PAREN, (10, 11));
        arg_ctx.push_token(SyntaxKind::IDENT, (11, 17)); // "radius"
        arg_ctx.push_token(SyntaxKind::COLON, (17, 18));
        arg_ctx.push_token(SyntaxKind::WHITESPACE, (18, 19));
        arg_ctx.push_token(SyntaxKind::NUMBER, (19, 22)); // "200"
        arg_ctx.push_token(SyntaxKind::COMMA, (22, 23));
        arg_ctx.push_token(SyntaxKind::WHITESPACE, (23, 24));
        arg_ctx.push_token(SyntaxKind::IDENT, (24, 30)); // "center"
        arg_ctx.push_token(SyntaxKind::COLON, (30, 31));
        arg_ctx.push_token(SyntaxKind::WHITESPACE, (31, 32));
        arg_ctx.push_token(SyntaxKind::IDENT, (32, 37)); // "mouse"
        arg_ctx.push_token(SyntaxKind::R_PAREN, (37, 38));
        ctx.push_child(arg_ctx);

        let result = compiled.try_match(&ctx, source, &extractors);
        assert!(
            result.is_ok(),
            "Union capture type should match: {:?}",
            result.err()
        );

        let fm = result.unwrap().0;
        assert_eq!(
            fm.captures.get("name"),
            Some(&CapturedValue::Ident("glow".to_string()))
        );
        assert_eq!(
            fm.captures.get("center"),
            Some(&CapturedValue::Ident("mouse".to_string())),
            "Union type should capture the matched variant"
        );
    }

    #[test]
    fn union_capture_type_rejects_invalid_variant() {
        // Union type should reject values not in the variant list
        let source = "@fill glow(center: keyboard)";
        let extractors = ExtractorRegistry::new();

        let form = FormClause {
            directive_name: "@fill".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: "name".to_string(),
                    capture_type: CaptureType::Ident,
                    modifier: CaptureModifier::Required,
                    alias_capture: None,
                },
                None,
            )],
            params: vec![FormParam {
                name: "center".to_string(),
                elements: vec![FormInlineElement::Capture(
                    FormCapture {
                        var_name: "center".to_string(),
                        capture_type: CaptureType::Union(vec![
                            "mouse".to_string(),
                            "screen".to_string(),
                        ]),
                        modifier: CaptureModifier::Required,
                        alias_capture: None,
                    },
                    None,
                )],
                default: None,
            }],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        };

        let compiled = CompiledForm {
            macro_name: "fill-glow".to_string(),
            creates_name: "fill".to_string(),
            form,
            specificity: 10,
            scopes: vec![],
            registers_form: false,
            drops: Vec::new(),
        };

        let mut ctx = NodeContext::new(SyntaxKind::DIRECTIVE);
        ctx.push_token(SyntaxKind::AT_SIGN, (0, 1));
        ctx.push_token(SyntaxKind::IDENT, (1, 5)); // "fill"
        ctx.push_token(SyntaxKind::WHITESPACE, (5, 6));
        ctx.push_token(SyntaxKind::IDENT, (6, 10)); // "glow"

        let mut arg_ctx = NodeContext::new(SyntaxKind::ARG_LIST);
        arg_ctx.push_token(SyntaxKind::L_PAREN, (10, 11));
        arg_ctx.push_token(SyntaxKind::IDENT, (11, 17)); // "center"
        arg_ctx.push_token(SyntaxKind::COLON, (17, 18));
        arg_ctx.push_token(SyntaxKind::WHITESPACE, (18, 19));
        arg_ctx.push_token(SyntaxKind::IDENT, (19, 27)); // "keyboard"
        arg_ctx.push_token(SyntaxKind::R_PAREN, (27, 28));
        ctx.push_child(arg_ctx);

        let result = compiled.try_match(&ctx, source, &extractors);
        assert!(
            result.is_err(),
            "Union type should reject invalid variant 'keyboard'"
        );
    }

    /// PLAN-122 W1.2 — the CSS scalar names resolve to stdlib grammars on THIS
    /// path too.
    ///
    /// `parse_capture_type_name` is a second copy of `parse_capture_type`
    /// (src/parser/mod.rs), and the duplication is why this test matters: a
    /// migration done in only one of the two tables fails SILENTLY. The name
    /// keeps resolving to its Rust arm on whichever path still has it, the
    /// stdlib production sits unused, and nothing errors — the exact failure
    /// mode FUP-151 was filed for.
    #[test]
    fn css_scalar_names_resolve_to_stdlib_on_the_form_compiler_path() {
        for name in ["color", "length", "duration", "time", "easing"] {
            assert_eq!(
                parse_capture_type_name(name),
                Some(CaptureType::Custom(name.to_string())),
                "`{name}` still resolves to a hardcoded Rust arm in \
                 parse_capture_type_name, so a form compiled through THIS path \
                 bypasses the stdlib grammar in css-values.st"
            );
        }
    }

    #[test]
    fn test_parse_body_properties_multiline_nested_value() {
        // A `value:` literal spanning multiple lines with nested objects must
        // be captured whole, not truncated at the first newline (BUG-031).
        let inner = "src: inline;\n    value: [\n      { \"id\": \"p1\", \"name\": \"Rug\" },\n      { \"id\": \"p2\", \"name\": \"Pot\" }\n    ];\n    cache: 0;";
        let props = parse_body_properties(inner);
        assert_eq!(props.get("src").copied(), Some("inline"));
        assert_eq!(props.get("cache").copied(), Some("0"));
        let value = props
            .get("value")
            .copied()
            .expect("value should be captured");
        assert!(
            value.starts_with('['),
            "value should start with [: {:?}",
            value
        );
        assert!(
            value.contains("p1") && value.contains("p2"),
            "both items present"
        );
        assert!(
            value.trim_end().ends_with(']'),
            "value should end with ]: {:?}",
            value
        );
    }

    #[test]
    fn test_parse_body_properties_single_line() {
        // Single-line form keeps working.
        let props = parse_body_properties("src: \"/api/x.json\"; cache: 1h;");
        assert_eq!(props.get("src").copied(), Some("\"/api/x.json\""));
        assert_eq!(props.get("cache").copied(), Some("1h"));
    }

    #[test]
    fn test_parse_body_properties_nested_block_before_property() {
        // PLAN-053: a nested block (`@drag`'s `dragging { ... }` states sub-block)
        // that precedes a property must NOT swallow that property. The macro body
        // uses no `;` between the block and `on-drop:`, so a depth-0 `}` is an
        // implicit statement boundary. Without it, the whole blob parses as one
        // statement keyed `dragging {...} on-drop` and `on-drop` is lost.
        let inner =
            "dragging {\n  scale: 1.04\n  z-index: 50\n}\non-drop: $move({ card: $.dataset.id })";
        let props = parse_body_properties(inner);
        assert_eq!(
            props.get("on-drop").copied(),
            Some("$move({ card: $.dataset.id })"),
            "on-drop must be captured even though a `dragging {{}}` block precedes it"
        );
        // The block itself has no depth-0 `:` so it produces no spurious key.
        assert!(
            !props.contains_key("dragging"),
            "the states block must not become a property key"
        );
    }

    #[test]
    fn test_convert_property_value_color_hex_short() {
        let val = convert_property_value("#fff", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "#fff"));
    }

    #[test]
    fn test_convert_property_value_color_hex_long() {
        let val = convert_property_value("#E85D4A", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "#E85D4A"));
    }

    #[test]
    fn test_convert_property_value_color_named() {
        let val = convert_property_value("cyan", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "cyan"));
    }

    #[test]
    fn test_convert_property_value_color_rgb() {
        let val = convert_property_value("rgb(232, 93, 74)", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "rgb(232, 93, 74)"));
    }

    #[test]
    fn test_convert_property_value_color_hsl() {
        let val = convert_property_value("hsl(120, 100%, 50%)", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "hsl(120, 100%, 50%)"));
    }

    #[test]
    fn test_convert_property_value_color_oklch() {
        let val = convert_property_value("oklch(0.7 0.15 180)", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "oklch(0.7 0.15 180)"));
    }

    #[test]
    fn test_convert_property_value_color_rgba() {
        let val = convert_property_value("rgba(0, 0, 0, 0.5)", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "rgba(0, 0, 0, 0.5)"));
    }

    #[test]
    fn test_convert_property_value_color_color_mix() {
        let val = convert_property_value("color-mix(in srgb, red, blue)", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "color-mix(in srgb, red, blue)"));
    }

    #[test]
    fn test_convert_property_value_color_display_p3() {
        let val = convert_property_value("color(display-p3 1 0.5 0)", &CaptureType::Color);
        assert!(matches!(val, CapturedValue::Color(ref s) if s == "color(display-p3 1 0.5 0)"));
    }

    // =========================================================================
    // PROJ-103: Reactive Properties — Parse Tests
    // =========================================================================

    // --- parse_class_toggle tests ---

    // --- parse_injection tests ---

    #[test]
    fn bug061_attr_selector_injection_is_selector_not_attr() {
        // FEAT-115 S3c / BUG-061: `[class="v"] <- $n` lowers to a `[class="v"]`
        // SELECTOR binding targeting the DESCENDANT's textContent — NOT a bogus
        // attribute named `[class="v"]`. `[slot="x"]` keeps its slot selector too.
        let f = crate::parse(
            "@template &c() { <div><span class=\"v\"></span></div>\n[class=\"v\"] <- $n; }\n<main></main>",
        )
        .expect("parse");
        let inj = find_injection_binding(&f);
        assert!(
            inj.iter()
                .any(|(sel, prop, val)| sel == "[class=\"v\"]" && prop == "text" && val == "$n"),
            "[class=v] <- $n must lower to a `[class=\"v\"]` text binding (descendant), got {:?}",
            inj
        );
        // The selector must be verbatim (not a bogus attribute name) — no binding
        // whose property is the bracket literal.
        assert!(
            !inj.iter().any(|(_s, prop, _v)| prop.starts_with('[')),
            "must not emit a bracket-literal property/attribute, got {:?}",
            inj
        );
    }

    // --- parse_self_property_binding tests ---

    #[test]
    fn test_text_injection_still_works() {
        // FEAT-115 S3c: `text <- $count` lowers to a unified `text` binding (root
        // selector), NOT a reify ContentInjection or a directive.
        let f = crate::parse(
            "@template &c() { <span class=\"c\">0</span>\ntext <- $count; }\n<main></main>",
        )
        .expect("parse");
        let inj = find_injection_binding(&f);
        assert!(
            inj.iter().any(|(_s, p, v)| p == "text" && v == "$count"),
            "text <- $count must produce a `text` binding, got {:?}",
            inj
        );
    }

    // --- parse_component_body integration tests ---

    #[test]
    fn test_component_body_text_injection() {
        // FEAT-115 S3c: `text <- $x` is transformed into a synthesized selector-scoped
        // reactive binding on the template scope (injection_scopes_from_body), not a
        // reify ContentInjection. Assert via the full parse path: the body root binding.
        let f = crate::parse(
            "@template &c() { <span class=\"count\">0</span>\ntext <- $count; }\n<main></main>",
        )
        .expect("parse");
        let inj = find_injection_binding(&f);
        assert!(
            inj.iter()
                .any(|(_sel, prop, val)| prop == "text" && val == "$count"),
            "text <- $count must lower to a `text` binding of $count, got {:?}",
            inj
        );
    }

    #[test]
    fn test_component_body_slot_injection() {
        // FEAT-115 S3c: `[slot="val"] <- $x` lowers to a synthesized `[slot="val"]`
        // selector binding (text/textContent on the matched slot child).
        let f = crate::parse(
            "@template &c() { <div><span slot=\"val\">0</span></div>\n[slot=\"val\"] <- $x; }\n<main></main>",
        )
        .expect("parse");
        let inj = find_injection_binding(&f);
        assert!(
            inj.iter()
                .any(|(sel, prop, val)| sel == "[slot=\"val\"]" && prop == "text" && val == "$x"),
            "[slot=val] <- $x must lower to a `[slot=\"val\"]` text binding, got {:?}",
            inj
        );
    }

    // === BUG-059 regression: brace/tag-aware segmentation ===

    #[test]
    fn feat073_malformed_match_surfaces_diagnostic_not_silent() {
        // FEAT-115 S3d: a malformed @match (missing `=>`) fails the flat `%macro match`
        // grammar, so no `match` construct is produced — and critically it must NOT
        // leak into the rendered html (the keystone anti-pattern: never swallow a
        // broken construct as markup). The flat macro owns @match now; the reify
        // body never produces a ComponentMatch.
        let f = crate::parse(
            "@template &c($k) { <div class=\"h\"></div>\n@match $k { \"a\" &ta($k); } }\n<main></main>",
        )
        .expect("parse");
        assert!(
            !f.matches.iter().any(|m| m.macro_name == "match"),
            "a malformed @match must not classify as a flat match construct"
        );
        // And the broken @match text must not leak into the template body HTML.
        // FEAT-119: html lives on the World-A template SCOPE, not the capture.
        let html = f
            .scopes
            .iter()
            .find(|s| s.selector.starts_with("@template:"))
            .map(|s| s.html.clone())
            .unwrap_or_default();
        assert!(
            !html.contains("@match"),
            "malformed @match must not leak into html, got: {:?}",
            html
        );
    }

    #[test]
    fn feat073_match_parses_via_stdlib_grammar() {
        // FEAT-115 S3d: @match is a flat `%macro match` (the render-once sibling of
        // @view) parsed by the stdlib match_block %capture_type grammar — not the
        // reify ComponentMatch. Assert the flat match surfaces with subject + arms
        // (incl. `_` wildcard) via the full parse path.
        let f = crate::parse(
            "@template &c($kind) { <div class=\"h\"></div>\n@match $kind { \"a\" => &ta($kind); \"b\" => &tb($kind); _ => &tc($kind); } }\n<main></main>",
        )
        .expect("parse");
        let mm: Vec<_> = f
            .matches
            .iter()
            .filter(|m| m.macro_name == "match")
            .collect();
        assert_eq!(
            mm.len(),
            1,
            "one flat @match construct, got {:?}",
            f.matches.iter().map(|m| &m.macro_name).collect::<Vec<_>>()
        );
        let m = mm[0];
        match m.captures.get("subject") {
            Some(crate::syntax::CapturedValue::Binding(s)) => {
                assert!(s.contains("kind"), "subject is $kind, got {:?}", s)
            }
            other => panic!("expected a binding subject, got {:?}", other),
        }
        match m.captures.get("arms") {
            Some(crate::syntax::CapturedValue::Array(arms)) => {
                assert_eq!(arms.len(), 3, "three arms")
            }
            other => panic!("expected an arms array, got {:?}", other),
        }
    }
    #[test]
    fn plan077_w2_derive_match_form_parses_all_type_shapes() {
        // PLAN-077 W2: `@data derive $x T : @match { (g) => V; _ => G; }` matches
        // the data-derive-match form (NOT plain data-derive) across all three
        // typeref shapes — named @type, absent, inline anonymous union — with
        // the cond_block arms record intact (guard: "_" | {expr}, cons: {name}).
        for (label, src, expect_type) in [
            (
                "untyped",
                "@data derive $badge : @match { ($isOrange) => Orange; _ => Green; }\n.dm { color: red; }",
                false,
            ),
            (
                "named",
                "@data derive $badge BadgeState : @match { ($isOrange) => Orange; _ => Green; }\n.dm { color: red; }",
                true,
            ),
            (
                "inline-union",
                "@data derive $badge (Blue | Orange | Green) : @match { ($isOrange) => Orange; _ => Green; }\n.dm { color: red; }",
                true,
            ),
        ] {
            let f = crate::parse(src).unwrap_or_else(|_| panic!("{} parses", label));
            let m = f
                .matches
                .iter()
                .find(|m| m.matched_macro.as_deref() == Some("data-derive-match"))
                .unwrap_or_else(|| {
                    panic!(
                        "{}: data-derive-match must match (got {:?})",
                        label,
                        f.matches
                            .iter()
                            .map(|m| &m.matched_macro)
                            .collect::<Vec<_>>()
                    )
                });
            match m.captures.get("match") {
                Some(crate::syntax::CapturedValue::Named(block)) => match block.get("arms") {
                    Some(crate::syntax::CapturedValue::Array(arms)) => {
                        assert_eq!(arms.len(), 2, "{}: two arms", label)
                    }
                    other => panic!("{}: expected arms array, got {:?}", label, other),
                },
                other => panic!("{}: expected cond_block record, got {:?}", label, other),
            }
            if expect_type {
                assert!(
                    m.captures.contains_key("type") || m.captures.contains_key("uitype"),
                    "{}: a type capture must be present",
                    label
                );
            }
            // The plain data-derive form must NOT also claim the line.
            assert_eq!(
                f.matches
                    .iter()
                    .filter(|m| m.matched_macro.as_deref() == Some("data-derive-match"))
                    .count(),
                1,
                "{}: exactly one data-derive-match",
                label
            );
        }
    }

    #[test]
    fn plan077_w2_payload_arm_cons_record_shape() {
        // PLAN-077 W2 gate-2: a payload-carrying constructor captures as
        // `{ name, payload: [{ arg }] }` — the key is `payload`, NOT `args`:
        // `{name, args}` is the serializer's template-invocation-map shape
        // (`is_template_invocation_map`, expand.rs), which array-wraps nested
        // records for the `a.inv[0]` contract. A variant constructor is not a
        // template invocation — distinct data, distinct keys.
        let f = crate::parse(
            "@data derive $fs FetchState : @match { ($e != \"\") => Failed($e); _ => Idle; }\n.dm { color: red; }\n",
        )
        .expect("parse");
        let m = f
            .matches
            .iter()
            .find(|m| m.matched_macro.as_deref() == Some("data-derive-match"))
            .expect("data-derive-match matches");
        let Some(crate::syntax::CapturedValue::Named(block)) = m.captures.get("match") else {
            panic!("expected cond_block record")
        };
        let Some(crate::syntax::CapturedValue::Array(arms)) = block.get("arms") else {
            panic!("expected arms array")
        };
        let crate::syntax::CapturedValue::Named(arm0) = &arms[0] else {
            panic!("arm 0 is a record")
        };
        match arm0.get("cons") {
            Some(crate::syntax::CapturedValue::Named(cons)) => {
                assert!(
                    matches!(cons.get("name"), Some(crate::syntax::CapturedValue::Ident(n)) if n == "Failed"),
                    "cons.name is Failed, got {:?}",
                    cons.get("name")
                );
                match cons.get("payload") {
                    Some(crate::syntax::CapturedValue::Array(payload)) => {
                        assert_eq!(payload.len(), 1, "one payload arg");
                        assert!(
                            matches!(&payload[0], crate::syntax::CapturedValue::Named(a) if matches!(a.get("arg"), Some(crate::syntax::CapturedValue::Expr(e)) if e == "$e")),
                            "payload[0].arg is the $e expr, got {:?}",
                            payload[0]
                        );
                    }
                    other => panic!("cons.payload must be an Array, got {:?}", other),
                }
                assert!(
                    !cons.contains_key("args"),
                    "cons must NOT use the invocation-map key `args`"
                );
            }
            other => panic!(
                "arm 0 cons must be a Named record (NOT array-wrapped), got {:?}",
                other
            ),
        }
    }

    #[test]
    fn plan077_w1_match_pat_variant_branch_parses() {
        // PLAN-077 W1: match_pat gained a `variant` branch (destructure pattern,
        // stdlib/enum/capture-types/variant-pattern.st) between `lit` and `wild`.
        // The new shape parses through the LIVE @match form; old shapes keep
        // their EXACT record contract ({lit}/{wild}) for match-render/view.st.
        let f = crate::parse(
            "@template &c($kind) { <div class=\"h\"></div>\n@match $kind { \"a\" => &ta($kind); Connected { $send, $received } => &tb($send); _ => &tc($kind); } }\n<main></main>",
        )
        .expect("parse");
        let mm: Vec<_> = f
            .matches
            .iter()
            .filter(|m| m.macro_name == "match")
            .collect();
        assert_eq!(mm.len(), 1, "one flat @match construct");
        let arms = match mm[0].captures.get("arms") {
            Some(crate::syntax::CapturedValue::Array(arms)) => arms,
            other => panic!("expected an arms array, got {:?}", other),
        };
        assert_eq!(arms.len(), 3, "string + variant + wildcard arms");

        // Arm 1: string literal pattern — the pre-PLAN-077 {lit} shape, verbatim.
        let pat1 = arm_pat(arms, 0);
        assert!(
            pat1.get("lit").is_some()
                && pat1.get("wild").is_none()
                && pat1.get("variant").is_none(),
            "string arm keeps the {{lit}} record shape, got {:?}",
            pat1
        );

        // Arm 2: the NEW variant destructure pattern — {variant:{name, bindings}}.
        let pat2 = arm_pat(arms, 1);
        match pat2.get("variant") {
            Some(crate::syntax::CapturedValue::Named(v)) => {
                match v.get("name") {
                    Some(crate::syntax::CapturedValue::Ident(n)) => {
                        assert_eq!(n, "Connected", "variant ctor name")
                    }
                    other => panic!("expected variant name ident, got {:?}", other),
                }
                match v.get("bindings") {
                    Some(crate::syntax::CapturedValue::Array(bs)) => {
                        assert_eq!(bs.len(), 2, "two destructured bindings, got {:?}", bs)
                    }
                    other => panic!("expected a bindings array, got {:?}", other),
                }
            }
            other => panic!("expected a {{variant}} record, got {:?}", other),
        }

        // Arm 3: the `_` wildcard — the pre-PLAN-077 {wild} shape, verbatim
        // (NOT routed to the variant branch despite `_` being an ident).
        let pat3 = arm_pat(arms, 2);
        match pat3.get("wild") {
            Some(crate::syntax::CapturedValue::Ident(w)) => {
                assert_eq!(w, "_", "wildcard stays on the wild branch")
            }
            other => panic!("expected {{wild: \"_\"}}, got {:?}", other),
        }
        assert!(
            pat3.get("variant").is_none(),
            "wildcard must not route to the variant branch"
        );
    }

    /// Extract the `pat` record of the nth arm as a Named map (panics otherwise).
    fn arm_pat(
        arms: &[crate::syntax::CapturedValue],
        n: usize,
    ) -> &std::collections::HashMap<String, crate::syntax::CapturedValue> {
        match &arms[n] {
            crate::syntax::CapturedValue::Named(arm) => match arm.get("pat") {
                Some(crate::syntax::CapturedValue::Named(pat)) => pat,
                other => panic!("arm {} has no pat record: {:?}", n, other),
            },
            other => panic!("arm {} is not a record: {:?}", n, other),
        }
    }
}

#[cfg(test)]
mod proj104_tests {
    use super::*;
    use crate::syntax::form_match::ExportDecl;

    /// Test shim (FEAT-079): the Rust parse_component_body was deleted; these integration tests
    /// assert the ComponentBodyDef from the live self-describing producer (validate_component_body).
    fn parse_component_body(inner: &str) -> Vec<crate::diagnostics::Diagnostic> {
        validate_component_body(inner, 0, "test")
    }

    // =========================================================================
    // PROJ-104: Template Refs & @exports — Parse Tests
    // =========================================================================

    #[test]
    fn test_parse_template_ref_anonymous() {
        let result = parse_template_ref(r#"&counter("Likes")"#);
        assert!(result.is_some(), "Should parse anonymous template ref");
        let r = result.unwrap();
        assert_eq!(r.ref_name, None);
        assert_eq!(r.template_name, "counter");
        assert!(!r.is_collection);
    }

    #[test]
    fn test_parse_template_ref_named() {
        let result = parse_template_ref(r#"&likeCounter &counter("Likes")"#);
        assert!(result.is_some(), "Should parse named template ref");
        let r = result.unwrap();
        assert_eq!(r.ref_name, Some("likeCounter".to_string()));
        assert_eq!(r.template_name, "counter");
        assert!(!r.is_collection);
    }

    #[test]
    fn test_parse_template_ref_collection() {
        let result = parse_template_ref("&cards[] &counter($item)");
        assert!(result.is_some(), "Should parse collection template ref");
        let r = result.unwrap();
        assert_eq!(r.ref_name, Some("cards".to_string()));
        assert_eq!(r.template_name, "counter");
        assert!(r.is_collection);
    }

    #[test]
    fn test_parse_template_ref_with_semicolon() {
        let result = parse_template_ref("&nav &nav-shell();");
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.ref_name, Some("nav".to_string()));
        assert_eq!(r.template_name, "nav-shell");
    }

    // =========================================================================
    // Selector ref parsing tests
    // =========================================================================

    #[test]
    fn test_parse_exports_block_readonly() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { $count }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].var_name, "count");
        assert!(!exports[0].mutable);
    }

    #[test]
    fn test_parse_exports_block_mutable() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { $count: mut }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].var_name, "count");
        assert!(exports[0].mutable);
    }

    #[test]
    fn test_parse_exports_block_multiple() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { $count; $label: mut }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert_eq!(exports.len(), 2);
        assert_eq!(exports[0].var_name, "count");
        assert!(!exports[0].mutable);
        assert_eq!(exports[1].var_name, "label");
        assert!(exports[1].mutable);
    }

    #[test]
    fn test_component_body_with_refs() {
        // FEAT-115 S3d: body template invocations are sourced from the template SCOPE's
        // invoke matches + harvest (cst_to_stfile), not reify. Assert the named ref
        // surfaces on the parsed template's body.refs via the full parse path.
        let f = crate::parse(
            "@template &counter($l) { <b>x</b> }\n@template &c() { <nav></nav>\n&likeCounter &counter(\"Likes\"); }\n<main></main>",
        )
        .expect("parse");
        // FEAT-119: refs are read from the World-A template SCOPE (`@template:<name>`),
        // not the retired ComponentBody capture (payload fields gone).
        let refs = f
            .scopes
            .iter()
            .filter(|s| s.selector.starts_with("@template:") && !s.refs.is_empty())
            .map(|s| s.refs.clone())
            .next()
            .unwrap_or_default();
        assert_eq!(refs.len(), 1, "one named ref, got {:?}", refs);
        assert_eq!(refs[0].ref_name, Some("likeCounter".to_string()));
        assert_eq!(refs[0].template_name, "counter");
    }

    #[test]
    fn test_component_body_with_exports() {
        // FEAT-115 S2: exports value-semantics live in parse_exports_block (the single
        // parser, now called from cst_to_stfile's `@exports` harvest instead of reify).
        let mut exports = Vec::new();
        let mut diags = Vec::new();
        parse_exports_block(
            "@exports { $count: mut }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].var_name, "count");
        assert!(exports[0].mutable);
        assert!(diags.is_empty());
    }
}

#[cfg(test)]
mod pdc_diagnostic_tests {
    use super::*;
    use crate::syntax::form_match::ExportDecl;

    /// Test shim (FEAT-079): the Rust parse_component_body was deleted; these diagnostic tests
    /// assert the ComponentBodyDef from the live self-describing producer (validate_component_body).
    fn parse_component_body(inner: &str) -> Vec<crate::diagnostics::Diagnostic> {
        validate_component_body(inner, 0, "test")
    }

    // =========================================================================
    // PDC-002: E0900 — Invalid content in component body
    // =========================================================================

    #[test]
    fn test_e0900_bare_dollar_var_triggers_diagnostic() {
        // $broken without a type is not a valid state declaration
        // and should trigger E0900
        let body = parse_component_body("<div>Hello</div>\n$broken");
        assert!(
            body.iter().any(|d| d.code.as_str() == "E0900"),
            "Bare $variable without type should trigger E0900, got: {:?}",
            body
        );
    }

    #[test]
    fn test_e0900_bare_ampersand_triggers_diagnostic() {
        // &invalid (not a valid template ref pattern) should trigger E0900
        let body = parse_component_body("<div>Hello</div>\n&broken");
        assert!(
            body.iter().any(|d| d.code.as_str() == "E0900"),
            "Bare &identifier without template call should trigger E0900, got: {:?}",
            body
        );
    }

    #[test]
    fn test_e0900_valid_html_no_diagnostic() {
        // Pure HTML should not trigger E0900
        let body = parse_component_body("<div><p>Hello World</p></div>");
        assert!(
            body.is_empty(),
            "Valid HTML should produce no diagnostics, got: {:?}",
            body
        );
    }

    #[test]
    fn test_e0900_valid_state_decl_no_diagnostic() {
        // Valid state declaration should not trigger E0900
        let body = parse_component_body("<div></div>\n$count number: 0;");
        assert!(
            body.is_empty(),
            "Valid state declaration should produce no E0900, got: {:?}",
            body
        );
    }

    #[test]
    fn test_e0900_valid_mixed_body_no_diagnostic() {
        // Full valid component body should not trigger E0900
        let body = parse_component_body(
            "<div></div>\n$open bool: false;\n.active: $open;\ntext <- $label;\n@exports { $open: mut }",
        );
        assert!(
            body.is_empty(),
            "Valid component body should produce no E0900, got: {:?}",
            body
        );
    }

    #[test]
    fn test_e0900_continues_parsing() {
        // E0900 should be non-fatal — parsing continues after the invalid content.
        // (FEAT-115: the valid `$count` decl is now scope-sourced for the payload; the
        // reify witness is that it is segmented out of `html` and the E0900 still fires.)
        let body = parse_component_body("<div>Hello</div>\n$broken\n$count number: 0;");
        // Should have the E0900 diagnostic (reify is a validator now — no html payload)
        assert!(body.iter().any(|d| d.code.as_str() == "E0900"));
    }

    // =========================================================================
    // PDC-003: E0904 — Malformed @exports declaration
    // =========================================================================

    #[test]
    fn test_e0904_export_without_dollar_sign() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { not_a_var }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert!(exports.is_empty(), "Malformed export should not be added");
        assert!(
            diags.iter().any(|d| d.code.as_str() == "E0904"),
            "Export without $ should trigger E0904, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_e0904_valid_export_no_diagnostic() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { $count; $label: mut }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert_eq!(exports.len(), 2);
        assert!(
            diags.is_empty(),
            "Valid exports should produce no E0904, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_e0904_mixed_valid_and_invalid() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { $count; invalid; $label: mut }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert_eq!(exports.len(), 2, "Valid exports should still be parsed");
        assert_eq!(diags.len(), 1, "Should have exactly one E0904");
        assert_eq!(diags[0].code.as_str(), "E0904");
    }

    #[test]
    fn test_e0904_hint_shows_correct_syntax() {
        let mut exports: Vec<ExportDecl> = Vec::new();
        let mut diags: Vec<crate::diagnostics::Diagnostic> = Vec::new();
        parse_exports_block(
            "@exports { broken }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert!(!diags.is_empty());
        let hint = diags[0].hint.as_deref().unwrap_or("");
        assert!(
            hint.contains("$varName"),
            "E0904 hint should show correct syntax with $varName, got: {}",
            hint
        );
    }

    #[test]
    fn test_e0904_via_component_body() {
        // FEAT-115 S2: E0904 (malformed export) is raised by parse_exports_block, the
        // single exports parser the `@exports` harvest in cst_to_stfile drives.
        let mut exports = Vec::new();
        let mut diags = Vec::new();
        parse_exports_block(
            "@exports { broken_entry }",
            &mut exports,
            &mut diags,
            Default::default(),
        );
        assert!(
            diags.iter().any(|d| d.code.as_str() == "E0904"),
            "Malformed @exports must trigger E0904, got: {:?}",
            diags
        );
    }

    // === SC-002: StateAction parsing tests ===

    #[test]
    fn test_transpile_signal_expr_basic() {
        use crate::syntax::transpile_signal_expr;
        assert_eq!(
            transpile_signal_expr("$count + 1"),
            "ST.get(__el, 'count') + 1"
        );
        // Distinct identifiers, string literal left intact.
        assert_eq!(
            transpile_signal_expr("$a + $ab + 'a $x'"),
            "ST.get(__el, 'a') + ST.get(__el, 'ab') + 'a $x'"
        );
    }

    #[test]
    fn test_transpile_signal_expr_global_scope() {
        use crate::syntax::{SignalScope, transpile_signal_expr_with};
        // Global scope reads from SpacetimeLocal (file-scope reactive bindings).
        assert_eq!(
            transpile_signal_expr_with("$cartTotal", SignalScope::Global),
            "SpacetimeLocal['cartTotal']"
        );
        assert_eq!(
            transpile_signal_expr_with("$a * 2 + $b", SignalScope::Global),
            "SpacetimeLocal['a'] * 2 + SpacetimeLocal['b']"
        );
    }

    #[test]
    fn test_state_action_no_assignment_emits_e0906() {
        // FEAT-115 S6: reify no longer classifies actions into a StateAction IR;
        // validate_on_action is the diagnostics-only probe. A body with no
        // recognized action (assignment, keyframe, emit, or signal-call) emits
        // E0906. BUG-166 widened acceptance beyond assignment-only (see below) —
        // "some random body" still has none of those shapes, so it still errors.
        let diag = validate_on_action(
            "some random body",
            crate::diagnostics::SourceSpan::default(),
        );
        assert!(diag.is_some(), "No recognized action should emit E0906");
        assert_eq!(diag.unwrap().code.as_str(), "E0906");
        // A well-formed assignment is clean.
        assert!(
            validate_on_action("$x <- $x + 1;", crate::diagnostics::SourceSpan::default())
                .is_none()
        );
    }

    #[test]
    fn test_on_body_accepts_animation_only_keyframe_statement() {
        // BUG-166: an @on body with ONLY a keyframe/transition statement (no
        // `$var <- expr`) must be accepted — it was always valid at page level
        // and always rendered correctly via the World-A path; only this
        // diagnostic-only validator was wrongly strict.
        assert!(
            validate_on_action(
                "opacity: 0 -> 1;",
                crate::diagnostics::SourceSpan::default()
            )
            .is_none(),
            "a bare keyframe statement must be accepted"
        );
    }

    #[test]
    fn test_on_body_accepts_scoped_ampersand_keyframe_wrapper() {
        // BUG-166's exact repro shape from stdlib/macros/template.st's own doc
        // example: `& { opacity: 0 -> 1; translate-y: 20px -> 0; }`.
        assert!(
            validate_on_action(
                "& { opacity: 0 -> 1; translate-y: 20px -> 0; }",
                crate::diagnostics::SourceSpan::default()
            )
            .is_none(),
            "a scoped `& {{ ... }}` keyframe wrapper must be accepted"
        );
    }

    #[test]
    fn test_on_body_accepts_emit_only_action() {
        assert!(
            validate_on_action("@emit clicked;", crate::diagnostics::SourceSpan::default())
                .is_none(),
            "a bare @emit action must be accepted"
        );
    }

    #[test]
    fn test_on_body_accepts_mixed_assignment_and_keyframe() {
        assert!(
            validate_on_action(
                "$open <- !$open; & { scale: 1 -> 0.95 -> 1; }",
                crate::diagnostics::SourceSpan::default()
            )
            .is_none(),
            "a mixed assignment+keyframe body must be accepted"
        );
    }

    #[test]
    fn test_on_body_rejects_unrecognized_statement() {
        // Widening acceptance must not become "accept everything" — a statement
        // matching none of the on_action grammar arms (mutation/emit/signal-call/
        // keyframe) still E0906s.
        let diag = validate_on_action("console.log(1);", crate::diagnostics::SourceSpan::default());
        assert!(
            diag.is_some(),
            "an unrecognized statement must still emit E0906"
        );
        assert_eq!(diag.unwrap().code.as_str(), "E0906");
    }

    #[test]
    fn test_on_body_rejects_empty() {
        let diag = validate_on_action("", crate::diagnostics::SourceSpan::default());
        assert!(diag.is_some(), "an empty body must still emit E0906");
    }

    #[test]
    fn test_component_body_media_recognized_no_diagnostic() {
        // BUG-167: a root `@media (…) { … }` in a @template body must be recognized
        // by the component_item grammar (cb_media arm) and produce ZERO diagnostics
        // (it is a pure-consume construct here — the CSS is hoisted at the
        // src/parser/mod.rs layer, out of this diagnostics-only validator's scope).
        let diags = validate_component_body(
            "<div class=\"mq\">x</div>\n@media (max-width: 700px) { .mq { color: green; } }",
            0,
            "test",
        );
        assert!(
            diags.is_empty(),
            "a recognized @media construct must not emit any diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_component_body_unknown_directive_rejected_with_registry() {
        // BUG-167: with a registry available, a GENUINELY unknown root directive
        // (zero forms registered for `@name`) surfaces E0900 instead of the old
        // unconditional silent drop.
        let reg = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
        let diags = validate_component_body_with_registry(
            "<div>x</div>\n@totallyBogusDirectiveThatDoesNotExist(foo: 1) { something: 1; }",
            0,
            "test",
            Some(reg),
        );
        assert!(
            diags.iter().any(|d| d.code.as_str() == "E0900"),
            "an unknown directive must emit E0900 when a registry is available, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_component_body_known_directive_stays_silent_with_registry() {
        // BUG-167 fix must NOT over-widen: a directive the registry DOES know
        // (matched by the wider macro-form system rather than a `component_item`
        // arm) must stay silent — not become a false E0900. This is the exact
        // risk the fix's acceptance criteria call out: many real directives rely
        // on that silent path.
        //
        // Probe directive is `@portal` (`stdlib/macros/portal.st`, `%scope
        // selector`). It was `@effect` until BUG-347 — but @effect was DELETED
        // in 0000c27d (retired onto `@on $sig.change`), so E0900 became the
        // CORRECT answer for it and this test could never pass again. A test
        // pinning a removed surface detects nothing; the guard itself is still
        // worth having, so it moves to a directive that actually exists.
        let reg = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
        let diags = validate_component_body_with_registry(
            "$open boolean: false;\n<div class=\"e\">x</div>\n@portal(when: $open);",
            0,
            "test",
            Some(reg),
        );
        assert!(
            !diags.iter().any(|d| d.code.as_str() == "E0900"),
            "a KNOWN directive (@portal) must not emit E0900, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_component_body_without_registry_stays_fully_permissive() {
        // The `registry: None` path (bootstrap / callers with no registry in hand)
        // must preserve the OLD fully-permissive behavior — no E0900 for an unknown
        // directive when there is nothing to check it against.
        let diags = validate_component_body(
            "<div>x</div>\n@totallyBogusDirectiveThatDoesNotExist(foo: 1) { something: 1; }",
            0,
            "test",
        );
        assert!(
            !diags.iter().any(|d| d.code.as_str() == "E0900"),
            "registry: None must stay permissive (old behavior), got: {:?}",
            diags
        );
    }

    #[test]
    fn test_state_action_expression_via_component_body_is_clean() {
        // A signal expression in an @on handler must NOT emit E0906 — it is a
        // supported reactive mutation.
        let body = parse_component_body("<div></div>\n$x number: 0;\n@on click { $x <- $x + $y; }");
        assert!(
            !body.iter().any(|d| d.code.as_str() == "E0906"),
            "Signal expressions must be supported (no E0906), got: {:?}",
            body
        );
    }
    #[test]
    fn feat104_merge_single_capture_group_binds_scalar() {
        // A single-capture group `( $s:string )?` compiles to a SCALAR (not Named);
        // merge_group_captures must still bind it under the inner capture name.
        use crate::parser::meta_ast::{CaptureModifier, CapturePatternAst, CaptureType};
        let group = CapturePatternAst::Group {
            pattern: Box::new(CapturePatternAst::Capture {
                var_name: "s".to_string(),
                capture_type: CaptureType::String,
                modifier: CaptureModifier::Required,
            }),
            modifier: Some(CaptureModifier::Optional),
        };
        let mut caps = HashMap::new();
        super::merge_group_captures(&group, CapturedValue::String("hi".to_string()), &mut caps);
        assert_eq!(
            caps.get("s"),
            Some(&CapturedValue::String("hi".to_string()))
        );
    }

    #[test]
    fn feat104_merge_repeat_group_binds_array() {
        // A repeat group `( $row:ident )*` yields an Array; bind it under the inner name.
        use crate::parser::meta_ast::{CaptureModifier, CapturePatternAst, CaptureType};
        let group = CapturePatternAst::Group {
            pattern: Box::new(CapturePatternAst::Capture {
                var_name: "row".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
            }),
            modifier: Some(CaptureModifier::ZeroOrMore),
        };
        let mut caps = HashMap::new();
        let arr = CapturedValue::Array(vec![CapturedValue::Ident("a".to_string())]);
        super::merge_group_captures(&group, arr.clone(), &mut caps);
        assert_eq!(caps.get("row"), Some(&arr));
    }

    #[test]
    fn feat104_merge_named_group_lifts_all_keys() {
        // A multi-element Sequence group yields Named; all keys lift to top level.
        use crate::parser::meta_ast::CapturePatternAst;
        let group = CapturePatternAst::Group {
            pattern: Box::new(CapturePatternAst::Sequence(vec![])),
            modifier: None,
        };
        let mut named = HashMap::new();
        named.insert("a".to_string(), CapturedValue::String("1".to_string()));
        named.insert("b".to_string(), CapturedValue::String("2".to_string()));
        let mut caps = HashMap::new();
        super::merge_group_captures(&group, CapturedValue::Named(named), &mut caps);
        assert_eq!(caps.get("a"), Some(&CapturedValue::String("1".to_string())));
        assert_eq!(caps.get("b"), Some(&CapturedValue::String("2".to_string())));
    }
}

#[cfg(test)]
mod bug127_property_value_tests {
    use super::*;

    /// BUG-127: `convert_property_value` used `trim_matches('"')`, which dropped a
    /// quote from any expression value that merely started/ended with one. A
    /// `@data query { where: $.col == "todo" }` predicate became `$.col == "todo`
    /// (closing quote lost) → invalid JS → the filter silently matched everything.
    #[test]
    fn expr_value_ending_in_quoted_string_keeps_both_quotes() {
        let v = convert_property_value("$.col == \"todo\"", &CaptureType::Expr);
        match v {
            CapturedValue::Expr(s) => assert_eq!(s, "$.col == \"todo\""),
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    #[test]
    fn expr_value_starting_with_quoted_string_keeps_both_quotes() {
        let v = convert_property_value("\"todo\" == $.col", &CaptureType::Expr);
        match v {
            CapturedValue::Expr(s) => assert_eq!(s, "\"todo\" == $.col"),
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// A WHOLE string literal value IS still unwrapped (the original intent).
    #[test]
    fn whole_string_literal_is_unwrapped() {
        let v = convert_property_value("\"hello\"", &CaptureType::String);
        match v {
            CapturedValue::String(s) => assert_eq!(s, "hello"),
            other => panic!("expected String, got {other:?}"),
        }
    }

    /// Two adjacent string literals must NOT be treated as one wrapping pair.
    #[test]
    fn two_string_literals_not_unwrapped() {
        let v = convert_property_value("\"a\" == \"b\"", &CaptureType::Expr);
        match v {
            CapturedValue::Expr(s) => assert_eq!(s, "\"a\" == \"b\""),
            other => panic!("expected Expr, got {other:?}"),
        }
    }

    /// BUG-138: a brace-body `:duration`/`:time` capture (`{ refresh: 5m }`) must
    /// convert the literal token to a number of MILLISECONDS (`Time(ms)`), matching
    /// the token-stream and param-default paths — not store the raw string. Without
    /// this, `@data fetch { refresh: 5m }` emitted `refreshInterval = "5m"` and the
    /// data-source guard `if (refreshInterval > 0)` no-opped.
    #[test]
    fn duration_literal_converts_to_ms() {
        for (input, expected) in [("5m", 300_000u32), ("300ms", 300), ("2s", 2000)] {
            match convert_property_value(input, &CaptureType::Duration) {
                CapturedValue::Time(ms) => assert_eq!(ms, expected, "duration {input}"),
                other => panic!("expected Time for {input:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn time_literal_converts_to_ms() {
        match convert_property_value("300ms", &CaptureType::Time) {
            CapturedValue::Time(ms) => assert_eq!(ms, 300),
            other => panic!("expected Time, got {other:?}"),
        }
    }

    /// A non-literal duration value (a binding/expr) cannot parse — it must fall
    /// back to `Expr` so `{ refresh: $userInterval }` still threads through.
    #[test]
    fn non_literal_duration_falls_back_to_expr() {
        match convert_property_value("$userInterval", &CaptureType::Duration) {
            CapturedValue::Expr(s) => assert_eq!(s, "$userInterval"),
            other => panic!("expected Expr fallback, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod split_semicolons_tests {
    use super::{split_statements_outside_strings, split_statements_outside_strings_raw};

    #[test]
    fn splits_plain_statements() {
        assert_eq!(
            split_statements_outside_strings("$a <- 1; $b <- 2"),
            vec!["$a <- 1", "$b <- 2"]
        );
    }

    #[test]
    fn semicolon_inside_double_quotes_is_content() {
        // BUG-225: prose semicolons severed the literal mid-string.
        let src = r#"$v <- "false for nobody; every owner wants control""#;
        assert_eq!(split_statements_outside_strings(src), vec![src]);
    }

    #[test]
    fn semicolon_inside_single_quotes_and_backticks_is_content() {
        let sq = r#"$v <- 'a; b'"#;
        assert_eq!(split_statements_outside_strings(sq), vec![sq]);
        let bt = "$v <- `a; b`";
        assert_eq!(split_statements_outside_strings(bt), vec![bt]);
    }

    #[test]
    fn escaped_quote_does_not_end_the_literal() {
        let src = r#"$v <- "she said \"a; b\" loudly""#;
        assert_eq!(split_statements_outside_strings(src), vec![src]);
    }

    #[test]
    fn apostrophe_inside_double_quotes_is_content() {
        // A lone `'` must not open a literal when inside a `"` string.
        let src = r#"$v <- "nobody's record; really"; $w <- 2"#;
        assert_eq!(
            split_statements_outside_strings(src),
            vec![r#"$v <- "nobody's record; really""#, "$w <- 2"]
        );
    }

    #[test]
    fn mixed_literal_and_separator() {
        let src = r#"$a <- "x; y"; $b <- 2; $c <- "p;q""#;
        assert_eq!(
            split_statements_outside_strings(src),
            vec![r#"$a <- "x; y""#, "$b <- 2", r#"$c <- "p;q""#]
        );
    }

    #[test]
    fn trimmed_variant_drops_empties() {
        assert_eq!(
            split_statements_outside_strings("$a <- 1;;  ; $b <- 2;"),
            vec!["$a <- 1", "$b <- 2"]
        );
    }

    #[test]
    fn raw_variant_round_trips_byte_identically() {
        for src in [
            "$a <- 1; $b <- 2",
            r#"$a <- "x; y"; $b <- 2"#,
            "$a <- 1;;  ; $b <- 2;",
            "",
            "no mutations here",
        ] {
            assert_eq!(
                split_statements_outside_strings_raw(src).join(";"),
                src,
                "round-trip failed for {src:?}"
            );
        }
    }
}
#[test]
fn a_bare_number_is_rejected_where_a_duration_is_expected() {
    // The strict-capture half of the near-miss guard: `refresh: 5` must
    // FAIL the productive duration capture so the error sibling can fire,
    // instead of being leniently accepted and silently ignored.
    assert!(is_bare_number_duration_reject("5", &CaptureType::Duration));
    assert!(is_bare_number_duration_reject("5", &CaptureType::Time));
    assert!(is_bare_number_duration_reject("  5.5  ", &CaptureType::Duration));
}

#[test]
fn wrong_kind_values_are_rejected_where_a_duration_is_expected() {
    // Reviewer 67.0: the guard used to reject only f64-PARSEABLE values, so
    // `refresh: 5px` / `refresh: foo` / `refresh: 5 ms` converted to Expr,
    // bound cleanly, and silently dropped (the timer never started). Anything
    // that is neither a duration literal nor a `$` signal reference is a
    // near-miss and must reject so the error sibling reports it.
    assert!(is_bare_number_duration_reject("5px", &CaptureType::Duration));
    assert!(is_bare_number_duration_reject("foo", &CaptureType::Duration));
    assert!(is_bare_number_duration_reject("5 ms", &CaptureType::Duration));
    assert!(is_bare_number_duration_reject("auto", &CaptureType::Time));
    assert!(is_bare_number_duration_reject("7em", &CaptureType::Duration));
}

#[test]
fn zero_is_not_rejected_where_a_duration_is_expected() {
    // Reviewer 67.0: ZERO needs no unit (CSS convention) and is the stdlib's
    // own no-auto-refresh sentinel (`refresh: 0` — data-kind.st:82,482), so it
    // must bind cleanly, not trip the strict bare-number rejection.
    assert!(!is_bare_number_duration_reject("0", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject("0", &CaptureType::Time));
    assert!(!is_bare_number_duration_reject("0.0", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject(" 0 ", &CaptureType::Duration));
}

#[test]
fn real_durations_and_non_durations_are_not_rejected() {
    // The negative space: a value that IS a duration passes, a `$` signal
    // reference passes (resolved at runtime), and captures that are not
    // durations never consult this rule at all.
    assert!(!is_bare_number_duration_reject("5s", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject("300ms", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject("1m", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject("$every", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject("$userInterval", &CaptureType::Duration));
    assert!(!is_bare_number_duration_reject("5", &CaptureType::Number));
    assert!(!is_bare_number_duration_reject("5", &CaptureType::String));
}


/// Advance a byte offset past leading whitespace and comments so a form
/// match's span starts at the directive itself, not the trivia preceding it.
/// `span_start` is the offset when the node OPENED, which for a directive that
/// follows another is the previous line's trailing newline (GH-27: a second
/// `@data` anchored to col 27 of the first line). Trivia is never code, so
/// stopping at the first non-trivia byte lands on the directive's own `@`/`&`.
fn skip_leading_trivia(source: &str, mut start: usize) -> usize {
    let b = source.as_bytes();
    loop {
        while start < b.len() {
            let c = b[start] as char;
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                start += 1;
            } else {
                break;
            }
        }
        if start + 1 < b.len() && b[start] == b'/' && b[start + 1] == b'/' {
            // Line comment: skip to end of line.
            while start < b.len() && b[start] as char != '\n' {
                start += 1;
            }
            continue;
        }
        if start + 1 < b.len() && b[start] == b'/' && b[start + 1] == b'*' {
            start += 2;
            while start + 1 < b.len() && !(b[start] == b'*' && b[start + 1] == b'/') {
                start += 1;
            }
            start = (start + 2).min(b.len());
            continue;
        }
        break;
    }
    start
}
