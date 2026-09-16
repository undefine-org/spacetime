//! Generic directive parsing.
//!
//! Parses directives structurally as `@name args (params) { body }` without
//! any per-family knowledge. The grammar is fully generic — semantic
//! interpretation is done by FormCompiler via %form patterns.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::events::parser::Parser;

/// Parse a directive: `@name args (params) { body }`
///
/// ```text
/// directive = '@' IDENT inline_args? arg_list? body?
/// ```
pub fn directive(p: &mut Parser) {
    directive_with_source(p, "");
}

/// Consume a directive used as the value immediately following a value colon.
///
/// The CST must keep this opaque: capture types run after CST grouping, so an
/// inner `@name` must be one ARG of the outer form rather than a sibling
/// DIRECTIVE matched independently. This is generic (`@match` is no longer
/// named): `: @name … { … }` and `: @name …;` both become a single ARG.
fn directive_value_atom(p: &mut Parser) -> bool {
    if !p.at(SyntaxKind::AT_SIGN)
        || p.prev_non_trivia() != SyntaxKind::COLON
        || !(p.nth(1) == SyntaxKind::IDENT || p.nth(1).is_keyword())
    {
        return false;
    }

    let value = p.start();
    p.bump_any(); // '@'
    p.bump_any(); // directive name
    while p.at_tight_qualifier() {
        p.bump_any(); // '/'
        p.bump_any(); // qualifier segment
    }

    // Mirror directive_with_source, but preserve the entire construct as an
    // ARG rather than emitting a nested DIRECTIVE node.
    let value_colon = inline_args(p);
    if value_colon && p.at(SyntaxKind::L_PAREN) {
        inline_value_expr(p);
    } else if p.at(SyntaxKind::L_PAREN) {
        arg_list(p);
    }
    inline_args(p);

    if p.at(SyntaxKind::L_BRACE) {
        // The following `;`, if any, terminates the OUTER directive value.
        // Keep its ownership unchanged from the retired @match-specific route.
        consume_balanced_brackets(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump_any();
    } else {
        // Keep the incomplete value atom isolated so a following sibling
        // directive is never swallowed; the normal diagnostic still names the
        // missing terminator.
        p.error("expected `{ … }` body or `;` after directive value");
    }

    value.complete_p(p, SyntaxKind::ARG);
    true
}

/// Parse a directive with source text for meta-clause dispatch in nested bodies.
pub fn directive_with_source(p: &mut Parser, source: &str) {
    let m = p.start();

    // '@'
    p.expect(SyntaxKind::AT_SIGN);

    // directive name (one or more idents, e.g., "on", "data")
    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
        // FEAT-118: namespace-qualified name `@scene/camera` — consume tightly
        // adjacent `/segment` runs into the directive head (mirrors the CST
        // parser). A spaced `/` (CSS value shorthand) is NOT consumed.
        while p.at_tight_qualifier() {
            p.bump_any(); // '/'
            p.bump_any(); // segment ident
        }
    }

    // Inline arguments: tokens between the name and '(' or '{'
    let value_colon = inline_args(p);

    // A value-introducing COLON immediately followed by `(` is a paren-led VALUE
    // expression (`@data derive $x : ($a || []).slice(0, 5)`), NOT a directive
    // arg-list. Consume the whole balanced run as a single VALUE arg so the
    // trailing `.member(...)` call survives intact in the directive's inline
    // tokens. (BUG-077 twin / BUG-082 follow-up.)
    //
    // A value-introducing `@name` is likewise ONE value ARG, generically — see
    // directive_value_atom. The COLON guard keeps a sibling directive after a
    // complete value separate. Checked both before and after an inline arg-list
    // because `(A | B)` may occur before the value colon.
    if directive_value_atom(p) {
        // routed: the directive value is captured opaquely as one ARG
    } else if value_colon && p.at(SyntaxKind::L_PAREN) {
        let vm = p.start();
        inline_value_expr(p);
        vm.complete_p(p, SyntaxKind::ARG);
    } else if p.at(SyntaxKind::L_PAREN) {
        // Optional parenthesized params: (...)
        arg_list(p);
    }

    // Consume trailing tokens between ')' and '{' (e.g., return type `: string`)
    inline_args(p);

    // ADDITIONAL parenthesized groups interleaved with inline args — e.g.
    // `@on &.visible(threshold: 0.2) --rise(distance: 8px);`: the driver's
    // params and the form's call args are BOTH parenthesized groups. Without
    // this the second group was left unparsed at DIRECTIVE close and silently
    // detached from the match (BUG-246: the form's args vanished and its
    // DEFAULTS substituted). Tight-only: a newline-led `(` closes the
    // directive as before (it was never a continuation).
    while p.at(SyntaxKind::L_PAREN) && !p.newline_before_current() {
        arg_list(p);
        inline_args(p);
    }

    // Second chance after an inline union / arg-list before the value colon.
    directive_value_atom(p);

    // A `{` that opens an OBJECT LITERAL value (`@data inline $x : { "k": v };`) is
    // the directive's VALUE captured as one ARG, not its body — disambiguated by a
    // quoted key (`{` followed by STRING). Bodies hold IDENT-named props/directives,
    // never a leading string literal (BUG-042 / FEAT-047 bare-object inline value).
    if p.at(SyntaxKind::L_BRACE) && starts_object_literal(p) {
        let am = p.start();
        consume_balanced_brackets(p);
        am.complete_p(p, SyntaxKind::ARG);
    } else if p.at(SyntaxKind::L_BRACE) {
        // Optional body: { ... }
        body_with_source(p, source);
    }

    // Optional semicolon
    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::DIRECTIVE);
}

/// Parse a variable reference at top level: `$name type: value;`
pub fn variable_ref(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::DOLLAR);

    // Name
    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
    }

    // Inline args (type, colon, value, etc.)
    inline_args(p);

    // BUG-342 — a `{` in VALUE position is an object literal, not a body.
    //
    // `$u object: { name: "Ada" };` reported E0946 while `$u: { name: "Ada" };`
    // (no type) and `$u object: [1,2];` (type + array) both compiled: adding a
    // type removed a value shape that already worked, which is exactly the kind
    // of unlearnable exception the language rules forbid.
    //
    // `directive()` above already makes this distinction with the SAME guard —
    // `starts_object_literal`, which uses POSITION as the discriminator (an
    // object value follows the `:` that introduces it; a body follows a
    // directive head). This path simply never asked, so it took ANY `{` as a
    // body, the value went missing, and the form matcher reported a grammar
    // mismatch against a declaration whose value it had never been given.
    //
    // Reusing the one guard rather than adding a second notion of "is this an
    // object" is the point: two notions are how these paths came to disagree.
    if p.at(SyntaxKind::L_BRACE) && starts_object_literal(p) {
        let am = p.start();
        consume_balanced_brackets(p);
        am.complete_p(p, SyntaxKind::ARG);
    } else if p.at(SyntaxKind::L_BRACE) {
        // Optional body
        body_with_source(p, "");
    }

    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::VARIABLE_REF);
}

