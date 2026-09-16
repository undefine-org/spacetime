// Structured property/field definitions (PLAN-023 W2).
//
// Parses a `;`-separated list of `name: value` declarations, used by:
//   - @type bodies:        `id: string; price: number; name?: string`  (field type refs)
//   - responsive styles:   `position: fixed; background: white`        (CSS declarations)
//
// Each item is one of THREE shapes:
//   - a `name (?)? : value` declaration;
//   - a form SPLICE (`--card-surface;`, `--card-surface(36px);`) — a style form
//     composing another form, which is what lets a design "school" bind a role
//     (`@form style --role-title { --sc-min-title; }`) instead of every pattern
//     naming the school directly (BUG-327);
//   - a nested directive block (`@dark { ... }`), skipped so later declarations
//     are still collected (matching the legacy Rust PropertiesExtractor).
//
// ORDER MATTERS, both ways. The DECLARATION arm comes first because a CSS
// custom property is spelled exactly like a splice plus a colon
// (`--ground: #f8fafc;`): a splice arm tried first matches the `--ground`
// token, ignores the rest of the line, and swallows every declaration after it
// — the same silent truncation, just from the other side. Trying `name ":"
// value` first means a colon decides: with one it is a declaration, without one
// it is a splice.
//
// (The splice arm still has to EXIST, and be tried before `skip_block`: before
// it did, a body containing `--card-surface;` parsed as EMPTY and took its
// sibling declarations with it — BUG-327.)
//
// The reify_properties Rust adapter maps each field record into a
// PropertyDef { name, type_ref, optional }; a splice item carries its name in
// `splice` and is reified into the same shape with an empty value, so the
// consumer (`expand_style_form_splices`) can resolve it recursively.
//
%capture_type properties {
    (
        ( $name:ident $optional:optmark? ":" $value:balanced(';') ";"? )
        | ( $splice:dashed_ident ( "(" $splice_args:call_arg* ")" )? ";"? )
        | $skip:skip_block
    )*
}
