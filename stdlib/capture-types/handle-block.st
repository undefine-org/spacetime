// `@handle` consumer-directive capture types (PLAN-038 W1, pillar 4).
//
// `@handle $sig { … }` is the CONSUMER half of a `@data signal`: it RE-OPENS the
// signal's reactive clauses for THIS scope (nearest-scope cascade, CSS-like). A
// signal fire resolves to its nearest enclosing `@handle`; absent one, the def
// defaults apply. `@each : collection-in-space :: @handle : responses-in-time`.
//
//   .quick-add {
//     @handle $add {
//       optimistic { $todos <- $todos.concat({ title: $draft, pending: true }); }
//       receive {
//         Created(todo) => { $todos <- $todos.replace(todo); }
//         Invalid(errs) => { $errors <- errs; }
//       }
//       final { $draft <- ""; }
//     }
//   }
//
// THREE clauses, all optional (a bare `@handle $sig {}` is legal — it just adopts
// the def behavior):
//   optimistic { <stmts> }   — applied immediately on fire (before the reply).
//   receive [to <Sum>]? { <arms> }  — matches over the response VARIANT SUM (NOT
//                                     the wire envelope): each arm is
//                                     `Variant(binding) => { <stmts> }`. The
//                                     binding is typed by the variant's carried
//                                     `as` type (decl↔use, inferred). Optional
//                                     `to <Sum>` restates the through-line.
//   final { <stmts> }        — runs when a terminal arm fires / on cancel/timeout
//                              (DRYs cleanup across terminal outcomes).
//
// This is the SAME matcher family as `@match` (render) and signal-def `receive`
// (decode) — §S1 pillar 3, three host positions. Here the consequence is effect
// STATEMENTS (mutations/emits), reusing the `statements` builtin body capture.

/// A handle-side receive arm: `Variant(binding) => { <stmts> }` or
/// `Variant(binding) => <stmt>;`. The constructor NAME + an optional bound
/// payload name (the carried value), then effect statements. A wildcard arm
/// uses `_` as the constructor.
%capture_type handle_arm {
    $ctor:ident ( "(" $bind:ident ")" )? "=>" ( "{" $body:balanced('}') "}" | $stmt:balanced(';') ";"? )
}

/// The handle-side `receive` clause: matches over the response sum. Optional
/// `to <Sum>` through-line head, then one or more variant arms.
%capture_type handle_receive {
    "receive" ( "to" $sum:ident )? "{" ( $arms:handle_arm )+ "}"
}

/// The `optimistic` clause: statements applied immediately on fire. The inner
/// capture is named `$optimistic` (NOT `$body`) so it lifts to a DISTINCT
/// top-level name when the `( $optimistic:handle_optimistic )?` group is merged
/// — a bare `$body` would collide with `handle_final`'s body (both lift to the
/// same name, and the later clause wins, dropping the optimistic statements).
%capture_type handle_optimistic {
    "optimistic" "{" $optimistic:balanced('}') "}"
}

/// The `final` clause: cleanup statements run on a terminal outcome. Inner
/// capture named `$final` for the same anti-collision reason as `handle_optimistic`.
%capture_type handle_final {
    "final" "{" $final:balanced('}') "}"
}
