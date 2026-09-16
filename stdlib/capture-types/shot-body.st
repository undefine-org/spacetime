// PLAN-150 W9: the body of a `@form shot` — a `:markup` slot (a template's
// HTML) and a `:motion` slot (an `@on &.clip` keyframe body). Placing the shot
// on a score instantiates the markup and binds the motion to that window.
//
// Both slots are captured verbatim (`balanced('}')`): the markup is reified as
// a template body, the motion as a keyframe body, at placement time — so the
// shot surface grows with those two grammars, not a second parser here. Lives
// in stdlib/capture-types/ (a capture type in a macros/ file is invisible to a
// %form body — the W8 transition-body lesson).
%capture_type shot_body {
    ":" "markup" "{" $markup:balanced('}') "}"
    ":" "motion" "{" $motion:balanced('}') "}"
}
