//! PLAN-023 W3 / FEAT-046 — unified `@data <kind>` surface dispatch.
//!
//! Enshrines the dispatch fix: the kind-word (`inline`/`fetch`/…) selects the
//! macro at PARSE time (specificity-ranked, literal-aware), and resolve HONORS
//! that selection via `FormMatch.matched_macro` instead of re-guessing from the
//! capture shape. Before the fix, `@data inline $x : v` and `@data derive $x : v`
//! share the capture shape `{name, value}`, so resolve tied and mis-dispatched.

use spacetime::{compile, compiler::CompileOptions, parse};

fn js_for(src: &str) -> String {
    let ast = parse(src).expect("parse");
    compile(&ast, CompileOptions::default()).js
}

/// `@data inline $count : 3` must lower to local-state (writes `_localState`),
/// NOT to a fetch/data-source or a derive/compute path.
#[test]
fn data_inline_lowers_to_local_state() {
    let js = js_for("@data inline $count : 3;\n");
    assert!(
        js.contains("_localState[\"count\"]"),
        "inline @data should initialize local state for `count`; got:\n{}",
        js
    );
    assert!(
        !js.contains("__spacetimeData"),
        "inline @data must NOT emit a remote data-source; got:\n{}",
        js
    );
}

/// A PAREN-LED derive value with a TRAILING method call
/// (`($x || []).filter(…).slice(…)`) must capture the WHOLE expression — not
/// truncate at the method name. The events grammar used to split the leading
/// `(…)` into an ARG_LIST and orphan the trailing call-args, bounding the
/// balanced(';') byte-scan at `.filter`/`.slice` (BUG-077 events-grammar twin,
/// surfaced building the FEAT-092 media library's `$assets` derive). The CST
/// parser had the paren-led guard; this asserts the events grammar agrees.
#[test]
fn data_derive_paren_led_value_keeps_trailing_call() {
    let js = js_for(
        "@data inline $items : [];\n@data derive $shown : ($items || []).filter(x => x > 0).slice(0, 5);\n",
    );
    assert!(
        js.contains(".filter(x => x > 0).slice(0, 5)"),
        "paren-led derive value must keep the trailing .filter(…).slice(…); got:\n{}",
        js
    );
}

/// `@data fetch $api : "/api/x"` must lower to a remote data-source, NOT to
/// local-state.
#[test]
fn data_fetch_lowers_to_data_source() {
    let js = js_for("@data fetch $api : \"/api/x\";\n");
    assert!(
        js.contains("__spacetimeData"),
        "fetch @data should register a data-source; got:\n{}",
        js
    );
    assert!(
        !js.contains("_localState[\"api\"] = "),
        "fetch @data must NOT initialize local state directly; got:\n{}",
        js
    );
}

/// The discriminating property: two kinds with the SAME capture shape must not
/// cross-dispatch. `inline` and `fetch` differ structurally enough, but this
/// asserts the kind-word actually routes — inline never produces fetch output
/// and vice-versa within one file.
#[test]
fn data_kinds_do_not_cross_dispatch() {
    let js = js_for("@data inline $items : 3;\n@data fetch $remote : \"/r\";\n");
    assert!(
        js.contains("_localState[\"items\"]"),
        "inline kind must lower to local-state; got:\n{}",
        js
    );
    assert!(
        js.contains("__spacetimeData"),
        "fetch kind must lower to data-source; got:\n{}",
        js
    );
}

/// The `;` terminator BOUNDS the value: a following scope block must NOT be
/// swallowed into the value expression. (Regression: without the terminator the
/// directive grammar greedily attaches the next `{ ... }` as its body, and the
/// expr extractor captured `7\n.panel { ... }` as the value — caught live in a
/// browser harness, not by emit inspection alone.)
#[test]
fn data_inline_value_is_bounded_by_semicolon() {
    let js = js_for("@data inline $seats : 7;\n\n.panel { text: $seats }\n");
    assert!(
        js.contains("_localState[\"seats\"] = 7"),
        "value must be exactly `7`, not the following block; got:\n{}",
        js
    );
    assert!(
        !js.contains(".panel {"),
        "the scope block must NOT be captured into the value; got:\n{}",
        js
    );
}

