//! MatchSink — Layer 3 of the MatchSink architecture.
//!
//! Implements the `Sink` trait to collect tokens per node, manage a context
//! stack, and dispatch to `CompiledForm::try_match()` on `finish_node()`.
//!
//! # Architecture
//!
//! ```text
//! Events → process() → MatchSink
//!   start_node → push NodeContext
//!   token      → collect into current context
//!   finish_node → pop context, try_match if matchable
//!   error      → collect diagnostic
//! ```

use crate::parser::meta_ast::MacroScope;
use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::FormMatch;

use super::extractors::ExtractorRegistry;
use super::form_compiler::{CompiledRegistry, FailureKind, MatchFailure, NodeContext};
use super::sink::Sink;

/// A directive whose shape matched no form is dropped — the BUG-216 family. The
/// DIRECTIVE branch of `committed_then_failed` reports a form that committed to
/// a grammar and then failed inside it. A `$` VARIABLE_REF has the same
/// silent-drop risk for DECLARATION-shaped source (`$user object: {…};` — I3 /
/// gh-35), but a ref is ALSO the normal spelling of an expression read
/// (`text <- $user.name`), which must NOT error. A ref only reports when the form
/// committed past its value separator: the `$name <type> :` run matched and the
/// failure is on the VALUE/terminator (or leftover content remains). This is the
/// consume-or-refuse line for declaration-shaped refs — the statement's bounded
/// region was not fully consumed.
pub fn ref_committed_declaration(
    registry: &CompiledRegistry,
    failure: &MatchFailure,
    prefix: char,
) -> bool {
    use crate::parser::meta_ast::FormInlineElement;
    // Leftover content after the full grammar matched is always a dropped clause.
    if matches!(failure.kind, FailureKind::ExtraInlineTokens { .. }) {
        return true;
    }
    // Otherwise the failure is a capture/missing-body at `failure.element_index`.
    // Only report when the form got PAST its value separator (`:` literal): a
    // declaration shape. A failure at/before the separator (`$user.name` dying on
    // `$type`) is a REFERENCE read and stays silent.
    let form = registry
        .forms_for_prefix(prefix)
        .iter()
        .find(|f| f.macro_name == failure.form_name);
    let Some(form) = form else { return false };
    let sep = form
        .form
        .inline_elements
        .iter()
        .rposition(|e| matches!(e, FormInlineElement::Literal(l) if l == ":"));
    let Some(sep) = sep else { return false };
    failure.element_index > sep
}

// =============================================================================
// MatchDiagnostic
// =============================================================================

/// A diagnostic from the matching process.
#[derive(Debug, Clone)]
pub struct MatchDiagnostic {
    /// Human-readable message.
    pub message: String,
    /// Byte offset in source.
    pub offset: usize,
    /// The best failure from attempted forms (if any).
    pub best_failure: Option<MatchFailure>,
}

/// Accumulate a sibling's union alternatives into the retained failure.
///
/// Both failures must be a [`FailureKind::CaptureTypeMismatch`] on the SAME
/// capture slot (same `var_name`, same `element_index`) carrying union
/// alternatives — which is exactly the kind-is-the-macro shape: N sibling
/// `%macro`s whose `%form`s differ only in a one-alternative union at the same
/// position (`@form $kind:("style") …` / `@form $kind:("motion") …`). Anything
/// else is left untouched, so this cannot widen an unrelated diagnostic.
///
/// Order-independent: alternatives are appended in registration order and
/// de-duplicated, so the reported vocabulary does not depend on which sibling
/// happened to rank first.
fn merge_union_alternatives(best: &mut Option<MatchFailure>, incoming: &MatchFailure) {
    let (
        FailureKind::CaptureTypeMismatch {
            var_name: new_var,
            expected_alternatives: new_alts,
            ..
        },
        Some(MatchFailure {
            element_index: kept_index,
            kind:
                FailureKind::CaptureTypeMismatch {
                    var_name: kept_var,
                    expected_alternatives: kept_alts,
                    ..
                },
            ..
        }),
    ) = (&incoming.kind, best.as_mut())
    else {
        return;
    };

    if new_var != kept_var || incoming.element_index != *kept_index {
        return;
    }
    if new_alts.is_empty() || kept_alts.is_empty() {
        return;
    }

    for alt in new_alts {
        if !kept_alts.contains(alt) {
            kept_alts.push(alt.clone());
        }
    }
}