/// Parse a variable reference inline (within CSS values): `$name`
pub fn variable_ref_inline(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::DOLLAR);

    // Name
    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
    }

    // Property path: $name.prop.subprop
    while p.at(SyntaxKind::DOT) {
        p.bump_any(); // consume DOT
        if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
            p.bump_any();
        } else {
            break; // DOT not followed by IDENT — stop path parsing
        }
    }

    // Type annotation: $name:type
    if p.at(SyntaxKind::COLON) {
        // Only if followed by an ident (to not eat the colon in property context)
        if p.nth(1) == SyntaxKind::IDENT {
            p.bump_any(); // colon
            p.bump_any(); // type name
        }
    }

    // Modifier: ?, *, +
    if p.at(SyntaxKind::QUESTION) || p.at(SyntaxKind::STAR) || p.at(SyntaxKind::PLUS) {
        p.bump_any();
    }

    m.complete_p(p, SyntaxKind::VARIABLE_REF);
}

/// Parse an element reference at top level: `&name selector;` or `&name(args) { body }`
pub fn element_ref(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::AMPERSAND);

    if p.at(SyntaxKind::IDENT) {
        p.bump_any();
    }

    inline_args(p);

    // Optional parenthesized args: &name(args)
    if p.at(SyntaxKind::L_PAREN) {
        arg_list(p);
    }

    // Optional body for inline scopes: &title [slot=title] { ... }
    if p.at(SyntaxKind::L_BRACE) {
        body_with_source(p, "");
    }

    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::ELEMENT_REF);
}

/// Parse an element reference inline (within CSS values): `&name`
pub fn element_ref_inline(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::AMPERSAND);

    if p.at(SyntaxKind::IDENT) {
        p.bump_any();
    }

    // Property path: &name.prop
    while p.at(SyntaxKind::DOT) {
        p.bump_any(); // consume DOT
        if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
            p.bump_any();
        } else {
            break; // DOT not followed by IDENT — stop path parsing
        }
    }

    m.complete_p(p, SyntaxKind::ELEMENT_REF);
}

/// Parse a meta definition: `%name { ... }` (without source-aware body dispatch).
///
/// This generic version is kept for backwards compatibility with code that
/// doesn't have source text available. For proper meta-clause handling with
/// keyword-based body dispatch (%form → raw body, %emit → island body),
/// use `grammar::meta::meta_def(p, source)` instead.
pub fn meta_def(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::PERCENT);

    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
    }

    inline_args(p);

    if p.at(SyntaxKind::L_PAREN) {
        arg_list(p);
    }

    if p.at(SyntaxKind::L_BRACE) {
        body_with_source(p, "");
    }

    m.complete_p(p, SyntaxKind::META_DEF);
}

/// Parse a scope block: selector(s) followed by `{ ... }`.
pub fn scope_or_property(p: &mut Parser) {
    // An element-type selector that opens a rule block (`div { }`,
    // `body:hover .nope { opacity: 1 }`) must be a scope block, NOT a property.
    // `at_property`'s two-token lookahead sees `body:hover` as `IDENT COLON` and
    // would parse the whole rule — and every sibling after it, as a property
    // value (the GH-23 poison: css_value/keyframe_block swallow the following
    // scopes). Resolve the ambiguity with the SAME forward scan the body parser
    // uses (the IDENT arm of `body_*`), so root and nested rules agree:
    // element scope block first, property second.
    if !(p.at(SyntaxKind::IDENT) && is_element_scope_block(p))
        && super::properties::at_property(p)
    {
        super::properties::css_property(p);
        return;
    }

    let m = p.start();

    // Parse selector (scope context: allows descendant combinators with IDENT)
    super::selectors::scope_selector(p);

    // Scope block body
    if p.at(SyntaxKind::L_BRACE) {
        body_with_source(p, "");
    }

    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::SCOPE_BLOCK);
}

