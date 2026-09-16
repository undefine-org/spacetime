// @match cond-mode (guard) arm grammar (PLAN-077 W1) — the VALUE-PRODUCING form.
//
// Distinct from dispatch-mode (match-arms.st): a cond-mode @match has NO subject
// (`@match { (guard) => Variant; }`, not `@match $subject { pat => &tpl(); }`), its
// arm pattern is an arbitrary boolean GUARD expression (not a string/ident/variant
// pattern), and its consequence is ALWAYS a variant constructor (never a template
// invocation — cond-mode only produces values, it never renders). Consumed by
// `@data derive $x T : @match { … }` (W2's derive form).
//
// GUARD DELIMITATION — `=>` is now a first-class FAT_ARROW token and a depth-0
// ExprExtractor boundary, so both `guard => Variant;` and `(guard) => Variant;`
// are sound. Parentheses remain available for visual grouping; they are no longer
// grammar-mandatory. The catch-all remains bare `_` because it is unambiguous at
// the choice point and reads as the ordinary wildcard.
//
// TOTALITY (E0931, hard error per PLAN-077 Mandate #4): the last arm MUST be a
// `_` catch-all. Guards are arbitrary booleans — the compiler cannot verify N
// conditions cover all cases, so the checkable invariant is that the chain
// always terminates in a value. The grammar admits an absent catch-all so W3's
// E0931 diagnostic can NAME the missing arm instead of failing with a generic
// parse error.

/// One payload argument inside a variant constructor: an `expr` with an OPTIONAL
/// trailing comma absorbed into the unit — the established comma-list idiom
/// (`param_list`, `variant_payload`). Factored so the arg list repeats under a
/// SINGLE NAMED capture (`$args:vc_arg+`) — the only shape the sequence
/// extractor threads as a named Array. `expr` self-limits at the top-level `,`
/// or the closing `)` (both are ExprExtractor boundaries at depth 0).
%capture_type vc_arg {
    $arg:expr ","?
}

/// A variant constructor: `Blue` (payload-less) or `Loaded($data)` /
/// `Pair($a, $b)` (payload-carrying, comma-separated expr args). Empty `()`
/// is rejected (`+`) — a payload-less variant is written bare. Composes
/// existing leaf types — no new Rust CaptureType (PLAN-077 W1 constraint).
///
/// The key is `$payload`, NOT `$args`: `{name, args}` is EXACTLY the
/// serializer's template-invocation-map shape (`is_template_invocation_map`,
/// expand.rs), which renders a nested record as a one-element ARRAY
/// (`a.inv[0]` — the dispatch-mode `arm.inv` contract). A variant
/// constructor is NOT a template invocation; sharing the key shape would get
/// every payload-carrying `cons` record misclassified and array-wrapped
/// (PLAN-077 W2 gate-2 root cause). Distinct data, distinct keys.
%capture_type variant_construct {
    $name:ident ( "(" $payload:vc_arg+ ")" )?
}

/// A cond-mode arm guard: `_`, a parenthesized expression, or a bare expression.
/// FAT_ARROW is a first-class ExprExtractor boundary, so `$a && $b => Blue;`
/// self-limits correctly. Choice order is load-bearing: `_` and parenthesized
/// forms MUST precede bare expr because ChoiceExtractor is first-match-wins.
%capture_type cond_guard {
    ( "_" ) | ( "(" $expr:balanced(')') ")" ) | ( $expr:expr )
}

/// A single cond-mode arm: `(guard) => Variant;` or `_ => Variant;`. First
/// truthy guard wins at runtime (cond semantics, first-match-wins). The `;` is
/// REQUIRED (unlike `arm`'s): a payload-less consequence followed by the
/// next arm's parenthesized guard would otherwise gobble the guard as payload
/// args — `Blue ($h) => …` parses `($h)` as `Blue`'s args — so the terminator
/// is the only sound boundary (match_value_arm's discipline; SWARM GATE 1
/// finding). The catch-all `_ => Variant;` keeps it too — uniform, and E0931
/// wants a precisely-locatable final arm.
%capture_type cond_arm {
    $guard:cond_guard "=>" $cons:variant_construct ";"
}

/// A full cond-mode @match block: NO subject, one or more cond arms. The
/// subject token's absence is the grammar-level discriminator vs dispatch-mode
/// `match_block` (same doctrine as @type's product/sum split). `$arms:cond_arm+`
/// is the direct named-repeat shape that threads the arms Array.
%capture_type cond_block {
    "@match" "{" $arms:cond_arm+ "}"
}