/// Typed variant (`<type>` before `:`) parses and still dispatches by kind.
#[test]
fn data_inline_typed_variant_dispatches() {
    let js = js_for("@data inline $seats number : 3;\n");
    assert!(
        js.contains("_localState[\"seats\"]"),
        "typed inline @data should still lower to local-state; got:\n{}",
        js
    );
}

/// `@data derive` lowers to the derived-signal primitive and the MULTI-TOKEN
/// value expression (including call-parens that the directive grammar otherwise
/// splits into ARG_LIST) survives intact. PLAN-133: survival is now STRUCTURAL —
/// the expression is SWC-parsed and lowered at compile time to a positional
/// function over compiler-discovered deps (no raw-string runtime eval).
#[test]
fn data_derive_captures_full_expression() {
    let js = js_for("@data inline $price : 10;\n@data derive $total : $price * 2;\n");
    assert!(
        js.contains("(price) => price * 2"),
        "derive value expr must be captured whole and lowered; got:\n{}",
        js
    );
    assert!(
        js.contains("__stDerive(dn, [\"price\"]"),
        "derive must pass the compiler-discovered deps as data; got:\n{}",
        js
    );
}

/// derive with a call-expression value (`sum($items)`) keeps the parens — the
/// regression that motivated the BalancedExtractor source byte-scan (the grammar
/// splits `(...)` into a separate ARG_LIST node invisible to a token-slice walk).
#[test]
fn data_derive_keeps_call_parens() {
    let js = js_for("@data inline $items : 3;\n@data derive $t : max($items, 0);\n");
    assert!(
        js.contains("(items) => max(items, 0)"),
        "derive must keep call-args parens through the compile-time lowering; got:\n{}",
        js
    );
}

/// Review regression (W3 reviewer wave): a `balanced(';')` value with the
/// trailing `;` OMITTED must NOT byte-scan past the directive into the following
/// sibling directive. The scan is bounded to the token-slice extent, so the
/// capture degrades (stops at the last token) instead of swallowing siblings.
#[test]
fn data_derive_missing_semicolon_does_not_swallow_sibling() {
    let js = js_for("@data derive $total : $price + $qty\n@data inline $x : 5;\n");
    assert!(
        !js.contains("$x") || !js.contains("exprText = \"$price + $qty\\n@data"),
        "derive expr must not absorb the following directive; got:\n{}",
        js
    );
    // The sibling inline must still lower independently.
    assert!(
        js.contains("_localState[\"x\"]"),
        "sibling @data inline must still compile; got:\n{}",
        js
    );
}

/// Review regression (W3 reviewer wave): a fold step that reads ANOTHER signal
/// (beyond acc/item) must watch that signal so it recomputes when it changes.
/// The emitted `watched` set must include the discovered signal deps, not just
/// the explicit fold source.
#[test]
fn data_fold_watches_step_signal_deps() {
    let js = js_for(
        "@data inline $items : 3;\n@data inline $rate : 2;\n@data fold $w number from $items : acc + item * $rate;\n",
    );
    // PLAN-133: the compiler discovers `rate` in the step and the call passes
    // it as data; the prelude's `__stFold` unions [source].concat(deps).
    assert!(
        js.contains("__stFold(dn, foldSource, initialVal, [\"rate\"]"),
        "fold must watch step signal deps, not only the source; got:\n{}",
        js
    );
    assert!(
        js.contains("[source].concat(deps)"),
        "__stFold must union the explicit source with the step's signal deps; got:\n{}",
        js
    );
}

/// `@data fold` lowers to derived-signal with the explicit `from $source` and
/// the reducer step body, with isFold set.
#[test]
fn data_fold_lowers_with_source_and_step() {
    let js = js_for("@data inline $nums : 3;\n@data fold $sum number from $nums : acc + item;\n");
    assert!(
        js.contains("(acc, item) => acc + item"),
        "fold step body must be captured and lowered to a step function; got:\n{}",
        js
    );
    assert!(
        js.contains("foldSource = 'nums'"),
        "fold must bind the explicit source; got:\n{}",
        js
    );
}