/// Parse inline arguments — tokens between the directive name and `(` or `{`.
///
/// Wraps contiguous non-delimiter tokens in ARG nodes. Special tokens
/// (keywords, variable refs) get their own nodes.
///
/// Returns `true` when the LAST inline arg consumed was a standalone
/// value-introducing COLON (`@data derive $x :` then a `(`), so the caller can
/// route a following `(` as a paren-led VALUE expression rather than an
/// arg-list. The colon is parsed as its OWN arg (never lumped with a trailing
/// ident) so any following token — e.g. the `cubic-bezier` in a preset value —
/// resets the flag, exactly mirroring the CST parser's `last_was_value_colon`.
pub(crate) fn inline_args(p: &mut Parser) -> bool {
    let mut last_was_value_colon = false;
    while !p.at_end()
        && !p.at(SyntaxKind::L_PAREN)
        && !p.at(SyntaxKind::L_BRACE)
        && !p.at(SyntaxKind::R_BRACE)
        && !p.at(SyntaxKind::SEMICOLON)
    {
        if p.at(SyntaxKind::AT_SIGN) || p.at(SyntaxKind::PERCENT) {
            break;
        }

        // `:root { … }` / `* { … }` are newline-led selectors that begin a NEW
        // rule, exactly like the `.class`/`#id` guard below — they were simply
        // missing from that list. Without this a terminator-less directive
        // (`@edit-toggle`) swallows the selector as an argument and the rule's
        // `{ … }` as its own body, reporting E0946 against the DIRECTIVE up to
        // ~94 lines above the rule that actually caused it (BUG-356).
        //
        // This MUST precede the value-colon branch below, which consumes any
        // COLON unconditionally — placed after it, the `:root` half of this
        // guard is unreachable and only the `*` half works.
        //
        // Tested by reaching a `{`, not by the token, so a `*` meaning
        // multiplication and a `:` introducing a value both stay legal mid-run.
        if (p.at(SyntaxKind::COLON) || p.at(SyntaxKind::STAR))
            && p.newline_before_current()
            && is_selector_scope_block(p)
        {
            break;
        }

        // A value-introducing COLON is its OWN arg so the next token decides
        // routing (paren-led value vs arg-list). Lumping `: ident` into one arg
        // would make a preset value `~x: cubic-bezier(…)` look paren-led and
        // misroute its `(`. (BUG-077 events-grammar twin.)
        if p.at(SyntaxKind::COLON) {
            let m = p.start();
            p.bump_any();
            m.complete_p(p, SyntaxKind::ARG);
            last_was_value_colon = true;
            continue;
        }

        // A `.`/`#`/`<` after a newline begins a NEW top-level statement, not a
        // continuation of this directive's inline args. Without this guard a
        // bodyless directive like `@import "x"` (no trailing `;`) followed by
        // `\n.box { … }` would consume `.box` as an arg and `{ … }` as its body,
        // and `\n<main>…` would swallow the file-scope HTML (blank page, BUG-070).
        // (Twin of the CST parser's guard; this path feeds FormMatch extraction.)
        if (p.at(SyntaxKind::DOT) || p.at(SyntaxKind::HASH) || p.at(SyntaxKind::LT))
            && p.newline_before_current()
        {
            break;
        }


        // A newline-led bare element selector (`h1 { … }`, `body { … }`) is the
        // same case: a bodyless `@import "x"` followed by `\nh1 { … }` would
        // otherwise swallow `h1` as an arg and `{ … }` as its body, so the inner
        // directive never gets FormMatch'd (FUP-078). Twin of the CST guard.
        if p.at(SyntaxKind::IDENT) && p.newline_before_current() && is_element_scope_block(p) {
            break;
        }

        // Variable reference in inline args: $var
        if p.at(SyntaxKind::DOLLAR) {
            let m = p.start();
            variable_ref_inline(p);
            m.complete_p(p, SyntaxKind::ARG);
            last_was_value_colon = false;
            continue;
        }

        // A newline-led `&` begins a NEW element-ref / entity-scope statement
        // (`&name;`, `&name .sel { … }`, `&name { @c }`), not a continuation of this
        // directive's inline args. Without this a bodyless `@import "x"` followed by
        // `\n&evernet { @peak }` consumes `&evernet` as an arg and `{ … }` as its
        // body, so the interior directives never get FormMatch'd — they vanish from
        // emit (FEAT-142 WAVE A; the `&`-led twin of the FUP-078 / BUG-070 guards).
        if p.at(SyntaxKind::AMPERSAND) && p.newline_before_current() {
            break;
        }

        // Element reference in inline args: &elem
        if p.at(SyntaxKind::AMPERSAND) {
            let m = p.start();
            element_ref_inline(p);
            m.complete_p(p, SyntaxKind::ARG);
            last_was_value_colon = false;
            continue;
        }

        // Keyword tokens keep their own kind
        if p.current().is_keyword() {
            p.bump_any();
            last_was_value_colon = false;
            continue;
        }

        // Regular inline argument: wrap contiguous tokens in ARG node
        let m = p.start();
        let mut inner_count = 0;
        // Consume tokens for this arg until a boundary
        while !p.at_end()
            && !p.at(SyntaxKind::L_PAREN)
            && !p.at(SyntaxKind::L_BRACE)
            && !p.at(SyntaxKind::R_BRACE)
            && !p.at(SyntaxKind::SEMICOLON)
            && !p.at(SyntaxKind::AT_SIGN)
            && !p.at(SyntaxKind::DOLLAR)
            && !p.at(SyntaxKind::AMPERSAND)
            && !p.at(SyntaxKind::PERCENT)
            && !p.current().is_keyword()
        {
            // Stop this arg at a newline-led `.`/`#`/`<`: it starts a new
            // top-level statement (scope or HTML), not a continuation (mirrors
            // the outer-loop guard; `<` covers BUG-070 file-scope markup).
            if inner_count > 0
                && (p.at(SyntaxKind::DOT) || p.at(SyntaxKind::HASH) || p.at(SyntaxKind::LT))
                && p.newline_before_current()
            {
                break;
            }
            // Newline-led bare element selector ends this arg (FUP-078, mirrors
            // the outer-loop guard). `inner_count` need not be checked: the outer
            // loop already breaks before entering here on a fresh element scope.
            if p.at(SyntaxKind::IDENT) && p.newline_before_current() && is_element_scope_block(p) {
                break;
            }
            // Inline array-literal value: `[{"k": v}, {"k": w}]`. Consume the whole
            // BALANCED bracket region (depth-aware over `[] {} ()`) so the inner `{`
            // is not mistaken for the directive body and the value capture survives
            // intact (BUG-042). The opener lookahead keeps attribute selectors
            // (`[data-x]`, peek = IDENT) on the per-token path. Twin of the CST
            // parser's consume_balanced_brackets.
            if p.at(SyntaxKind::L_BRACKET)
                && matches!(
                    p.nth(1),
                    SyntaxKind::L_BRACE
                        | SyntaxKind::STRING
                        | SyntaxKind::NUMBER
                        | SyntaxKind::NUMBER_WITH_UNIT
                        | SyntaxKind::L_BRACKET
                        | SyntaxKind::R_BRACKET
                )
            {
                consume_balanced_brackets(p);
                inner_count += 1;
                continue;
            }
            // A dimension is ONE value spelled as two adjacent tokens at Kernel
            // tier (`500ms` = NUMBER + IDENT). Consuming only the number would
            // strand the unit, which surfaces as "Unexpected N extra inline
            // token(s) after directive" on ordinary calls like
            // `@fade-in-up(duration: 500ms)`.
            if p.at_dimension_start() {
                p.bump_any();
                p.bump_any();
                inner_count += 1;
                continue;
            }
            p.bump_any();
            inner_count += 1;
        }
        if inner_count == 0 {
            // Safety: avoid empty ARG nodes that consume no tokens
            break;
        }
        m.complete_p(p, SyntaxKind::ARG);
        last_was_value_colon = false;
    }
    last_was_value_colon
}

/// True when the `{` at the cursor opens an OBJECT LITERAL value (`{ "key": … }`)
/// rather than a directive body. An object literal begins with a quoted key
/// followed by `:` (`{ STRING COLON …`); a directive body holds IDENT-named
/// properties/directives and an `@fn` body may begin with a bare STRING followed
/// by an OPERATOR (`{ "Hello" + … }`), never `STRING COLON`. nth() already skips
/// trivia. (BUG-042 / FEAT-047 bare-object inline value.)
///
/// FEAT-166 will extend this to a BARE key (`{ ink: #e8eef7 }`) so a typed seed
/// can be written CSS-natively. That needs the `value_colon` guard as its
/// disambiguator — `{ ident: … }` is also the shape of a body full of CSS
/// declarations, and what separates them is POSITION (a value follows the `:`
/// that introduces it; a body follows a directive head). The guard alone is not
/// the feature though: the captured record still needs a typeref-directed
/// grammar to reach codegen, so it lands with that wave rather than ahead of it.
fn starts_object_literal(p: &Parser) -> bool {
    // A QUOTED key is the JSON-shaped object literal (`{ "k": v }`,
    // BUG-042 / FEAT-047): a directive body never opens with STRING COLON.
    let quoted_key = p.nth(1) == SyntaxKind::STRING && p.nth(2) == SyntaxKind::COLON;
    // A BARE key (`{ ink: #e8eef7; … }`, FEAT-166) is the CSS-native seed
    // surface, but it has the same shape as a directive BODY of CSS
    // declarations — so the lookahead alone is not enough. POSITION
    // discriminates (the `value_colon` guard the doc above prescribes): an
    // object value immediately follows the `:` that introduces it; a body
    // follows a directive head. Without the guard every `@form … { ident: … }`
    // / `@type` body was swallowed as an object value — E0946 across the tree.
    // (The CST parser's `bare_object_value` is this same test.)
    let bare_key = p.prev_non_trivia() == SyntaxKind::COLON
        && p.nth(1) == SyntaxKind::IDENT
        && p.nth(2) == SyntaxKind::COLON;
    quoted_key || bare_key
}

/// Consume a balanced `[ … ]` region (depth-aware over `[] {} ()`) into the
/// current open node. STRING is a single token, so quotes inside are inert.
/// Used to keep inline object-literal arrays whole (BUG-042). Twin of the CST
/// parser's `consume_balanced_brackets`.
/// Consume a paren-led inline VALUE expression: a balanced run from the opening
/// `(` until a depth-0 `;`/`{`/`}` (the statement boundary). Mirrors the CST
/// parser's `parse_inline_expression`; used when a value-colon is immediately
/// followed by `(` so the whole `(…).member(…)` value is one ARG and the
/// trailing call-args are not orphaned (BUG-077 events-grammar twin).
fn inline_value_expr(p: &mut Parser) {
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    while !p.at_end() {
        let at_boundary = paren_depth == 0 && bracket_depth == 0;
        match p.current() {
            SyntaxKind::L_PAREN => paren_depth += 1,
            SyntaxKind::R_PAREN => {
                if paren_depth == 0 {
                    break; // unmatched closer belongs to an enclosing group
                }
                paren_depth -= 1;
            }
            SyntaxKind::L_BRACKET => bracket_depth += 1,
            SyntaxKind::R_BRACKET => {
                if bracket_depth == 0 {
                    break;
                }
                bracket_depth -= 1;
            }
            SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE | SyntaxKind::R_BRACE if at_boundary => {
                break;
            }
            _ => {}
        }
        p.bump_any();
    }
}