// =============================================================================
// MatchSink
// =============================================================================

/// Sink implementation that produces `Vec<FormMatch>` from parser events.
///
/// Manages a context stack of `NodeContext`. When a matchable node finishes,
/// the sink tries all compiled forms for the node's prefix (most specific first).
/// On match, the FormMatch is stored. On failure, the best failure is recorded
/// as a diagnostic.
pub struct MatchSink<'s> {
    /// Source text for token text extraction.
    source: &'s str,
    /// Pre-compiled forms indexed by prefix.
    compiled_registry: CompiledRegistry,
    /// Extractors for capture types.
    extractors: ExtractorRegistry,
    /// Successful matches.
    matches: Vec<FormMatch>,
    /// Match diagnostics (failures).
    errors: Vec<MatchDiagnostic>,
    /// Stack of in-progress nodes.
    context_stack: Vec<NodeContext>,
    /// Current byte offset (updated by token events).
    offset: usize,
}

impl<'s> MatchSink<'s> {
    /// Create a new MatchSink.
    pub fn new(
        source: &'s str,
        compiled_registry: CompiledRegistry,
        extractors: ExtractorRegistry,
    ) -> Self {
        Self {
            source,
            compiled_registry,
            extractors,
            matches: Vec::new(),
            errors: Vec::new(),
            context_stack: Vec::new(),
            offset: 0,
        }
    }

    /// Consume the sink and return accumulated matches and diagnostics.
    pub fn finish(self) -> (Vec<FormMatch>, Vec<MatchDiagnostic>) {
        (self.matches, self.errors)
    }

    /// Get the matches collected so far (borrowed).
    pub fn matches(&self) -> &[FormMatch] {
        &self.matches
    }

    /// Get the errors collected so far (borrowed).
    pub fn errors(&self) -> &[MatchDiagnostic] {
        &self.errors
    }
}