/// BUG-042 (FEAT-047 unblock): an `@data inline` whose value is an array of OBJECT
/// literals must capture the WHOLE balanced `[ … ]` as the value (the inner `{` of
/// each object must not be mistaken for the directive body). Both the CST parser and
/// the events directive grammar consume the balanced bracket region as one ARG.
#[test]
fn data_inline_object_literal_array_captures_whole_value() {
    let html = compile(
        &parse(
            "@data inline $todos : [{\"title\": \"Alpha\"}, {\"title\": \"Beta\"}, {\"title\": \"Gamma\"}];\n<ul class=\"todos\"></ul>\n.todos { @each($todos as $t) { <li>`$t.title`</li> } }\n",
        )
        .expect("parse object-literal array"),
        CompileOptions::default(),
    )
    .html;
    for w in ["Alpha", "Beta", "Gamma"] {
        assert!(
            html.contains(&format!(">{}<", w)),
            "missing {w} in SSG unroll:\n{html}"
        );
    }
}

/// Guard: a `[data-x]` attribute selector after a directive name must NOT be swallowed
/// by the object-array balanced-consume path (its lookahead requires an array opener,
/// not an IDENT). The `@on click [data-add]` form must still parse + compile.
#[test]
fn attribute_selector_not_consumed_as_array() {
    let _ = compile(
        &parse("@data inline $f : [\"A\"];\n.grid { @on &.click [data-add] { $f <- []; } }\n")
            .expect("parse attr selector"),
        CompileOptions::default(),
    )
    .js;
}

/// FEAT-047 (namespaced-signals capability): `@data inline` may take a bare OBJECT
/// literal value (`{ "k": v }`). The `{` is the VALUE, not a directive body —
/// disambiguated by a quoted key followed by `:` (`{ STRING COLON …`). A directive
/// body / `@fn` body never starts with `STRING COLON`, so the two never collide.
#[test]
fn data_inline_bare_object_literal_value() {
    let js = js_for(
        "@data inline $pan : { \"deltaX\": 0, \"deltaY\": 1, \"scale\": 100 };\n.v { text: $pan.deltaX; }\n",
    );
    assert!(
        js.contains("deltaX"),
        "object value lost in lowering:\n{js}"
    );
}

/// Guard: an `@fn` body that begins with a bare string (`{ \"Hello \" + $n }`) must
/// stay a BODY, never be mistaken for an object-literal value (no `STRING COLON`).
#[test]
fn fn_body_starting_with_string_is_not_object_value() {
    // Must parse + compile without treating the body as a value capture.
    let _ = js_for("@fn greet($name) : string { \"Hello, \" + $name }\n");
}
/// FEAT-162: `ephemeral` is declarative send metadata, not a board-specific
/// runtime. A live emit selects the bridge's fire-and-forget rail while its
/// unflagged sibling retains the correlated `pushEvent` path.
#[test]
fn live_emit_ephemeral_selects_fire_and_forget_bridge() {
    let js = js_for(
        r#"@host $board : live("MyAppWeb.BoardLive");
@data signal $cursor($x number) to $board {
  send emit "cursor" ephemeral { x: $x }
  receive { _ => $.reply; }
}
"#,
    );
    assert!(
        !js.contains("var ephemeral = null;"),
        "ephemeral send flag must reach signal lowering; got:\n{js}"
    );
    assert!(
        js.contains("bridge.pushEphemeral(target"),
        "ephemeral live emit must use the fire-and-forget bridge; got:\n{js}"
    );
    assert!(
        js.contains(
            "return ephemeral ? doLiveEphemeral(payload) : doLiveRequest(payload, generation)"
        ),
        "live dispatch must select ephemeral transport without changing normal emits; got:\n{js}"
    );
}

#[test]
fn ordinary_live_emit_remains_correlated() {
    let js = js_for(
        r#"@host $board : live("MyAppWeb.BoardLive");
@data signal $move($x number) to $board {
  send emit "move" { x: $x }
  receive { _ => $.reply; }
}
"#,
    );
    assert!(
        js.contains("bridge.pushEvent(target, payload"),
        "ordinary live emits must retain reply-correlated pushEvent; got:\n{js}"
    );
}
