// PLAN-150 W8: the body of a `@form transition` — two `&from`/`&to` region
// blocks whose inner keyframes are captured verbatim for placement-time reify.
%capture_type transition_body {
    "&" $fromName:ident "{" $from:balanced('}') "}"
    "&" $toName:ident "{" $to:balanced('}') "}"
}