fn consume_balanced_brackets(p: &mut Parser) {
    let mut depth = 0i32;
    while !p.at_end() {
        match p.current() {
            SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE | SyntaxKind::L_PAREN => {
                depth += 1;
                p.bump_any();
            }
            SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE | SyntaxKind::R_PAREN => {
                depth -= 1;
                p.bump_any();
                if depth <= 0 {
                    break;
                }
            }
            _ => p.bump_any(),
        }
    }
}

/// Parse a parenthesized argument list: `(arg1, arg2, ...)`.
///
/// Produces structured ARG nodes wrapping each argument value.
pub fn arg_list(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::L_PAREN);

    while !p.at_end() && !p.at(SyntaxKind::R_PAREN) {
        // Parse one argument
        arg(p);

        // Optional comma separator
        if p.at(SyntaxKind::COMMA) {
            p.bump_any();
        }
    }

    p.expect(SyntaxKind::R_PAREN);

    m.complete_p(p, SyntaxKind::ARG_LIST);
}

/// Parse a single argument within a parenthesized list.
fn arg(p: &mut Parser) {
    let m = p.start();

    // Variable reference as arg
    if p.at(SyntaxKind::DOLLAR) {
        variable_ref_inline(p);
        m.complete_p(p, SyntaxKind::ARG);
        return;
    }

    // Consume tokens until comma, closing paren, or end
    let mut paren_depth = 0i32;
    while !p.at_end() {
        if p.at(SyntaxKind::L_PAREN) {
            paren_depth += 1;
            p.bump_any();
        } else if p.at(SyntaxKind::R_PAREN) {
            if paren_depth > 0 {
                paren_depth -= 1;
                p.bump_any();
            } else {
                break;
            }
        } else if p.at(SyntaxKind::COMMA) && paren_depth == 0 {
            break;
        } else {
            p.bump_any();
        }
    }

    m.complete_p(p, SyntaxKind::ARG);
}

/// Parse a body block: `{ ... }`.
///
/// Dispatches to appropriate parsers for body items (CSS properties,
/// directives, scope blocks, variable refs, etc.).
pub fn body(p: &mut Parser) {
    body_with_source(p, "");
}

/// Parse a body block with source text for meta-clause keyword dispatch.
pub fn body_with_source(p: &mut Parser, source: &str) {
    let m = p.start();

    p.expect(SyntaxKind::L_BRACE);

    while !p.at_end() && !p.at(SyntaxKind::R_BRACE) {
        body_item(p, source);
    }

    p.expect(SyntaxKind::R_BRACE);

    m.complete_p(p, SyntaxKind::BODY);
}

/// Parse a single item within a body block.
fn body_item(p: &mut Parser, source: &str) {
    match p.current() {
        // Directive: @name ...
        SyntaxKind::AT_SIGN => {
            directive_with_source(p, source);
        }

        // Variable: $name ... (inline or declaration depending on context)
        SyntaxKind::DOLLAR => {
            // Check if this is a variable declaration ($name type: value;)
            // or just a variable reference ($name used in an expression)
            if is_variable_declaration(p) {
                variable_ref(p);
            } else {
                variable_ref_inline(p);
            }
        }

        // Element reference: &name ...
        SyntaxKind::AMPERSAND => {
            element_ref(p);
        }

        // Meta definition: %name ...
        SyntaxKind::PERCENT => {
            if source.is_empty() {
                meta_def(p);
            } else {
                super::meta::meta_def(p, source);
            }
        }

        // Class selector scope: .class { ... }
        SyntaxKind::DOT => {
            if is_scope_block(p) {
                scope_block(p);
            } else if is_class_property(p) {
                // Reactive class toggle at selector scope: `.active: $v;` (no `{`).
                // Parse as a CSS_PROPERTY whose name KEEPS the leading `.` so the emitter
                // routes it to classList.toggle (BUG-068). Without this the dot was dropped
                // and `active` mis-lowered to setAttribute.
                super::properties::class_property(p);
            } else {
                // Stray dot — consume
                p.bump_any();
            }
        }

        // ID selector or universal selector scope
        SyntaxKind::HASH | SyntaxKind::STAR => {
            scope_block(p);
        }

        // Attribute selector scope: `[data-st-instance] { ... }` (FEAT-127).
        // Mirrors the CST parser's top-level L_BRACKET arm; without this a
        // nested attribute selector's body directives never become a SCOPE_BLOCK.
        SyntaxKind::L_BRACKET => {
            scope_block(p);
        }

        // Pseudo block: :hover { ... }
        SyntaxKind::COLON => {
            scope_block(p);
        }

        // Identifier: CSS property, element selector scope, or bare ident
        SyntaxKind::IDENT => {
            if is_element_scope_block(p) {
                scope_block(p);
            } else if super::properties::at_property(p) {
                super::properties::css_property(p);
            } else {
                // Bare identifier (e.g., inside meta blocks: `event <- $event;`)
                // Consume as generic tokens until semicolon or brace boundary
                while !p.at_end()
                    && !p.at(SyntaxKind::SEMICOLON)
                    && !p.at(SyntaxKind::R_BRACE)
                    && !p.at(SyntaxKind::L_BRACE)
                    && !p.at(SyntaxKind::AT_SIGN)
                    && !p.at(SyntaxKind::PERCENT)
                {
                    p.bump_any();
                }
                p.eat(SyntaxKind::SEMICOLON);
            }
        }

        // Keywords that start properties
        kind if kind.is_keyword() => {
            if super::properties::at_property(p) {
                super::properties::css_property(p);
            } else {
                p.bump_any();
            }
        }

        // HTML element literal inside a body (PLAN-039): `<div>…</div>` in a @template /
        // scope body. Without this arm a `<` falls through to `_ => bump_any` and the
        // element's `>` plus everything AFTER it is swallowed token-by-token — so sibling
        // constructs (a following `.rt { @editable }` scope, `@on`, `@each`) never become
        // structural nodes and can't surface as matches. Mirror grammar::root's file-scope
        // handling: skip the whole HTML region opaquely via scan_html_end (its reactive
        // holes are lowered separately). Guarded to a real element (`<` then a tag ident or
        // `/`), so a stray `<` comparison still falls through to the default consume.
        SyntaxKind::LT if lt_starts_html_element(p) => {
            let start = p.current_offset() as usize;
            let end = crate::syntax::cst::scan_html_end(source, start);
            while !p.at_end() && (p.current_offset() as usize) < end {
                p.bump_any();
            }
            if (p.current_offset() as usize) <= start && !p.at_end() {
                p.bump_any();
            }
        }

        // String literal (e.g., in @fn bodies)
        SyntaxKind::STRING => {
            p.bump_any();
        }

        // Nested brace block
        SyntaxKind::L_BRACE => {
            nested_braces(p);
        }

        // Any other token: consume to make progress
        _ => {
            p.bump_any();
        }
    }
}

