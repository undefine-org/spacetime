// `receive` decode-matcher capture types (PLAN-038 W1, pillar 3).
//
// The `receive` block inside a `@data signal`/`@data stream` DECODES the wire
// envelope `$.` into a tagged-sum variant. It reuses the SAME pattern-matcher
// shape as `@match` (one matcher, three host positions — §S1 pillar 3), but each
// arm's CONSEQUENCE is a VARIANT CONSTRUCTOR (decode), not a &template (render).
//
//   receive to AddResult {
//     2xx => Created($.body as Todo);      // status-class pattern → construct Created
//     422 => Invalid($.body as Errors);    // exact status → construct Invalid
//     _   => Failed($.statusText);         // wildcard fallback
//   }
//
// GRAMMAR PIECES
//   recv_pat   — what an arm matches against the envelope discriminator:
//                an exact number (`422`), a bare ident status-class/tag (`ok`,
//                `2xx` parses as ident; `_` wildcard), or a string (ws message
//                `type`). Mirrors match_pat + numbers.
//   recv_cons  — the arm consequence: `Variant(expr)` (construct a sum variant
//                carrying an expression) or a bare `expr` (auto-named / passthrough).
//   recv_arm   — `<pat> => <cons> [final] ;`  (`final` marks a TERMINAL outcome).
//   recv_head  — the optional `(<alias>)? to <Sum>` head before `{ arms }`
//                (the `to <Sum>` through-line; alias left of `to` names the
//                envelope, sum right of `to` names the response type). Position
//                decides role; a lone name is FORBIDDEN (§S1 POSITION DECIDES ROLE).
//   receive_block — `receive <head>? { <arms>+ }`.
//
// Reified downstream into the signal's decode table + the response @type
// (variant names + carried `as` types register a scoped sum, exactly like a
// `@type Name { … }` sum — one type-creation mechanism, two scopes, §S1).

/// An arm pattern: exact status number, bare ident (status-class `2xx`, tag, or
/// `_` wildcard), or a string literal (ws message type). Bounded leaves before
/// the `=>`.
%capture_type recv_pat {
    ( $num:number ) | ( $name:ident ) | ( $lit:string )
}

/// A variant-construct consequence: `Created($.body as Todo)` — a constructor
/// NAME with a parenthesised payload expression. The payload is a balanced run
/// up to the closing paren (carries `$.body as Todo`, `normalize($.x) as R`, etc).
%capture_type recv_variant {
    $ctor:ident "(" $payload:balanced(')') ")"
}

/// An arm consequence: either a variant constructor (`Created(expr)`) or a bare
/// expression (`$.statusText` — auto-named / passthrough outcome).
%capture_type recv_cons {
    ( $variant:recv_variant ) | ( $expr:balanced(';') )
}

/// A single decode arm: `<pat> => <cons> [final] ;`. The optional `final`
/// keyword marks a TERMINAL outcome (ends a stream; triggers @handle final{}).
%capture_type recv_arm {
    $pat:recv_pat "=>" $cons:recv_cons $final:recv_final? ";"?
}

/// Terminal-outcome marker: the bare `final` keyword after an arm consequence.
%capture_type recv_final {
    "final"
}

/// A full `receive` block: the keyword, an optional head, then arms. The head is
/// an inline Choice (§S1 POSITION DECIDES ROLE), tried left-to-right:
///   `to <Sum>`            — response sum only (the through-line)
///   `<alias> to <Sum>`    — envelope alias (left of `to`) + response sum (right)
///   `<alias>`             — envelope alias only (self-doc, e.g. `Response.status`)
/// The literal `to` branch is tried FIRST so `receive to AddResult` binds
/// `sum=AddResult` (NOT `alias=to`) — a bare `$alias:ident` would otherwise
/// greedily swallow the `to` keyword. The alias names the wire envelope; the sum
/// names the decoded response type. Both optional.
%capture_type receive_block {
    "receive" ( ( "to" $sum:ident ) | ( $alias:ident "to" $sum:ident ) | ( $alias:ident ) )? "{" ( $arms:recv_arm )+ "}"
}

// =============================================================================
// `send` request builder + `policy` lifecycle (the other @data signal clauses)
// =============================================================================

/// The `send` clause builds the outbound request. Two shapes by transport:
///   http:  `send POST "/api/todos" { title: $text }`   (METHOD url { body })
///   ws:    `send emit "proposal" { options: $opts }`     (emit type { payload })
/// `verb` is the HTTP method (`POST`/`PUT`/`PATCH`/`GET`/`DELETE`) OR the literal
/// `emit` (ws/live). `target` is the url/message-type string. `ephemeral` marks a
/// live emit as lossy, fire-and-forget presence-like state; it carries no reply
/// correlation. The optional `{ body }` is a balanced object run.
%capture_type ephemeral_send {
    "ephemeral"
}

%capture_type send_clause {
    "send" $verb:ident $target:string ( $ephemeral:ephemeral_send )? ( "{" $body:balanced('}') "}" )?
}

/// The `policy` clause owns races/lifecycle. `mode` is one of
/// latest|queue|parallel|drop; optional `timeout <dur>` and `retry <n>` tails.
/// (Bare keyword form: `policy latest`.)
%capture_type policy_clause {
    "policy" $mode:ident ( "timeout" $timeout:duration )? ( "retry" $retry:number )?
}
