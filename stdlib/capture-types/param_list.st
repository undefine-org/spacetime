// Template parameter list capture type
// Parses: $title, &content, $footer?, $href url, $alt string = ""
//
// Each item is a value binding (`$name`) or element ref (`&name`), with an
// OPTIONAL type annotation (SPACE form, matching signal declarations like
// `$count number: 0` — the type sits between the name and any value/default),
// an OPTIONAL `?` marker, and an OPTIONAL `= default`.
//
// Example matches:
//   $title                    -> [{ name: "title", kind: "binding", optional: false }]
//   $a, &b, $c?               -> [binding a, element b, optional binding c]
//   $href url                 -> [{ name: "href", kind: "binding", type: "url" }]
//   $alt string = ""          -> [{ name: "alt", type: "string", default: "\"\"" }]
//   $size "S" | "M" | "L"      -> [{ name: "size", type: '"S" | "M" | "L"' }]  (FUP-043)
//   ()                        -> [] (empty list allowed)
//
// The type annotation is SPACE-separated (`$href url`), NOT colon (`$href: url`):
// author-level `:` introduces a VALUE everywhere else (signal decls, @data), so
// reusing it for types here would overload the sigil. @fn's `name: type` form is
// the divergent surface (FUP-040). The optional `= default` mirrors @fn defaults.
//
%capture_type param_list {
    ( ( $binding:binding | $element:element ) $collection:collmark? $ptype:typeref? $optional:optmark? ( "=" $default:expr )? ","? )*
}

// Collection marker: a trailing `[]` after an element param (`&items[]`) marks a
// BLOCK region — a repeating editable sub-space holding child blocks (FUP-042),
// distinct from a bare `&caption` inline region. Reuses the `[ ]` collection
// sigil already used for `&name[]` selector refs in component bodies.
%capture_type collmark {
    "[" "]"
}

// Optional-marker terminal: matches a single "?" (PLAN-023 W2). Lets param_list capture
// whether a parameter was marked optional, under the `optional` record key.
%capture_type optmark {
    "?"
}
