// Dispatch-mode variant DESTRUCTURE pattern (PLAN-077 W1).
//
// A typed-union variant pattern WITH a payload destructure — the `@match $subject`
// arm-pattern shape `Connected { $send, $received }`. The payload-less form
// (`Blue => …`) is NOT this capture type: a bare ident parses as the `lit` branch
// of `match_pat` (StringExtractor's FUP-079 bare-ident acceptance — the SAME
// routing as before PLAN-077), and the runtime dispatch compares it against the
// union value's `type` tag. Keeping bare idents on `lit` preserves the EXACT
// runtime record shape (`{lit: "Blue"}`) that `stdlib/enum/dispatch.st`
// / `view.st` read today — zero consumer changes for every existing `.st` source.
// (Only `_` ever reaches the `wild` branch: StringExtractor refuses a lone `_`.)
//
// WHY the `{ … }` destructure is REQUIRED here (not optional): the %capture_type
// engine's ChoiceExtractor is ordered-choice FIRST-MATCH-WINS with no backtracking
// (`src/syntax/events/extractors/custom.rs` ChoiceExtractor — "Tries alternatives
// in order, returns the first match"). If a bare-ident variant alternative came
// before `wild`, it would COMMIT on `Connected` and the arm would then fail at
// `{`. Requiring the brace group makes this branch self-limiting: it fails on any
// bare ident (no `{` follows), cleanly falling through to `wild`.
//
// WHY pure-stdlib (not reusing Rust's CaptureType::PatternMatch): PatternMatch is
// a HARDCODED BUILT-IN (`src/syntax/events/extractors/pattern.rs`), and a stdlib
// %capture_type production composes only OTHER %capture_type productions or leaf
// types — not built-in parameterized Rust CaptureTypes (`custom.rs::
// capture_type_to_extractor`). This mirrors PatternMatchExtractor's destructure
// shape in stdlib primitives, per the self-describing-metasystem discipline and
// PLAN-077 W1's zero-new-Rust-CaptureType constraint.

/// One binding inside the destructure: `$field` with an OPTIONAL trailing comma
/// absorbed into the unit — the established comma-list idiom (`param_list`,
/// `variant_payload`). Factored into its own capture type so the list repeats
/// under a SINGLE NAMED capture (`$bindings:variant_binding+`) — the only shape
/// the sequence extractor threads as a named Array (a named capture inside an
/// UNNAMED repeat group loses its Array; type-variants.st documents this).
%capture_type variant_binding {
    $b:binding ","?
}

/// A variant pattern with payload destructure: `Connected { $send, $received }`.
/// The constructor name is an `ident`; the brace group is REQUIRED (see header);
/// `$bindings:variant_binding+` rejects a meaningless empty `Variant {}`.
%capture_type variant_pattern {
    $name:ident "{" $bindings:variant_binding+ "}"
}
