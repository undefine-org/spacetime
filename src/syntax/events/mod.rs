//! Event-based parser for Spacetime.
//!
//! Adapted from rust-analyzer's architecture: the parser emits a flat
//! `Vec<Event>` that multiple sinks consume to produce different outputs
//! (CST, FormMatch, diagnostics) from a single parse pass.
//!
//! # Architecture
//!
//! ```text
//! Lexer -> Input -> Parser -> Vec<Event> -> process() -> Sink
//!                                                          |
//!                                            +-------------+-------+
//!                                            |             |       |
//!                                       TreeSink    MatchSink  DiagSink
//!                                            |             |       |
//!                                       SyntaxNode  FormMatch  Diagnostic
//! ```

pub mod event;
pub mod input;
pub mod parser;
pub mod strategy;

#[cfg(test)]
pub mod component_body_corpus;
#[cfg(test)]
pub mod test_support;
pub mod diag_sink;
pub mod extractors;
pub mod form_compiler;
pub mod grammar;
pub mod match_sink;
pub mod process;
pub mod sink;
pub mod tree_sink;

// Re-exports
pub use diag_sink::DiagnosticSink;
pub use event::{CompletedMarker, Event, Marker};
pub use form_compiler::{
    CompiledForm, CompiledRegistry, MatchFailure, NodeContext, convert_property_value_public,
};
pub use input::Input;
pub use match_sink::MatchSink;
pub use parser::Parser;
pub use process::process;
pub use sink::Sink;
pub use strategy::ErrorStrategy;
pub use tree_sink::TreeSink;

// =============================================================================
// High-level API: parse source → Vec<FormMatch> via MatchSink
// =============================================================================

use crate::syntax::cst::lexer::Lexer;
use crate::syntax::form_match::FormMatch;
use crate::syntax::registry::SyntaxRegistry;

use self::extractors::ExtractorRegistry;
pub use self::match_sink::MatchDiagnostic;

/// Parse source text and produce `Vec<FormMatch>` using the event-based parser.
///
/// This is the replacement for `convert_cst_directive_to_form_match()` and friends.
/// It lexes the source, runs the event-based parser, and processes events through
/// MatchSink to produce form matches from a single pass.
///
/// Returns `(matches, diagnostics)`.
pub fn parse_matches(
    source: &str,
    registry: &SyntaxRegistry,
) -> (Vec<FormMatch>, Vec<MatchDiagnostic>) {
    let mut extractors = ExtractorRegistry::new();
    // Compile stdlib %capture_type productions into custom extractors so grammar defined
    // in stdlib actually drives matching (PLAN-023 W2). A defs map lets one production
    // reference another by name (types-on-types, e.g. param_list -> optmark); reifiers
    // restore typed CapturedValue shapes for the migrated category-B types.
    let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
        registry
            .capture_types()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();
    for ct in registry.capture_types() {
        let inner = self::extractors::custom::compile_pattern_with_defs(
            &ct.pattern,
            &extractors,
            &defs,
            &mut Vec::new(),
        );
        let extractor = self::extractors::custom::wrap_reifier(&ct.name, inner);
        extractors.register_custom(ct.name.clone(), extractor);
    }
    let compiled = CompiledRegistry::from_registry(registry, &extractors);

    // 1. Lex source into tokens
    let lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    // 2. Build Input from tokens
    let input = Input::from_tokens(&tokens);

    // 3. Run event-based parser
    let mut parser = Parser::new(&input, ErrorStrategy::Verbose);
    grammar::root(&mut parser, source);
    let mut events = parser.finish();

    // 4. Process events through MatchSink
    let mut match_sink = MatchSink::new(source, compiled, extractors);
    process(&mut events, source, &input, &mut match_sink);

    match_sink.finish()
}

#[cfg(test)]
mod baseline_tests;