/// PLAN-039: true when the current `<` begins an HTML element — i.e. `<` is immediately
/// followed (after trivia) by a tag name (IDENT/keyword) or `/` (a close tag). A stray `<`
/// (a comparison operator) is NOT followed by either and falls through to the default
/// consume. Mirrors the CST parser's lt_starts_html_element guard.
fn lt_starts_html_element(p: &Parser) -> bool {
    let next = p.nth(1);
    next == SyntaxKind::IDENT || next == SyntaxKind::SLASH || next.is_keyword()
}

/// Parse a scope block (selector + body).
fn scope_block(p: &mut Parser) {
    let m = p.start();

    super::selectors::scope_selector(p);

    if p.at(SyntaxKind::L_BRACE) {
        body_with_source(p, "");
    }

    m.complete_p(p, SyntaxKind::SCOPE_BLOCK);
}

/// Check if current position starts a scope block: `.class { }` or `#id { }`.
fn is_scope_block(p: &Parser) -> bool {
    // DOT or HASH followed by IDENT ... L_BRACE pattern
    let mut i = 1usize; // skip the DOT/HASH

    // Skip optional ident
    if p.nth(i) == SyntaxKind::IDENT {
        i += 1;
    }

    // Look for L_BRACE (possibly after more selector tokens)
    for _ in 0..20 {
        let k = p.nth(i);
        match k {
            SyntaxKind::L_BRACE => return true,
            SyntaxKind::EOF | SyntaxKind::SEMICOLON | SyntaxKind::R_BRACE => return false,
            // Selector continuation tokens
            SyntaxKind::DOT
            | SyntaxKind::HASH
            | SyntaxKind::IDENT
            | SyntaxKind::L_BRACKET
            | SyntaxKind::R_BRACKET
            | SyntaxKind::COLON
            | SyntaxKind::STAR
            | SyntaxKind::GT
            | SyntaxKind::PLUS
            | SyntaxKind::TILDE
            | SyntaxKind::COMMA => {
                i += 1;
            }
            // Not a selector continuation — probably not a scope block
            _ => return false,
        }
    }
    false
}

/// Check if `.<ident> :` starts a reactive class-toggle property (no `{` block):
/// `.active: $v;`. Distinguishes from a scope block (`.x { }`, handled first).
fn is_class_property(p: &Parser) -> bool {
    // DOT IDENT|kw COLON ... (a property), not DOT IDENT { (a scope block). Keyword class
    // names (`on`/`from`/`to`) are valid — match the CST guard + class_property body.
    (p.nth(1) == SyntaxKind::IDENT || p.nth(1).is_keyword()) && p.nth(2) == SyntaxKind::COLON
}

/// Whether a newline-led `:` or `*` begins a selector that reaches a `{`.
///
/// `:root { … }` / `*, .x { … }` start a RULE; a `:` introducing a value and a
/// `*` meaning multiplication do not. Both spellings share the selector scan
/// with `is_element_scope_block` — the only difference is which token is
/// skipped to reach it — so the vocabulary of "what may appear in a selector"
/// stays defined in exactly one place (BUG-356).
fn is_selector_scope_block(p: &Parser) -> bool {
    // `::before` — skip the second colon so the scan starts at the name.
    let skip = if p.at(SyntaxKind::COLON) && p.nth(1) == SyntaxKind::COLON {
        2
    } else {
        1
    };
    scan_selector_to_brace(p, skip)
}

/// Check if current IDENT starts an element scope block: `div { }`, `body { }`.
fn is_element_scope_block(p: &Parser) -> bool {
    scan_selector_to_brace(p, 1) // skip the IDENT
}

/// Scan forward through CSS selector continuation tokens, returning true only
/// if the chain reaches an `L_BRACE`. Shared by both scope-block tests.
fn scan_selector_to_brace(p: &Parser, start: usize) -> bool {
    let mut i = start;

    for _ in 0..30 {
        let k = p.nth(i);
        match k {
            SyntaxKind::L_BRACE => return true,
            SyntaxKind::EOF | SyntaxKind::SEMICOLON | SyntaxKind::R_BRACE => return false,
            // Colon: could be pseudo-class (:hover, ::before) or CSS property (name: value).
            // Look ahead past the colon to distinguish.
            SyntaxKind::COLON => {
                let mut j = i + 1;
                // Double colon for pseudo-element: ::before, ::after
                if p.nth(j) == SyntaxKind::COLON {
                    j += 1;
                }
                // If followed by IDENT/keyword, it's a pseudo-class selector continuation
                if p.nth(j) == SyntaxKind::IDENT || p.nth(j).is_keyword() {
                    j += 1;
                    // Skip optional functional args: :nth-child(2n+1)
                    if p.nth(j) == SyntaxKind::L_PAREN {
                        let mut depth = 1i32;
                        j += 1;
                        while depth > 0 && p.nth(j) != SyntaxKind::EOF {
                            if p.nth(j) == SyntaxKind::L_PAREN {
                                depth += 1;
                            } else if p.nth(j) == SyntaxKind::R_PAREN {
                                depth -= 1;
                            }
                            j += 1;
                        }
                    }
                    i = j;
                    continue;
                } else {
                    // COLON not followed by pseudo name = CSS property colon
                    return false;
                }
            }
            // L_PAREN means function call, not scope
            SyntaxKind::L_PAREN => return false,
            // Selector continuation tokens
            SyntaxKind::DOT
            | SyntaxKind::HASH
            | SyntaxKind::IDENT
            | SyntaxKind::L_BRACKET
            | SyntaxKind::R_BRACKET
            | SyntaxKind::STAR
            | SyntaxKind::GT
            | SyntaxKind::PLUS
            | SyntaxKind::TILDE
            | SyntaxKind::COMMA
            | SyntaxKind::AMPERSAND => {
                i += 1;
            }
            _ => return false,
        }
    }
    false
}

/// Check if current DOLLAR starts a variable declaration.
///
/// Variable declarations: `$name type: value;` or `$name type;`
/// Variable properties: `$name: value;` (colon right after name)
/// Mutations: `$name <- value;`
///
/// As opposed to inline references: `$name + $other` or `$name.prop`
fn is_variable_declaration(p: &Parser) -> bool {
    // $name is at position 0 (DOLLAR) + 1 (IDENT)
    let mut i = 1usize;

    // Skip past the identifier
    if p.nth(i) != SyntaxKind::IDENT && !p.nth(i).is_keyword() {
        return false;
    }
    i += 1;

    // Check what follows the name:
    match p.nth(i) {
        // $name: value; → variable property
        SyntaxKind::COLON => true,
        // $name type → variable declaration with type
        SyntaxKind::IDENT => {
            // Skip past type name and optional suffixes: string[], Product[], string?
            let mut j = i + 1;
            // Skip array brackets: []
            if p.nth(j) == SyntaxKind::L_BRACKET && p.nth(j + 1) == SyntaxKind::R_BRACKET {
                j += 2;
            }
            // Skip optional marker: ?
            if p.nth(j) == SyntaxKind::QUESTION {
                j += 1;
            }
            // Look for : or ; after the type
            p.nth(j) == SyntaxKind::COLON || p.nth(j) == SyntaxKind::SEMICOLON
        }
        // $name <- value → mutation
        SyntaxKind::LEFT_ARROW => true,
        // $name; → just a name with semicolon (declaration)
        SyntaxKind::SEMICOLON => true,
        // $name { → declaration with body
        SyntaxKind::L_BRACE => true,
        // Anything else is likely an expression context
        _ => false,
    }
}

