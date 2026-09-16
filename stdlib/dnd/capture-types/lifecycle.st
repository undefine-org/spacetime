// `@drag` lifecycle-arm capture types (PLAN-058 W3) — kinds-as-DATA.
//
//   on-enter $zone { ... }   on-leave $zone { ... }
//   on-drop  $zone { ... }   on-cancel { ... }
//
// The clause NAME is a captured IDENT field; the drag-zones primitive routes by
// it (and ignores any non-`on-*` block, e.g. a `dragging { … }` STATE that the
// shared body also surfaces). Routing-by-data, not a Rust match per clause.
// Each body is balanced effect statements run via ST.runMutations with
// $zone/$payload bound as locals (the @handle receive-arm rail). Sibling of
// `handle_arm`.

/// One lifecycle arm. Two shapes: `clause $zone { body }` (on-enter/on-leave/
/// on-drop carry a zone bind) or `clause { body }` (on-cancel — no zone). The
/// clause is a bare ident; the primitive keeps only `on-*` clauses.
%capture_type drag_lifecycle_arm {
    ( $clause:ident $zone:ident "{" $zbody:balanced('}') "}" )
    | ( $bare:ident "{" $body:balanced('}') "}" )
}

/// The lifecycle clause SET: zero or more arms. Repetition lives INSIDE a
/// capture_type (mirroring `match_block`'s `( $arms:arm )*`); the form body
/// references it as a single optional capture `( $lifecycle:drag_lifecycle )?`.
%capture_type drag_lifecycle {
    ( $arms:drag_lifecycle_arm )*
}
