// Inline anonymous union type literal (PLAN-077 W1).
//
// An ANONYMOUS (unnamed) enum declared inline at a @data derive's typeref
// position, for one-off unions that don't warrant a named `@type`:
//
//   @data derive $badge (Blue | Orange | Turquoise | Green) : @match {
//     ($isOrange) => Orange;
//     _           => Green;
//   }
//
// Parenthesized to disambiguate from a plain `typeref` ident (forcing parens per
// PLAN-077 Mandate #1's explicit allowance). The `( … )` reuses the grammar-
// combinator sigil meaning already established for `%form`/`%capture_type` groups
// (AGENTS.md sigil-harmony rule), applied here at the AUTHOR level in a typeref
// position. The `|` bar reuses the SAME alternation bar `@type`'s `variant_list`
// and string-enums already use (one sigil, one meaning).
//
// GRAMMAR IS THE DISCRIMINATOR (type-variants.st doctrine): `$tail+` REQUIRES at
// least one `|`, so a parenthesized single ident `(A)` FAILS this production and
// falls through to whatever else owns that position — exactly as `variant_list`
// fails a product-type body and lets `@type` fall through to `properties`.
//
// WHY anonymous: NOT registered in TypeRegistry under a name — valid only at
// this one derive site. If the same union is needed elsewhere, promote it to a
// named `@type` (shareable, gets exhaustiveness checking against a stable name).
// Totality still applies: E0931 requires the derive's cond block to end in a
// `_` catch-all regardless of where the variant set came from.
//
// NULLARY ONLY: an inline union is a bare-tag enum. Payload-carrying variants
// (`Loaded(Todo)`) require a named `@type` — an inline literal has no place to
// declare payload types, and keeping the inline form nullary keeps it a pure
// list of tags (the same shape as a string-enum, but with variant semantics).

/// A `| Variant` alternative tail unit. Factored into its own capture type so
/// the list repeats under a SINGLE NAMED capture (`$tail:inline_variant_alt+`) —
/// the only shape the sequence extractor threads as a named Array (a named
/// capture inside an UNNAMED repeat group loses its Array; type-variants.st
/// established this pattern with `variant_alt`).
%capture_type inline_variant_alt {
    "|" $v:ident
}

/// An inline anonymous union type literal: `(A | B | C)` — a head variant plus
/// ONE OR MORE `| Variant` tails (`+` forces >=1 bar; see header).
%capture_type inline_variant_type {
    "(" $head:ident $tail:inline_variant_alt+ ")"
}