/// Whether a stdlib `%capture_type` production accepts this text WHOLE.
///
/// "Whole" is the load-bearing word: a grammar that matches a prefix has not
/// accepted the value. `#e8eef7z` matches `hex_color` for its first seven bytes
/// and must still be refused, which is why the token count is compared rather
/// than merely checked for success.
///
/// This is the one entry point for "does this text satisfy this grammar?" —
/// callers outside the matcher (the typed-seed checker, the editor) ask HERE
/// rather than reimplementing a parse.
pub fn capture_type_accepts(capture_type: &str, text: &str) -> bool {
    let registry = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;

    // A SCALAR also accepts a reference to a value of itself: `--brand-ink`,
    // `var(--ink)`, `var(--ink, #fff)`. Neither is a literal, and neither is an
    // error — they name a value the cascade resolves later, so refusing them at
    // build time would be asserting something we cannot know. The declaration
    // validator already reaches this conclusion for `var()`
    // (`src/validation/css.rs`); this is the same rule at the second place it
    // was needed.
    //
    // ADMITTED ONCE, not per production. Adding `| ( $token:token_ref )` to each
    // leaf grammar would be six places to forget and — worse — a NEW
    // `%scalar_type` row would not inherit it, which is precisely the
    // hand-synced-list drift PLAN-122 spent seven commits deleting. Keying off
    // the scalar-capture set the registry already maintains means a scalar added
    // tomorrow gets this for free, and `a_new_scalar_inherits_reference_support`
    // is the gate that keeps it that way.
    if registry.scalar_captures().contains(capture_type) && is_token_reference(text) {
        return true;
    }
    let extractors = ExtractorRegistry::new();
    let defs: std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst> =
        registry
            .capture_types()
            .map(|c| (c.name.clone(), c.clone()))
            .collect();
    // An EMPTY value satisfies nothing, and this check precedes the
    // unknown-type escape below on purpose: "not ours to refuse" is about a
    // grammar we cannot model, not about a value that is not there. Splitting
    // the matcher out (BUG-279) briefly moved this test inside it, which let an
    // empty value pass whenever the named production was missing — exactly the
    // condition stdlib_embedded.rs already documents as a silent-accept hazard.
    if text.trim().is_empty() {
        return false;
    }

    // A capture type is EITHER a stdlib grammar (`%capture_type color`) or a
    // BUILTIN the parser maps to a `CaptureType` variant (`number`, `bool`,
    // `ident`). Both are real types with real extractors, so both must be asked.
    //
    // `match_capture_type` below answers only the stdlib half — it starts with
    // `defs.get(capture_type)?`, so a builtin name returns `None` and would fall
    // straight into the "no such production" arm. That arm says `true`, which is
    // exactly the PLAN-136 W1 defect: `number` accepted `#e8eef7`, `bool`
    // accepted `abc`, and the 344 of 409 form params (84%) declaring a builtin
    // type were checked by nothing at all. `NumberExtractor` was correct the
    // whole time; it was never reached.
    //
    // So the builtin half is asked FIRST, and only a name that is neither a
    // stdlib production nor a builtin reaches "not ours to refuse".
    if !defs.contains_key(capture_type) {
        let builtin = crate::parser::parse_capture_type_public(capture_type);
        // `Custom` means the name matched no builtin AND no stdlib production:
        // genuinely unknown, and not ours to refuse.
        if matches!(builtin, crate::parser::meta_ast::CaptureType::Custom(_)) {
            return true;
        }
        let extractors = ExtractorRegistry::new();
        let extractor = extractors::custom::builtin_extractor_for(&builtin, &extractors);
        return whole_match(extractor.as_ref(), text);
    }

    // No such production — not ours to refuse.
    match match_capture_type(capture_type, text, &defs) {
        None => !defs.contains_key(capture_type),
        Some(_) => true,
    }
}

/// Does `extractor` consume every SIGNIFICANT token of `text`?
///
/// Shared by the builtin path in `capture_type_accepts` and, through
/// `match_capture_type`, by the stdlib-grammar path — one definition of "the
/// value matched WHOLE", so the two halves of the type system cannot drift into
/// disagreeing about what a complete match is.
///
/// TRIVIA IS KEPT in the token run. `AdjacentExtractor` decides `40px` from
/// `40 px` by asking whether a whitespace token sits between the number and its
/// unit, so filtering trivia out does not merely lose information — it makes an
/// invalid dimension look valid, and the `length` grammar then consumes both
/// tokens and reports a whole match. `300 ms` was accepted as a duration
/// exactly this way. Only the COMPARISON ignores trailing trivia, because
/// `#e8eef7 ` is the same colour as `#e8eef7`.
fn whole_match(extractor: &dyn extractors::CaptureExtractor, text: &str) -> bool {
    use crate::syntax::cst::SyntaxKind;
    use crate::syntax::cst::lexer::Lexer;
    use extractors::TokenData;

    let tokens: Vec<TokenData> = Lexer::new(text)
        .tokenize()
        .into_iter()
        .filter(|t| t.kind != SyntaxKind::EOF)
        .map(|t| TokenData {
            kind: t.kind,
            text_range: (t.offset, t.offset + t.len()),
        })
        .collect();
    if tokens.iter().all(|t| t.kind.is_trivia()) {
        return false;
    }
    let last_significant = tokens
        .iter()
        .rposition(|t| !t.kind.is_trivia())
        .map(|i| i + 1)
        .unwrap_or(0);
    matches!(extractor.extract(&tokens, text), Some((_, n)) if n >= last_significant)
}

