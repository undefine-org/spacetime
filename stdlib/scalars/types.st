// The scalar type table — one row per scalar the language knows.
//
// FEAT-168 / PLAN-122 W4. A scalar answers the same four questions wherever it
// appears, and until now each answer lived in a different Rust `match`:
//
//   is `color` a type at all?      type_system.rs:1012   (the recognition list)
//   what JSON schema does it get?  type_system.rs:791    (primitive_to_schema)
//   what is its zero value?        type_system.rs:533    (zero_value_guarded)
//   which admin control edits it?  server.rs:6387        (widget_for)
//
// Four lists, keyed by the same strings, kept in sync by hand. Nothing enforces
// the agreement, so a scalar added to three of them and forgotten in the fourth
// is not a build error — it is a field that silently renders as a bare text box,
// or a type that silently degrades to `Reference` and resolves to nothing.
//
// This file is the single source. A row here IS the scalar: adding one should
// require no Rust change at all, which is precisely the acceptance test for this
// wave. If a new scalar still needs a Rust edit, the table is not the source of
// truth and the wave has not landed, however green the suite is.
//
// RELATIONSHIP TO css-values.st: that file says what a scalar's syntax IS (the
// grammar that accepts `#e8eef7` and refuses `#e8ee1`). This file says what a
// scalar MEANS to everything downstream of parsing — schema, storage, editing.
// The `%capture` row below is the join between them: it names the production in
// css-values.st that validates this scalar's literals.

/// A CSS colour. Serializes as its source string (`"#e8eef7"`, `"oklch(...)"`),
/// never as a parsed struct — see PLAN-122's representation decision.
%scalar_type color {
  %capture "color"
  %schema "string"
  %format "color"
  %zero ""
  %widget "color"
  %docs "A CSS colour: hex, colour function, or keyword."
}

/// A CSS length. `%` is included by its grammar, because a percentage is legal
/// wherever a length is and `score_measure` composes the two.
%scalar_type length {
  %capture "length"
  %schema "string"
  %format "length"
  %zero ""
  %widget "range-length"
  %docs "A CSS length: a number with a unit, or a percentage."
}

/// A duration. Note the widget differs from `length` even though both are
/// strings with a number and a unit — which is exactly why `%widget` is a
/// column here rather than being derived from `%schema`.
%scalar_type duration {
  %capture "duration"
  %schema "string"
  %format "duration"
  %zero ""
  %widget "range-duration"
  %docs "A duration: a number with a time unit (ms, s, m, h)."
}

/// A URL. Predates the CSS scalars and is included because it is the same KIND
/// of thing — a string with a format and a specialised control. Leaving it in
/// Rust while its neighbours moved would keep the split this table closes.
%scalar_type url {
  %capture "string"
  %schema "string"
  %format "uri"
  %zero ""
  %widget "media"
  %docs "A URL. Rendered by the media picker."
  %loses_to "color easing"
}

/// Rich text: a string at runtime, flagged for the constrained-AST block editor
/// (FUP-029). Its `%format` is empty because the admin dispatches on the
/// `x-st-richtext` marker rather than on a format string — recorded here so the
/// exception is visible in the table instead of hidden in a Rust branch.
%scalar_type richtext {
  %capture "string"
  %schema "string"
  %format ""
  %zero ""
  %widget "richtext"
  %docs "Rich text edited as a constrained block AST."
  %loses_to "color easing"
}

// ---------------------------------------------------------------------------
// The base scalars
// ---------------------------------------------------------------------------
//
// These carry no `%format` and no domain grammar: they ARE the JSON primitives.
// They belong in the table anyway, because the recognition list they came from
// did not distinguish them — `"string" | "number" | "boolean" | "url" | ...` was
// one match arm. A table that held only the interesting half would leave that
// arm in Rust and defeat the point.

%scalar_type date {
  %capture "date"
  %schema "string"
  %format "date"
  %zero ""
  %widget "date"
  %docs "An ISO date, serialized as a string."
}
%scalar_type string {
  %capture "string"
  %schema "string"
  %format ""
  %zero ""
  %widget "text"
  %docs "Text."
  %loses_to "color easing length duration time angle percentage number"
}

/// The one scalar whose zero is not the empty string.
%scalar_type number {
  %capture "number"
  %schema "number"
  %format ""
  %zero "0"
  %widget "number"
  %docs "A number."
}

%scalar_type boolean {
  %capture "bool"
  %schema "boolean"
  %format ""
  %zero "false"
  %widget "toggle"
  %docs "True or false."
}

/// An ISO date, stored as a string. Added as the wave's live proof that a
/// `%scalar_type` row IS the scalar: recognition, JSON schema, zero value, and
/// admin widget all read from this one row with no Rust change.

/// `time` is `duration` under its older name — the spelling most of stdlib
/// still uses. It is a ROW rather than a Rust alias because the table is what
/// answers "is this a scalar?", and a name that resolves to a grammar but has
/// no row is exactly the silent gap this wave exists to close: it parses, then
/// flattens differently from its twin.
%scalar_type time {
  %capture "time"
  %schema "string"
  %format "duration"
  %zero ""
  %widget "range-duration"
  %docs "A duration, under its older name."
  %loses_to "duration"
}

/// An angle. Carried by transforms and gradients.
%scalar_type angle {
  %capture "angle"
  %schema "string"
  %format "angle"
  %zero ""
  %widget "text"
  %docs "An angle: a number with deg, rad, or turn."
}

/// A percentage. Distinct from `length` even though a length ACCEPTS one —
/// `50%` is legal as a length, but a field declared `percentage` must not
/// accept `8px`.
%scalar_type percentage {
  %capture "percentage"
  %schema "string"
  %format "percentage"
  %zero ""
  %widget "range-percentage"
  %docs "A percentage: a number followed by %."
}

/// An easing curve: a keyword (`ease-in-out`) or a function (`cubic-bezier(...)`).
%scalar_type easing {
  %capture "easing"
  %schema "string"
  %format "easing"
  %zero ""
  %widget "text"
  %docs "An easing curve: a keyword or a timing function."
}