/// Parse balanced nested braces: `{ ... }`.
fn nested_braces(p: &mut Parser) {
    p.bump_any(); // L_BRACE
    let mut depth = 1i32;
    while !p.at_end() && depth > 0 {
        if p.at(SyntaxKind::L_BRACE) {
            depth += 1;
        } else if p.at(SyntaxKind::R_BRACE) {
            depth -= 1;
            if depth == 0 {
                p.bump_any();
                return;
            }
        }
        p.bump_any();
    }
}

#[cfg(test)]
mod tests {
    use super::super::root;
    use crate::parser::SourceSpan;
    use crate::parser::meta_ast::{
        CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, FormParam,
        MacroDefAst,
    };
    use crate::syntax::cst::SyntaxKind;
    use crate::syntax::cst::lexer::Lexer;
    use crate::syntax::events::diag_sink::DiagnosticSink;
    use crate::syntax::events::event::Event;
    use crate::syntax::events::extractors::ExtractorRegistry;
    use crate::syntax::events::form_compiler::CompiledRegistry;
    use crate::syntax::events::test_support::{driver_member, register_driver_expr};
    use crate::syntax::events::input::Input;
    use crate::syntax::events::match_sink::MatchSink;
    use crate::syntax::events::parser::Parser;
    use crate::syntax::events::process::process;
    use crate::syntax::events::strategy::ErrorStrategy;
    use crate::syntax::events::tree_sink::TreeSink;
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
    

    /// Lex, parse, and return events.
    fn parse_source(source: &str) -> (Vec<Event>, Input) {
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let input = Input::from_tokens(&tokens);
        let mut parser = Parser::new(&input, ErrorStrategy::Verbose);
        root(&mut parser, source);
        (parser.finish(), input)
    }