/// Which ARM of a `%capture_type` this text matches WHOLE, if any.
///
/// `capture_type_accepts` asks whether a value satisfies a grammar; this asks
/// WHICH alternative it satisfied, and it is the same parse — a production is a
/// choice over named arms (`( $core:color_core ) | ( $wide:css_wide )`), and the
/// arm's capture NAME is the answer a router needs. Splitting these into two
/// matchers would be two answers to one question, which is the shape this file's
/// doc comment exists to prevent.
///
/// `defs` is passed IN rather than read from the global stdlib registry because
/// the caller may hold a different one: the compiler's registry is resolved from
/// the source tree while `STDLIB_REGISTRY` is cwd-relative (BUG-326), and a
/// router that silently consulted the other one would answer about a stdlib the
/// page was not compiled against.
///
/// Returns `None` when the type is unregistered, the text is empty, no arm
/// matches, or a match does not consume every significant token. A caller that
/// routes on the answer therefore falls through to its unrouted behavior for
/// anything this grammar does not CLAIM — which is what keeps an unknown value
/// shape behaving exactly as it did before the router existed.
pub fn capture_type_matched_arm(
    capture_type: &str,
    text: &str,
    defs: &std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst>,
) -> Option<String> {
    match match_capture_type(capture_type, text, defs)? {
        crate::syntax::form_match::CapturedValue::Named(map) if map.len() == 1 => {
            map.into_keys().next()
        }
        _ => None,
    }
}

/// Match `text` against a registered production, returning the captured value.
///
/// The one matcher behind both public questions above.
fn match_capture_type(
    capture_type: &str,
    text: &str,
    defs: &std::collections::HashMap<String, crate::parser::meta_ast::CaptureTypeDefAst>,
) -> Option<crate::syntax::form_match::CapturedValue> {
    use crate::syntax::cst::lexer::Lexer;
    use crate::syntax::cst::SyntaxKind;
    use extractors::{ExtractorRegistry, TokenData};

    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let extractors = ExtractorRegistry::new();
    let def = defs.get(capture_type)?;
    let inner = extractors::custom::compile_pattern_with_defs(
        &def.pattern,
        &extractors,
        defs,
        &mut Vec::new(),
    );
    let extractor = extractors::custom::wrap_reifier(capture_type, inner);

    // TRIVIA IS KEPT. `AdjacentExtractor` decides `40px` from `40 px` by asking
    // whether a whitespace token sits between the number and its unit, so
    // filtering trivia out here does not merely lose information — it makes an
    // invalid dimension look valid, and the `length` grammar then consumes both
    // tokens and reports a whole match. `300 ms` was accepted as a duration
    // exactly this way.
    let tokens: Vec<TokenData> = Lexer::new(text)
        .tokenize()
        .into_iter()
        .filter(|t| t.kind != SyntaxKind::EOF)
        .map(|t| TokenData {
            kind: t.kind,
            text_range: (t.offset, t.offset + t.len()),
        })
        .collect();
    if tokens.iter().all(|t| t.kind.is_trivia()) {
        return None;
    }
    // A whole match consumes every SIGNIFICANT token. Trailing trivia is not
    // content — `#e8eef7 ` is the same colour as `#e8eef7` — so the comparison
    // is against the last non-trivia index, not the raw length.
    let last_significant = tokens
        .iter()
        .rposition(|t| !t.kind.is_trivia())
        .map(|i| i + 1)
        .unwrap_or(0);
    match extractor.extract(&tokens, text) {
        Some((value, n)) if n >= last_significant => Some(value),
        _ => None,
    }
}
/// Is `text` a reference to a design token rather than a literal?
///
/// Two spellings, one meaning — "the value lives elsewhere and resolves later":
///   - `--token`      Spacetime's bare reference
///   - `var(--token)` the CSS spelling, with an optional fallback
///
/// A bare `--token` is NOT legal CSS in a value position; it is a Spacetime
/// affordance that lowers to `var(--token)`. Both are accepted here because
/// both are things an author writes today (`demos/spacetime-docs/index.st:148`).
pub(crate) fn is_token_reference(text: &str) -> bool {
    // `--name`: the sigil must be followed by an actual name, so `--` alone and
    // `-ink` (one dash) are both refused.
    if let Some(name) = text.strip_prefix("--") {
        return !name.is_empty()
            && !name.starts_with('-')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && name.chars().any(|c| c.is_ascii_alphanumeric());
    }

    // `var( ... )`: case-insensitive per CSS, and the parens must balance so an
    // unclosed `var(--ink` stays an error. The interior is deliberately NOT
    // validated — a fallback may nest arbitrarily (`var(--a, var(--b, red))`)
    // and its correctness is the browser's to judge, not ours.
    let folded = text.to_ascii_lowercase();
    let Some(rest) = folded.strip_prefix("var(") else {
        return false;
    };
    let Some(inner) = rest.strip_suffix(')') else {
        return false;
    };
    if !inner.trim_start().starts_with("--") {
        return false;
    }
    // Parens inside a STRING or a COMMENT are content, not structure.
    // `var(--ink, /* ) */ red)` has a valid fallback, and counting the
    // comment's `)` drives the depth negative and refuses it — a false refusal
    // on legal CSS. Same reasoning as `parens_balanced_outside_strings` in
    // validation::css, which already learned this.
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == '\\' {
                chars.next();
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = '\0';
                for d in chars.by_ref() {
                    if prev == '*' && d == '/' {
                        break;
                    }
                    prev = d;
                }
            }
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && quote.is_none()
}