impl Sink for MatchSink<'_> {
    fn start_node(&mut self, kind: SyntaxKind) {
        let mut ctx = NodeContext::new(kind);
        ctx.span_start = self.offset;
        self.context_stack.push(ctx);
    }

    fn token(&mut self, kind: SyntaxKind, text: &str) {
        let start = self.offset;
        let end = start + text.len();
        self.offset = end;

        if let Some(ctx) = self.context_stack.last_mut() {
            ctx.push_token(kind, (start, end));
        }
    }

    fn finish_node(&mut self) {
        let mut ctx = match self.context_stack.pop() {
            Some(c) => c,
            None => return,
        };
        ctx.span_end = self.offset;

        // An ELEMENT_REF / VARIABLE_REF that is the INLINE ARGUMENT of a directive
        // (e.g. `&self.isActive` in `@when &self.isActive { … }`) must NOT be matched
        // as a standalone form — it belongs to the enclosing directive's form as an
        // argument capture. Without this the inner `&self.isActive` greedily matches
        // the `element-ref` form first (name=self, selector=.isActive) and the parent
        // `@when` directive is left with no child to capture (BUG-088). Defer it to the
        // parent as a child so the directive form can consume its tokens.
        let is_directive_inline_ref =
            matches!(ctx.kind, SyntaxKind::ELEMENT_REF | SyntaxKind::VARIABLE_REF)
                && self
                    .context_stack
                    .iter()
                    .rev()
                    .find(|p| p.kind != SyntaxKind::ARG)
                    .map(|p| p.kind == SyntaxKind::DIRECTIVE)
                    .unwrap_or(false);
        if is_directive_inline_ref {
            if let Some(parent) = self.context_stack.last_mut() {
                parent.push_child(ctx);
            }
            return;
        }

        if ctx.is_matchable() {
            // Try to match against compiled forms
            let prefix = ctx.extract_prefix(self.source);



            if let Some(prefix) = prefix {
                let forms = self.compiled_registry.forms_for_prefix(prefix);

                // Determine scope context: inside a SCOPE_BLOCK means selector scope.
                // PLAN-039: a @template body is ALSO a scope — constructs inside it match at
                // selector scope (so a nested `&x(){}` resolves to a template INVOCATION, not
                // the file-scope template-inline DEFINITION; and nested @editable/@on match).
                let in_scope_block = self.context_stack.iter().any(|c| {
                    c.kind == SyntaxKind::SCOPE_BLOCK
                        || c.is_template_directive(self.source, &self.compiled_registry)
                });

                let mut best_failure: Option<MatchFailure> = None;

                // Sibling-overload body-shape resolution: if this node carries a
                // `{ … }` BODY block, a form that does NOT consume a body (an
                // inline/paren-args overload like the 3D `@light(type, …)`) matches
                // it only by IGNORING the body — and, ranking higher on specificity,
                // would shadow a sibling overload that genuinely takes the body (the
                // responsive `light-mode` `@light { $styles }`), silently dropping the
                // body's captures (`$styles`, `@data fetch … { refresh }`). When a body
                // is present we therefore PREFER the first body-consuming match; a
                // non-body-consuming match is only a FALLBACK (used when no sibling
                // consumes the body — e.g. the bodyless `element-ref` whose trailing
                // `{ … }` is a separate scope, with no body-bearing overload).
                let node_has_body = ctx.child_by_kind(SyntaxKind::BODY).is_some();
                let mut fallback: Option<(FormMatch, NodeContext)> = None;
                // BUG-264: a LENIENT productive match (one that dropped a clause it
                // could not bind) is deferred below a clean error sibling, so a
                // near-miss spelling is refused loudly rather than silently accepted.
                let mut lenient_fallback: Option<(FormMatch, NodeContext)> = None;

                for compiled in forms {
                    // Filter by %scope: skip forms incompatible with the current context
                    if !compiled.scopes.is_empty() {
                        let allowed = if in_scope_block {
                            compiled.scopes.contains(&MacroScope::Selector)
                        } else {
                            compiled.scopes.contains(&MacroScope::File)
                        };
                        if !allowed {
                            continue;
                        }
                    }
                    match compiled.try_match(&ctx, self.source, &self.extractors) {
                        Ok((fm, is_lenient)) => {

                            // Defer a non-body-consuming match when a body is present:
                            // stash it as the fallback and keep scanning for a sibling
                            // that DOES consume the body. (First fallback wins — forms
                            // are specificity-ordered.)
                            if node_has_body && !compiled.consumes_body() {
                                if fallback.is_none() {
                                    fallback = Some((fm, ctx.clone()));
                                }
                                continue;
                            }
                            // BUG-264: a LENIENT productive match (it dropped a clause
                            // it could not bind — ARG_LIST/body trailing leniency) is NOT
                            // clean. Defer it below a matching error sibling: keep
                            // scanning; if a clean sibling wins, the near-miss is
                            // refused loudly. Only with no clean rival do we accept it
                            // (preserving today's behaviour for forms with no error
                            // sibling).
                            // BUG-264: a LENIENT productive match (it dropped a clause
                            // it could not bind — ARG_LIST/body trailing leniency) is NOT
                            // clean. Defer it below a matching error sibling: keep
                            // scanning; if a clean sibling wins, the near-miss is
                            // refused loudly. Only with no clean rival do we accept it
                            // (preserving today's behaviour for forms with no error
                            // sibling).
                            if is_lenient {
                                if lenient_fallback.is_none() {
                                    lenient_fallback = Some((fm, ctx.clone()));
                                }
                                continue;
                            }
                            // Check if parent is a BODY node whose grandparent
                            // is a matchable directive. If so, store the match
                            // on the BODY's matched_children so the enclosing
                            // directive can collect them via $children* body
                            // capture. In scope blocks (SCOPE_BLOCK > BODY),
                            // the grandparent is not matchable, so matches go
                            // to the flat list as before.
                            let stack_len = self.context_stack.len();
                            // PLAN-039: a @template body is a SCOPE — its nested matches
                            // surface to the FLAT list (then assigned to the template-body
                            // scope by span containment, like a .sel{}), NOT absorbed into the
                            // $body:component_body capture. A @template grandparent therefore
                            // does NOT capture its children; other body-capturing matchable
                            // directives keep parent-routing.
                            let grandparent_is_template = stack_len >= 2
                                && self.context_stack[stack_len - 2]
                                    .is_template_directive(self.source, &self.compiled_registry);
                            let route_to_parent = stack_len >= 2
                                && self.context_stack[stack_len - 1].kind == SyntaxKind::BODY
                                && self.context_stack[stack_len - 2].is_matchable()
                                && !grandparent_is_template
                                && !(stack_len >= 2
                                    && self.context_stack[stack_len - 2].kind
                                        == SyntaxKind::ELEMENT_REF);

                            if route_to_parent {
                                if let Some(parent) = self.context_stack.last_mut() {
                                    parent.matched_children.push(fm);
                                    // Also preserve node as a child so that
                                    // token-based body captures (e.g.,
                                    // `$invocations:template_invocation+`)
                                    // can still extract from the raw tokens.
                                    parent.push_child_tokens_only(ctx);
                                }
                            } else {
                                self.matches.push(fm);
                            }
                            return; // Matched — done with this node
                        }
                        Err(failure) => {
                            // Keep the failure from the most specific form.
                            //
                            // BUG-229: "most specific" must not mean "first tried".
                            // Forms are scanned in specificity order across ALL forms
                            // sharing this prefix bucket, so the first failure is
                            // often a DirectivePrefixMismatch from an unrelated form
                            // (`@bind` losing to `@spike`) — which then SHADOWED the
                            // real failure of the form the author actually wrote.
                            // A form whose prefix matched got far enough to judge the
                            // author's intent, so its failure always outranks one from
                            // a form that was never applicable.
                            let is_prefix_mismatch = |f: &MatchFailure| {
                                matches!(f.kind, FailureKind::DirectivePrefixMismatch { .. })
                            };
                            // Sibling overloads that discriminate on a
                            // ONE-ALTERNATIVE union capture (`@form style …` /
                            // `@form motion …` — kind-is-the-macro, see
                            // `stdlib/macros/form.st`) each know exactly ONE word of
                            // a shared vocabulary. Keeping only the winner's failure
                            // would tell the author "expected `easing`" for a
                            // misspelt `mootion` — technically the retained
                            // failure, useless as guidance, and dependent on
                            // registration order. Union alternatives are DATA on the
                            // failure, so ACCUMULATE them across siblings that failed
                            // at the same slot: the reported error then names the
                            // whole vocabulary the author may choose from.
                            merge_union_alternatives(&mut best_failure, &failure);

                            // A failure is REPORTABLE when the form committed to a
                            // grammar and then rejected content inside it — as opposed
                            // to a LiteralMismatch, which only says "different overload
                            // / CSS query" (it is how sibling overloads discriminate).
                            let reportable = |f: &MatchFailure| {
                                matches!(
                                    f.kind,
                                    FailureKind::CaptureFailedNoTokens { .. }
                                        | FailureKind::CaptureTypeMismatch { .. }
                                        | FailureKind::MissingBody
                                        | FailureKind::ExtraInlineTokens { .. }
                                )
                            };
                            let replaces = match best_failure.as_ref() {
                                None => true,
                                Some(current) => {
                                    // A form that matched the prefix always outranks one
                                    // that did not.
                                    if is_prefix_mismatch(current) != is_prefix_mismatch(&failure) {
                                        is_prefix_mismatch(current)
                                    } else if reportable(&failure) != reportable(current) {
                                        // W3 consume-or-refuse: a REPORTABLE failure (the
                                        // form committed to a grammar and rejected its
                                        // content) must outrank a non-reportable
                                        // LiteralMismatch from a SIBLING overload,
                                        // regardless of how far the sibling got. A
                                        // LiteralMismatch only says "different overload /
                                        // CSS query" — letting it shadow a real
                                        // commitment silently dropped the directive.
                                        // (BUG-313: `@wait_for_signal … to_equal 1
                                        // timeout 1500 "message"` — the `to_be_truthy`
                                        // sibling failed at element 3 with a
                                        // LiteralMismatch that hid the `to_equal` form's
                                        // reportable ExtraInlineTokens, so the malformed
                                        // spellings were accepted and dropped.)
                                        reportable(&failure)
                                    } else {
                                        // Both got equally far past the prefix (and are
                                        // equally reportable): prefer the one that
                                        // consumed MORE of its form before failing.
                                        // Sibling overloads share a surface (`type` and
                                        // `type-sum` both declare `@type`), and a
                                        // high-specificity sibling that dies on its
                                        // discriminator at element 0 says far less than the
                                        // sibling that reached the body grammar. Keeping
                                        // the first would let the informative failure be
                                        // permanently shadowed.
                                        failure.element_index > current.element_index
                                    }
                                }
                            };
                            if replaces {
                                best_failure = Some(failure);
                            }
                        }
                    }
                }

                // No body-consuming form matched. If we deferred a non-body-consuming
                // match as a fallback (e.g. the bodyless `element-ref` whose trailing
                // `{ … }` is a separate scope, or any directive with no body-bearing
                // sibling overload), accept it now — the body simply isn't this form's
                // to consume and is handled downstream as a scope block.
                if let Some((fm, fctx)) = fallback.take() {
                    let stack_len = self.context_stack.len();
                    let grandparent_is_template = stack_len >= 2
                        && self.context_stack[stack_len - 2]
                            .is_template_directive(self.source, &self.compiled_registry);
                    let route_to_parent = stack_len >= 2
                        && self.context_stack[stack_len - 1].kind == SyntaxKind::BODY
                        && self.context_stack[stack_len - 2].is_matchable()
                        && !grandparent_is_template
                        && !(stack_len >= 2
                            && self.context_stack[stack_len - 2].kind == SyntaxKind::ELEMENT_REF);
                    if route_to_parent {
                        if let Some(parent) = self.context_stack.last_mut() {
                            parent.matched_children.push(fm);
                            parent.push_child_tokens_only(fctx);
                        }
                    } else {
                        self.matches.push(fm);
                    }
                    return;
                }

                // No form matched. Retain a diagnostic even when this directive is
                // nested under a root context, where generic form failures are
                // otherwise intentionally passed upward as child tokens.
                //
                // BUG-229 generalizes what BUG-234 did for `@data subscribe` alone.
                // The principle behind that special case was never "subscribe is
                // special" — it was: a directive that COMMITTED to a grammar and then
                // failed inside it is a real error, not a directive we merely tried.
                // `FailureKind` already draws that line:
                //
                //   DirectivePrefixMismatch — wrong directive entirely; every form is
                //     tried against every directive, so this is the normal negative
                //     result of a search. Silent, correctly.
                //   Capture{FailedNoTokens,TypeMismatch} — the prefix DID match, so
                //     this form was the author's intent, and its declared capture type
                //     rejected the content. Silence here is what let a malformed body
                //     compile green.
                //
                // The second half is IDENTITY — "was this form the one the author
                // meant?" — and it is answered by the failure KIND, not by a name.
                //
                // Not by a name, because two earlier attempts were wrong:
                //   * `form_name == directive` — `form_name` is the MACRO name, and
                //     dispatch keys on the `%form`'s first token, not the macro name
                //     (`stdlib/testing/automation.st:59-61`). `%macro
                //     wait-for-appears` declaring `@wait-for` would never match, so
                //     every such directive stayed silent.
                //   * slicing the name out of the source — string-sniffing that
                //     `src/syntax/ARCHITECTURE.md` forbids, and it missed a qualified
                //     `@scene/camera` whose leaf the prefix matcher resolves.
                //
                // "Prefix matched" alone is TOO WIDE, and CSS is why. `@media
                // (prefers-reduced-motion: reduce)` is valid CSS with no `%form` that
                // accepts it — `@media($query:string)` wants a QUOTED string — so it
                // fails at the query and falls through to the CSS layer that owns it.
                // Reporting that turns plain CSS into a compile error.
                //
                // The reportable kinds are the ones that can only arise AFTER a form
                // has committed to shape, where no CSS fallthrough is plausible:
                //   * Capture{FailedNoTokens,TypeMismatch} — a declared grammar
                //     rejected the content (the BUG-229 case).
                //   * MissingBody — `try_match` reaches this only once the prefix and
                //     all preceding inline grammar matched, so the form is
                //     unambiguously selected and its required body is absent.
                //   * ExtraInlineTokens — the declared inline grammar matched and
                //     content REMAINS (`@form motion --x junk { … }`): the same
                //     commitment class, one token later. Suppressing it let a
                //     directive compile with its tail silently ignored.
                // LiteralMismatch is deliberately EXCLUDED: literals also discriminate
                // sibling overloads (and CSS queries), so it cannot distinguish "wrong
                // shape" from "different overload".
                //
                // NB narrow by design. A directive matching NO form (a typo'd
                // `@dirctive`) yields only prefix mismatches and stays W0714's domain.
                //
                // Content inside a `%macro` definition is NEVER reported here: it
                // is grammar/definition DATA, not an invocation — a `%form`
                // pattern is the grammar itself, and a `%includes` clause is
                // parametric BY DESIGN (`%includes { @fade-in(duration: $duration) }`
                // in stdlib/macros/fade-in.st: the `$duration` is the including
                // macro's own param, so `Duration` rejecting DOLLAR is correct
                // grammar behavior and a meaningless error). Matching inside a
                // META_DEF still runs (composition bookkeeping consumes it); only
                // the author-facing diagnostic is suppressed.
                let inside_meta_def = self
                    .context_stack
                    .iter()
                    .any(|p| p.kind == SyntaxKind::META_DEF);
                let reportable_kind = matches!(
                    best_failure.as_ref().map(|f| &f.kind),
                    Some(FailureKind::CaptureFailedNoTokens { .. })
                        | Some(FailureKind::CaptureTypeMismatch { .. })
                        | Some(FailureKind::MissingBody)
                        | Some(FailureKind::ExtraInlineTokens { .. })
                );
                // I3 / gh-35: a `$`-declaration whose bounded statement is not
                // fully consumed must fail loud — but a VARIABLE_REF is also the
                // spelling of an expression READ (`text <- $user.name`), which
                // must stay silent. The ref only reports when the form committed
                // past its value separator (a declaration shape), via
                // `ref_committed_declaration`.
                let committed_then_failed = !inside_meta_def && match ctx.kind {
                    SyntaxKind::DIRECTIVE => reportable_kind,
                    SyntaxKind::VARIABLE_REF => {
                        reportable_kind
                            && best_failure.as_ref().is_some_and(|f| {
                                ref_committed_declaration(&self.compiled_registry, f, '$')
                            })
                    }
                    _ => false,
                };
                // BUG-264 ranking: an error sibling's committed-then-failed report
                // BEATS a lenient productive match. Track whether such a refusal was
                // emitted so the lenient fallback below is accepted only otherwise.


                let mut refusal_emitted = false;
                if committed_then_failed {
                    // `MatchFailure::message()` already renders every kind; reuse it
                    // rather than re-deriving a parallel set of strings here.
                    let failure = best_failure.as_ref().expect("guarded above");
                    self.errors.push(MatchDiagnostic {
                        message: failure.message(),
                        offset: ctx.span_start,
                        best_failure: best_failure.clone(),
                    });
                    refusal_emitted = true;
                } else if self
                    .source
                    .get(ctx.span_start..ctx.span_end)
                    .unwrap_or("")
                    .split_whitespace()
                    .take(2)
                    .eq(["@data", "subscribe"])
                {
                    // BUG-234. `subscribe` is a KEYWORD COMMITMENT: once it is
                    // written, `@data` can only be the live-data grammar, so a missing
                    // `from` is a malformed directive rather than a different overload.
                    // The generic rule above cannot see that, because the failure is a
                    // `LiteralMismatch` — the one kind that must stay excluded there,
                    // since literals are also how sibling overloads and CSS queries
                    // discriminate.
                    //
                    // This is the ONE construct that earns a keyword-commitment
                    // exception. If a second one appears, the commitment belongs in
                    // `%form` data (a `%commits` marker the matcher reads) rather than
                    // a second hardcoded shape here.
                    self.errors.push(MatchDiagnostic {
                        message: "malformed @data subscribe".to_string(),
                        offset: ctx.span_start,
                        best_failure: best_failure.clone(),
                    });
                    refusal_emitted = true;
                }

                // BUG-264 ranking: productive-clean > error-sibling > productive-lenient
                // > no-match. The error sibling refused above; a lenient productive match
                // is accepted ONLY when no such refusal fired — so a near-miss with an
                // error sibling stays refused, while every other lenient form keeps its
                // historical acceptance.
                if !refusal_emitted {
                    if let Some((fm, fctx)) = lenient_fallback.take() {
                        let stack_len = self.context_stack.len();
                        let grandparent_is_template = stack_len >= 2
                            && self.context_stack[stack_len - 2]
                                .is_template_directive(self.source, &self.compiled_registry);
                        let route_to_parent = stack_len >= 2
                            && self.context_stack[stack_len - 1].kind == SyntaxKind::BODY
                            && self.context_stack[stack_len - 2].is_matchable()
                            && !grandparent_is_template
                            && !(stack_len >= 2
                                && self.context_stack[stack_len - 2].kind
                                    == SyntaxKind::ELEMENT_REF);
                        if route_to_parent {
                            if let Some(parent) = self.context_stack.last_mut() {
                                parent.matched_children.push(fm);
                                parent.push_child_tokens_only(fctx);
                            }
                        } else {
                            self.matches.push(fm);
                        }
                        return;
                    }
                }

                // If this node has a parent context, push it as a child so
                // the parent directive can consume its tokens during form
                // matching. This handles cases like `&card` (ELEMENT_REF)
                // inside `@template &card($content) {}` — the element ref
                // doesn't match any standalone `&` form, but its tokens
                // are needed by the enclosing DIRECTIVE's form matching.
                if !self.context_stack.is_empty() {
                    if let Some(parent) = self.context_stack.last_mut() {
                        parent.push_child(ctx);
                    }
                } else if let Some(failure) = best_failure {
                    // Top-level unmatched node — record diagnostic
                    self.errors.push(MatchDiagnostic {
                        message: failure.message(),
                        offset: ctx.span_start,
                        best_failure: Some(failure),
                    });
                }
            }
        } else {
            // Non-matchable node: push as child into parent context
            if let Some(parent) = self.context_stack.last_mut() {
                parent.push_child(ctx);
            }
        }
    }

    fn error(&mut self, msg: String, offset: usize) {
        self.errors.push(MatchDiagnostic {
            message: msg,
            offset,
            best_failure: None,
        });
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use crate::parser::meta_ast::{
        CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, MacroDefAst,
    };
    use crate::syntax::events::test_support::{driver_member, register_driver_expr};
    use crate::syntax::registry::SyntaxRegistry;

    fn make_test_macro_def(name: &str, form: FormClause) -> MacroDefAst {
        MacroDefAst {
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
    

    /// Helper: build a SyntaxRegistry with a single "@on $event:ident" form.
    fn make_on_event_registry() -> SyntaxRegistry {
        let mut reg = SyntaxRegistry::new();
        let macro_def = make_test_macro_def(
            "on-event",
            FormClause {
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
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: vec![],
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            },
        );
        reg.register(&macro_def);
        reg
    }

    /// Helper: build a SyntaxRegistry with a single "@on $driver:driver_expr" form
    /// (the sigil driver head; `make_on_event_registry` stays the legacy bare-ident
    /// shape for the tests that still exercise it).
    fn make_driver_registry() -> SyntaxRegistry {
        let mut reg = SyntaxRegistry::new();
        let macro_def = make_test_macro_def(
            "on-event",
            FormClause {
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
            },
        );
        reg.register(&macro_def);
        reg
    }

    #[test]
    fn match_sink_processes_directive() {
        let source = "@on &.hover";
        let syntax_reg = make_driver_registry();
        let mut extractors = ExtractorRegistry::new();
        register_driver_expr(&mut extractors);
        let compiled = CompiledRegistry::from_registry(&syntax_reg, &extractors);

        let mut sink = MatchSink::new(source, compiled, extractors);

        // Simulate events: DIRECTIVE containing an ELEMENT_REF driver child.
        sink.start_node(SyntaxKind::DIRECTIVE);
        sink.token(SyntaxKind::AT_SIGN, "@");
        sink.token(SyntaxKind::IDENT, "on");
        sink.token(SyntaxKind::WHITESPACE, " ");
        sink.start_node(SyntaxKind::ELEMENT_REF);
        sink.token(SyntaxKind::AMPERSAND, "&");
        sink.token(SyntaxKind::DOT, ".");
        sink.token(SyntaxKind::IDENT, "hover");
        sink.finish_node(); // ELEMENT_REF → becomes a child of DIRECTIVE
        sink.finish_node(); // DIRECTIVE → tries matching

        let (matches, errors) = sink.finish();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].macro_name, "on");
        assert_eq!(
            driver_member(&matches[0], "driver").as_deref(),
            Some("hover"),
            "driver capture should carry member=hover"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn match_sink_qualified_invocation_matches_unqualified_form() {
        // FEAT-118: `@scene/on hover` — the `scene/` namespace qualifier is
        // skipped so the leaf `on` still matches the unqualified `@on` form.
        let source = "@scene/on hover";
        let syntax_reg = make_on_event_registry();
        let extractors = ExtractorRegistry::new();
        let compiled = CompiledRegistry::from_registry(&syntax_reg, &extractors);

        let mut sink = MatchSink::new(source, compiled, ExtractorRegistry::new());

        sink.start_node(SyntaxKind::DIRECTIVE);
        sink.token(SyntaxKind::AT_SIGN, "@");
        sink.token(SyntaxKind::IDENT, "scene");
        sink.token(SyntaxKind::SLASH, "/");
        sink.token(SyntaxKind::IDENT, "on");
        sink.token(SyntaxKind::WHITESPACE, " ");
        sink.token(SyntaxKind::IDENT, "hover");
        sink.finish_node();

        let (matches, errors) = sink.finish();
        assert_eq!(matches.len(), 1, "qualified @scene/on matches the @on form");
        assert_eq!(matches[0].macro_name, "on");
        assert_eq!(
            matches[0].captures.get("event"),
            Some(&crate::syntax::form_match::CapturedValue::Ident(
                "hover".to_string()
            )),
            "the leaf form still captures its args after the qualifier"
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn match_sink_nested_non_matchable_becomes_child() {
        let source = "@on &.hover(300ms)";
        let syntax_reg = make_on_event_registry();
        let extractors = ExtractorRegistry::new();
        let compiled = CompiledRegistry::from_registry(&syntax_reg, &extractors);

        let mut sink = MatchSink::new(source, compiled, ExtractorRegistry::new());

        // Simulate: DIRECTIVE containing an ARG_LIST child
        sink.start_node(SyntaxKind::DIRECTIVE);
        sink.token(SyntaxKind::AT_SIGN, "@");
        sink.token(SyntaxKind::IDENT, "on");
        sink.token(SyntaxKind::WHITESPACE, " ");
        sink.token(SyntaxKind::IDENT, "hover");

        // Nested ARG_LIST node
        sink.start_node(SyntaxKind::ARG_LIST);
        sink.token(SyntaxKind::L_PAREN, "(");
        sink.token(SyntaxKind::NUMBER_WITH_UNIT, "300ms");
        sink.token(SyntaxKind::R_PAREN, ")");
        sink.finish_node(); // finishes ARG_LIST → becomes child of DIRECTIVE

        sink.finish_node(); // finishes DIRECTIVE → tries matching

        let (matches, _errors) = sink.finish();
        // Match should succeed (ARG_LIST is optional in this form)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].macro_name, "on");
    }

    #[test]
    fn match_sink_error_collection() {
        let source = "@on &.hover";
        let compiled = CompiledRegistry::from_registry(
            &SyntaxRegistry::new(), // empty registry — nothing to match
            &ExtractorRegistry::new(),
        );
        let mut sink = MatchSink::new(source, compiled, ExtractorRegistry::new());

        sink.start_node(SyntaxKind::DIRECTIVE);
        sink.token(SyntaxKind::AT_SIGN, "@");
        sink.token(SyntaxKind::IDENT, "on");
        sink.finish_node();

        let (matches, errors) = sink.finish();
        assert!(matches.is_empty());
        // No forms to try, so no best_failure, just no match
        assert!(errors.is_empty());
    }

    #[test]
    fn match_sink_records_parser_errors() {
        let source = "broken";
        let compiled =
            CompiledRegistry::from_registry(&SyntaxRegistry::new(), &ExtractorRegistry::new());
        let mut sink = MatchSink::new(source, compiled, ExtractorRegistry::new());

        sink.error("Unexpected token".to_string(), 0);

        let (_, errors) = sink.finish();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Unexpected token");
    }
}
