// Sum-type variant list capture (PLAN-038 W0 — the keystone for @type sums).
//
// Grammar for a TAGGED SUM body inside `@type Name { … }`:
//
//   @type AddResult {
//     Created(Todo)
//     | Partial(Report)
//     | Invalid(Errors)
//     | Failed                       // payload-less variant
//   }
//
// A variant is a Constructor NAME with an OPTIONAL parenthesised payload — a
// comma-separated list of TYPE references (`Created(Todo)`, `Pair(Todo, Report)`,
// or bare `Failed`). Variants are separated by the top-level `|` sigil, reusing
// the SAME union-bar that `"a" | "b"` string-enums already use (one sigil, one
// meaning — the alternation bar).
//
// DISCRIMINATION (why this never collides with the product `@type` body): a
// product body is a `properties` list — `name: Type;` declarations keyed by the
// VALUE colon. A sum body has NO colons and ≥1 top-level `|`. The `@type` macro
// tries the variant group FIRST as an optional leading body-group; `variant_list`
// REQUIRES a leading constructor followed by at least one `| Variant` (the `+`
// repeat), so a product body (no `|`) fails the group and falls through to the
// `properties` capture untouched. No specificity tie, no Rust discriminator — the
// grammar itself is the discriminator (self-describing metasystem rule).
//
// Reified by `reify_type_variants` (src/syntax/events/extractors/custom.rs) into
// an Array of Named records `{ name, payload }` where `payload` is the raw
// comma-list type text (empty for a payload-less variant). The type registry
// (`TypeRegistry::from_form_matches`) reads the `variants` capture into
// `TypeDef.variants`, marking the type a SUM.

/// A single payload type reference inside a variant's parens. A `typeref` leaf
/// (`Todo`, `Todo[]`, `Errors`), comma-separated. Kept as a bounded leaf so the
/// list stops at `,` or the closing `)`.
%capture_type variant_payload {
    ( $ty:typeref ","? )*
}

/// A single sum variant: `Created(Todo)`, `Pair(A, B)`, or bare `Failed`.
/// The constructor name is an `ident`; the payload parens are OPTIONAL (a
/// payload-less variant is a nullary constructor / enum tag).
%capture_type variant_def {
    $name:ident ( "(" $payload:variant_payload ")" )?
}

/// A `| variant` alternative tail unit. Factored into its own capture type so the
/// list can repeat it under a SINGLE NAMED capture (`$tail:variant_alt+`) — the
/// only shape the sequence extractor threads as a named Array (a named capture
/// inside an UNNAMED repeat group loses its Array; mirrors match_block's
/// `$arms:arm+`). The leading `|` is the alternation bar.
%capture_type variant_alt {
    "|" $v:variant_def
}

/// A full sum body: a leading variant `$head` followed by ONE OR MORE `| variant`
/// tails (`$tail:variant_alt+`). The `+` forces ≥1 alternation bar, so a
/// product-type body (zero bars) cleanly fails this group and the `@type` macro
/// falls through to its `properties` field body — the grammar IS the discriminator.
%capture_type variant_list {
    $head:variant_def $tail:variant_alt+
}