    #[test]
    fn poc_directive_produces_cst() {
        let source = "@on &.hover";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directive = tree.children().find(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(directive.is_some(), "Should have DIRECTIVE node");
    }

    #[test]
    fn poc_directive_with_args_produces_cst() {
        let source = "@on &.hover(0.3s)";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directive = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .expect("Should have DIRECTIVE");

        let arg_list = directive
            .children()
            .find(|n| n.kind() == SyntaxKind::ARG_LIST);
        assert!(arg_list.is_some(), "DIRECTIVE should contain ARG_LIST");
    }

    #[test]
    fn poc_directive_with_body_produces_cst() {
        let source = "@on &.hover { opacity: 0; }";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directive = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .expect("Should have DIRECTIVE");

        let body = directive.children().find(|n| n.kind() == SyntaxKind::BODY);
        assert!(body.is_some(), "DIRECTIVE should contain BODY");
    }

    #[test]
    fn poc_full_directive_cst_and_match() {
        // The critical POC test: parse "@on hover(0.3s) { opacity: 0 -> 1; }"
        // through BOTH TreeSink AND MatchSink from the same events.
        let source = "@on &.hover(0.3s) { opacity: 0 -> 1; }";
        let (mut events, input) = parse_source(source);

        // 1. TreeSink produces valid CST
        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, cst_errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(cst_errors.is_empty(), "CST errors: {:?}", cst_errors);

        // Verify CST structure: ROOT > DIRECTIVE > [ARG_LIST, BODY]
        let directive = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .expect("Should have DIRECTIVE");
        assert!(
            directive
                .children()
                .any(|n| n.kind() == SyntaxKind::ARG_LIST),
            "DIRECTIVE should have ARG_LIST"
        );
        assert!(
            directive.children().any(|n| n.kind() == SyntaxKind::BODY),
            "DIRECTIVE should have BODY"
        );

        // 2. MatchSink produces FormMatch
        //    Re-parse to get fresh events (process consumes)
        let (mut events2, input2) = parse_source(source);
        let mut syntax_reg = SyntaxRegistry::new();
        syntax_reg.register(&make_test_macro_def(
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
                params: vec![FormParam {
                    name: String::new(),
                    // PLAN-122 W1.2/W3: this form is built BY HAND, so it can
                    // only name capture types the Rust enum still has. The
                    // scalar types (`time`, `length`, `color`, `easing`) left
                    // that enum for stdlib grammars in css-values.st, and the
                    // `Time` arm's extractor needed the fused NUMBER_WITH_UNIT
                    // token that the Kernel tier no longer produces.
                    //
                    // What this test is FOR is the two-sink POC — that one event
                    // stream feeds both TreeSink and MatchSink and both agree.
                    // `Number` keeps that question intact without re-testing the
                    // scalar grammars, which have their own gates (a real
                    // `$x:time` capture is exercised end-to-end through stdlib's
                    // own forms).
                    elements: vec![FormInlineElement::Capture(
                        FormCapture {
                            var_name: "duration".to_string(),
                            capture_type: CaptureType::Number,
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
            },
        ));
        let mut extractors = ExtractorRegistry::new();
        register_driver_expr(&mut extractors);
        let compiled = CompiledRegistry::from_registry(&syntax_reg, &extractors);
        let mut match_sink = MatchSink::new(source, compiled, extractors);
        process(&mut events2, source, &input2, &mut match_sink);
        let (matches, _match_errors) = match_sink.finish();

        assert_eq!(matches.len(), 1, "Should produce exactly one FormMatch");
        let fm = &matches[0];
        assert_eq!(fm.macro_name, "on");

        use crate::syntax::form_match::CapturedValue;
        assert_eq!(
            driver_member(fm, "driver").as_deref(),
            Some("hover"),
            "Should capture driver member"
        );
        assert_eq!(
            fm.captures.get("duration"),
            Some(&CapturedValue::Number(0.3)),
            "Should capture duration from arg list"
        );

        // 3. DiagnosticSink produces no errors for valid input
        let (mut events3, input3) = parse_source(source);
        let mut diag_sink = DiagnosticSink::new();
        process(&mut events3, source, &input3, &mut diag_sink);
        let diagnostics = diag_sink.finish();
        assert!(
            diagnostics.is_empty(),
            "Valid input should produce no diagnostics"
        );
    }

    #[test]
    fn poc_variable_ref() {
        let source = "$count number;";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(errors.is_empty());

        let var_ref = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::VARIABLE_REF);
        assert!(var_ref.is_some(), "Should have VARIABLE_REF node");
    }

    #[test]
    fn poc_element_ref() {
        let source = "&button .btn;";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(errors.is_empty());

        let elem_ref = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::ELEMENT_REF);
        assert!(elem_ref.is_some(), "Should have ELEMENT_REF node");
    }

    #[test]
    fn poc_cst_preserves_all_text() {
        // Lossless: CST text should reconstruct to original source
        let source = "@on &.hover(0.3s) { opacity: 0 -> 1; }";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, _) = tree_sink.finish();

        // Rowan green tree should reconstruct original text
        let reconstructed: String = tree
            .descendants_with_tokens()
            .filter_map(|el| el.into_token())
            .map(|tok| tok.text().to_string())
            .collect();

        assert_eq!(
            reconstructed, source,
            "CST should losslessly preserve source text"
        );
    }

    #[test]
    fn body_produces_css_properties() {
        let source = "@on &.hover { opacity: 0 -> 1; }";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directive = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .expect("Should have DIRECTIVE");

        let body = directive
            .children()
            .find(|n| n.kind() == SyntaxKind::BODY)
            .expect("DIRECTIVE should have BODY");

        let css_prop = body
            .children()
            .find(|n| n.kind() == SyntaxKind::CSS_PROPERTY);
        assert!(css_prop.is_some(), "BODY should contain CSS_PROPERTY");
    }

    #[test]
    fn arg_list_produces_arg_nodes() {
        let source = "@computed total($a, $b) { $a + $b }";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directive = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .expect("Should have DIRECTIVE");

        let arg_list = directive
            .children()
            .find(|n| n.kind() == SyntaxKind::ARG_LIST)
            .expect("DIRECTIVE should have ARG_LIST");

        let args: Vec<_> = arg_list
            .children()
            .filter(|n| n.kind() == SyntaxKind::ARG)
            .collect();
        assert_eq!(args.len(), 2, "ARG_LIST should have 2 ARG nodes");
    }

    /// Helper: parse source through event-based parser and verify lossless CST.
    fn assert_parses_losslessly(source: &str) {
        let (mut events, input) = parse_source(source);
        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert_eq!(tree.kind(), SyntaxKind::ROOT, "Root should be ROOT");
        assert!(
            errors.is_empty(),
            "Unexpected errors for {:?}: {:?}",
            source,
            errors
        );

        // Lossless: reconstruct source from tokens
        let reconstructed: String = tree
            .descendants_with_tokens()
            .filter_map(|el| el.into_token())
            .map(|tok| tok.text().to_string())
            .collect();
        assert_eq!(
            reconstructed, source,
            "CST should losslessly preserve source text"
        );
    }

    /// Helper: parse and return the first DIRECTIVE node.
    fn parse_directive(source: &str) -> crate::syntax::cst::SyntaxNode {
        let (mut events, input) = parse_source(source);
        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
        tree.children()
            .find(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .expect("Should have DIRECTIVE node")
    }

    // =======================================================
    // All 11 directive families through generic event parser
    // =======================================================

    #[test]
    fn all_families_on_event() {
        assert_parses_losslessly("@on &.hover { opacity: 0 -> 1; }");
        assert_parses_losslessly("@on &.click(0.3s) { scale: 0.95 -> 1; }");
        assert_parses_losslessly("@on &.focus { outline: 2px solid blue; }");

        let d = parse_directive("@on &.hover(0.3s) { opacity: 0 -> 1; }");
        assert!(d.children().any(|n| n.kind() == SyntaxKind::ARG_LIST));
        assert!(d.children().any(|n| n.kind() == SyntaxKind::BODY));
    }

    #[test]
    fn all_families_scroll() {
        assert_parses_losslessly("@scroll page-progress { opacity: 0 -> 1; }");
        assert_parses_losslessly(
            "@scroll viewport(0.2, 0.8) { transform: scale(0.8) -> scale(1); }",
        );
    }

    #[test]
    fn all_families_load() {
        assert_parses_losslessly("@load(0.5s) { opacity: 0 -> 1; }");
        assert_parses_losslessly(
            "@load(0.5s, ease-out) { opacity: 0 -> 1; transform: translateY(20px) -> translateY(0); }",
        );
    }

    #[test]
    fn all_families_loop() {
        assert_parses_losslessly("@loop(2s) { opacity: 0 -> 1 -> 0; }");
        assert_parses_losslessly(
            "@loop(1s, ease-in-out) { transform: rotate(0deg) -> rotate(360deg); }",
        );
    }

    #[test]
    fn all_families_each() {
        assert_parses_losslessly("@each $item in $items { .card { } }");
        assert_parses_losslessly("@each $user in $users { .profile { } }");

        let d = parse_directive("@each $item in $items { .card { } }");
        assert!(d.children().any(|n| n.kind() == SyntaxKind::BODY));
    }

    #[test]
    fn all_families_data() {
        assert_parses_losslessly("@data users from \"/api/users\"");
        assert_parses_losslessly("@data posts from \"/api/posts\"");
    }

    #[test]
    fn all_families_type() {
        assert_parses_losslessly("@type User { id: number; name: string; }");

        let d = parse_directive("@type User { id: number; name: string; }");
        assert!(d.children().any(|n| n.kind() == SyntaxKind::BODY));
    }

    #[test]
    fn all_families_fn() {
        assert_parses_losslessly("@fn greet($name) { \"Hello, \" + $name }");

        let d = parse_directive("@fn greet($name) { \"Hello, \" + $name }");
        assert!(d.children().any(|n| n.kind() == SyntaxKind::ARG_LIST));
        assert!(d.children().any(|n| n.kind() == SyntaxKind::BODY));
    }

    #[test]
    fn fn_with_return_type_has_body() {
        let source = "@fn formatPrice(price: number): string { \"$\" + price }";
        assert_parses_losslessly(source);

        let d = parse_directive(source);
        assert!(
            d.children().any(|n| n.kind() == SyntaxKind::ARG_LIST),
            "DIRECTIVE should have ARG_LIST"
        );
        assert!(
            d.children().any(|n| n.kind() == SyntaxKind::BODY),
            "DIRECTIVE should have BODY even with return type annotation between ) and {{"
        );
    }

    #[test]
    fn all_families_computed() {
        assert_parses_losslessly("@computed total($a, $b) { $a + $b }");

        let d = parse_directive("@computed total($a, $b) { $a + $b }");
        assert!(d.children().any(|n| n.kind() == SyntaxKind::ARG_LIST));
        assert!(d.children().any(|n| n.kind() == SyntaxKind::BODY));
    }

    #[test]
    fn all_families_let() {
        // @let is parsed as a directive
        assert_parses_losslessly("@let x = 42;");
    }

    #[test]
    fn all_families_pattern() {
        assert_parses_losslessly("@pattern fade-in { opacity: 0 -> 1; }");

        let d = parse_directive("@pattern fade-in { opacity: 0 -> 1; }");
        assert!(d.children().any(|n| n.kind() == SyntaxKind::BODY));
    }

    // =======================================================
    // Meta definitions through generic event parser
    // =======================================================

    #[test]
    fn meta_def_all_types() {
        // All 8 meta-clause types parse generically
        assert_parses_losslessly("%primitive button { }");
        assert_parses_losslessly("%macro fade-in { }");
        assert_parses_losslessly("%form { @on $event:ident }");
        assert_parses_losslessly("%capture_type easing { }");
        assert_parses_losslessly("%bind { event <- $event; }");
        assert_parses_losslessly("%emit js { console.log('hello'); }");
        assert_parses_losslessly("%yield animation;");
        assert_parses_losslessly("%cleanup { removeEventListener(); }");
    }

    // =======================================================
    // Variable and element refs through generic event parser
    // =======================================================

    #[test]
    fn variable_ref_declaration() {
        assert_parses_losslessly("$count number: 0;");

        let (mut events, input) = parse_source("$count number: 0;");
        let mut tree_sink = TreeSink::new();
        process(&mut events, "$count number: 0;", &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty());
        assert!(
            tree.children()
                .any(|n| n.kind() == SyntaxKind::VARIABLE_REF)
        );
    }

    #[test]
    fn element_ref_declaration() {
        assert_parses_losslessly("&button .btn;");

        let (mut events, input) = parse_source("&button .btn;");
        let mut tree_sink = TreeSink::new();
        process(&mut events, "&button .btn;", &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty());
        assert!(tree.children().any(|n| n.kind() == SyntaxKind::ELEMENT_REF));
    }

    #[test]
    fn element_ref_with_arg_list() {
        assert_parses_losslessly("&card($title);");

        let (mut events, input) = parse_source("&card($title);");
        let mut tree_sink = TreeSink::new();
        process(&mut events, "&card($title);", &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let elem_ref = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::ELEMENT_REF)
            .expect("Should have ELEMENT_REF node");

        assert!(
            elem_ref
                .children()
                .any(|n| n.kind() == SyntaxKind::ARG_LIST),
            "ELEMENT_REF should have ARG_LIST child"
        );
    }

    #[test]
    fn element_ref_with_arg_list_and_body() {
        let source = "&card($title) { <div>$title</div> }";
        assert_parses_losslessly(source);

        let (mut events, input) = parse_source(source);
        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let elem_ref = tree
            .children()
            .find(|n| n.kind() == SyntaxKind::ELEMENT_REF)
            .expect("Should have ELEMENT_REF node");

        assert!(
            elem_ref
                .children()
                .any(|n| n.kind() == SyntaxKind::ARG_LIST),
            "ELEMENT_REF should have ARG_LIST child"
        );
        assert!(
            elem_ref.children().any(|n| n.kind() == SyntaxKind::BODY),
            "ELEMENT_REF should have BODY child"
        );
    }

    // =======================================================
    // Scope blocks and nested constructs
    // =======================================================

    #[test]
    fn scope_block_class_selector() {
        assert_parses_losslessly(".card { padding: 16px; }");

        let (mut events, input) = parse_source(".card { padding: 16px; }");
        let mut tree_sink = TreeSink::new();
        process(
            &mut events,
            ".card { padding: 16px; }",
            &input,
            &mut tree_sink,
        );
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty());
        assert!(tree.children().any(|n| n.kind() == SyntaxKind::SCOPE_BLOCK));
    }

    #[test]
    fn nested_directive_in_body() {
        assert_parses_losslessly(".card { @on &.hover { opacity: 0.8; } }");
    }

    #[test]
    fn no_per_family_functions() {
        // This test documents the design invariant: the grammar module has no
        // per-directive-family functions (parse_on, parse_scroll, etc.).
        // All directives go through the single generic directive() function.
        // If this comment is ever inaccurate, the test serves as a reminder.
        let families = [
            "@on &.hover { }",
            "@scroll page-progress { }",
            "@load(0.5s) { }",
            "@loop(2s) { }",
            "@each $x in $xs { }",
            "@data items from \"/api\"",
            "@type T { }",
            "@fn f($x) { }",
            "@computed c($x) { }",
            "@let x = 1;",
            "@pattern p { }",
        ];

        for source in families {
            let (mut events, input) = parse_source(source);
            let mut tree_sink = TreeSink::new();
            process(&mut events, source, &input, &mut tree_sink);
            let (tree, errors) = tree_sink.finish();
            assert!(
                errors.is_empty(),
                "Family {:?} had errors: {:?}",
                source,
                errors
            );
            assert!(
                tree.children().any(|n| n.kind() == SyntaxKind::DIRECTIVE),
                "Family {:?} should produce DIRECTIVE node",
                source,
            );
        }
    }

    #[test]
    fn element_pseudo_scope_followed_by_media_directive() {
        // Regression test: `a:hover { }` inside a body must not prevent
        // subsequent `@media(...)` from being parsed as a directive.
        let source = r#".foo a:hover {
    color: red;
}

.bar {
    display: none;

    @media("max-width: 768px") {
        display: flex;
    }
}"#;
        let (mut events, input) = parse_source(source);
        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();

        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        // Collect all SCOPE_BLOCK nodes
        let scope_blocks: Vec<_> = tree
            .children()
            .filter(|n| n.kind() == SyntaxKind::SCOPE_BLOCK)
            .collect();
        assert_eq!(
            scope_blocks.len(),
            2,
            "Should have 2 scope blocks (.foo a:hover and .bar)"
        );

        // The second scope (.bar) should contain a DIRECTIVE (@media) in its BODY
        let bar_scope = &scope_blocks[1];
        let bar_body = bar_scope
            .children()
            .find(|n| n.kind() == SyntaxKind::BODY)
            .expect(".bar should have BODY");

        let has_directive = bar_body
            .children()
            .any(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(
            has_directive,
            ".bar's body should contain a DIRECTIVE (@media)"
        );
    }

    #[test]
    fn fup078_import_then_element_scope_directive_is_matched() {
        // FUP-078 (events-parser twin of the CST guard): the FormMatch extractor
        // runs on the SAME token stream. A bare `@import` followed by a newline-led
        // element scope whose body holds a directive must yield BOTH the `import`
        // match AND the inner directive match. Before the guard the element ident
        // was swallowed as an inline arg, the scope became the import's body, and
        // the inner directive produced NO FormMatch — collapsing runtime JS to 0.
        use crate::syntax::{STDLIB_REGISTRY, events::parse_matches};
        let cases: &[&str] = &[
            "@import \"stdlib/text\"\nh1 { @balance(lineHeight:1.2) }",
            "@import \"stdlib/text\"\nh1.x { @balance(lineHeight:1.2) }",
            "@import \"stdlib/text\"\nbody { margin: 0 }\nh1 { @balance(lineHeight:1.2) }",
        ];
        for src in cases {
            let (matches, _) = parse_matches(src, &STDLIB_REGISTRY);
            let names: Vec<&str> = matches.iter().map(|m| m.macro_name.as_str()).collect();
            assert!(
                names.contains(&"import"),
                "[{src}] expected an `import` match, got {names:?}"
            );
            assert!(
                names.contains(&"balance"),
                "[{src}] the directive inside the recovered element scope must be \
                 matched (was swallowed by the bare @import), got {names:?}"
            );
        }
    }
    #[test]
    fn directive_value_atom_keeps_match_body_in_one_outer_arg() {
        let source = "@data derive $x State : @match { _ => Ready; };";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directives: Vec<_> = tree
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::DIRECTIVE)
            .collect();
        assert_eq!(
            directives.len(),
            1,
            "the value @match must not become a sibling/nested DIRECTIVE"
        );
        assert!(
            directives[0].children().any(|node| {
                node.kind() == SyntaxKind::ARG && node.text().to_string().contains("@match")
            }),
            "the entire @match body must be one outer ARG"
        );
    }

    #[test]
    fn directive_value_atom_is_generic_and_preserves_following_sibling() {
        let source = "@data inline $x : @custom;\n@on click {}";
        let (mut events, input) = parse_source(source);

        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        let (tree, errors) = tree_sink.finish();
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);

        let directives: Vec<_> = tree
            .children()
            .filter(|node| node.kind() == SyntaxKind::DIRECTIVE)
            .collect();
        assert_eq!(directives.len(), 2, "the sibling @on must not be swallowed");
        assert!(
            directives[0].children().any(|node| {
                node.kind() == SyntaxKind::ARG && node.text().to_string().contains("@custom;")
            }),
            "a non-match directive value must be captured as an opaque ARG"
        );
    }
}
