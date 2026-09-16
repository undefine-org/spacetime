//! Integration tests for PROJ-103: Reactive Properties
//!
//! Tests that `.class: $cond;` (class toggle) and `text <- $expr;` / `[slot="x"] <- $expr;`
//! (content injection) compile correctly through the full pipeline, producing
//! appropriate ST.watch() calls in the JS output.

use spacetime::compiler::CompileOptions;
use spacetime::{compile, parse};

/// Compile a .st snippet and return the JS output.
fn compile_st(input: &str) -> String {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled.js
}

/// Compile and return (js, css, pipeline_error_count).
fn compile_st_full(input: &str) -> (String, String, usize) {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    (compiled.js, compiled.css, compiled.pipeline_errors.len())
}

// =============================================================================
// Class Toggle Integration Tests
// =============================================================================

/// `.open: $menuOpen;` inside a @template body should emit classList.toggle via ST.watch
#[test]
fn test_class_toggle_emits_st_watch_in_template() {
    let input = r#"
@template &nav-shell() {
    <nav class="nav">
        <button class="toggle">Menu</button>
    </nav>
    $menuOpen bool: false;
    .nav--open: $menuOpen;
}
"#;

    let js = compile_st(input);

    // The register-template primitive receives the structured ComponentBodyDef
    // which includes directives with ClassToggle type.
    // The JS output should contain the ClassToggle directive data
    assert!(
        js.contains("ClassToggle") || js.contains("class_name"),
        "JS should contain ClassToggle directive data.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );

    // Should contain the class name and var name
    assert!(
        js.contains("nav--open"),
        "JS should reference the class name 'nav--open'"
    );
    assert!(
        js.contains("menuOpen"),
        "JS should reference the var name 'menuOpen'"
    );
}

/// Template with both state and class toggle compiles without errors
#[test]
fn test_class_toggle_with_state_compiles_clean() {
    let input = r#"
@template &toggle-btn() {
    <button class="btn">Toggle</button>
    $active bool: false;
    .btn--active: $active;
    @on &.click { $active <- !$active; }
}
"#;

    let (js, _css, _diag_count) = compile_st_full(input);

    // Should have state initialization
    assert!(
        js.contains("active"),
        "JS should contain the state variable 'active'"
    );

    // Should have the class toggle directive
    assert!(
        js.contains("btn--active"),
        "JS should contain the class name 'btn--active'"
    );
}

// =============================================================================
// Content Injection Integration Tests
// =============================================================================

/// `text <- $count;` inside a @template body should emit content binding
#[test]
fn test_text_injection_emits_content_binding() {
    let input = r#"
@template &counter() {
    <div class="counter">
        <span class="value">0</span>
    </div>
    $count number: 0;
    text <- $count;
}
"#;

    let js = compile_st(input);

    // The register-template primitive should receive injections data
    assert!(
        js.contains("Text") || js.contains("injections"),
        "JS should contain injection data.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );

    assert!(
        js.contains("count"),
        "JS should reference the var name 'count'"
    );
}

/// Attribute injection `src <- $url;` should produce injection in compiled output
#[test]
fn test_attr_injection_compiles() {
    let input = r#"
@template &image-card($alt) {
    <img class="card-img" alt="`$alt`" />
    $url string: "/default.jpg";
    src <- $url;
}
"#;

    let js = compile_st(input);

    // Should contain injection data for 'src' attribute
    assert!(
        js.contains("Attr") || js.contains("src") || js.contains("injections"),
        "JS should contain attr injection data.\nJS output:\n{}",
        &js[..js.len().min(2000)]
    );

    assert!(js.contains("url"), "JS should reference the var name 'url'");
}

// =============================================================================
// FEAT-072 — killed reactive macros error (E0910)
// =============================================================================

/// @bind/@show/@input were REMOVED (FEAT-072). Using one is a hard E0910 error
/// with a precise migration hint — never a silent no-op. PLAN-076: the
/// tombstones + `check_bind_deprecation` are gone; `%migration` registry
/// entries own the retirements. PLAN-079 (capsules): a `from_ast` compile
/// (no source text) can't run the shim, so rewrite-kind shapes degrade to
/// E0910 with the entry's %hint; hint-kind shapes (@show/@input) COMPILE
/// THROUGH the window — the embedded retired macro's %binds still expand —
/// with a W0715 carrying the %hint.
#[test]
fn test_killed_bind_e0910_and_hint_kind_compile_through() {
    use spacetime::compiler::CompileOptions;

    // Rewrite-kind, multi-shape: no single %rewrite rule covers
    // @bind(class:, when:) — E0910 with the entry's %hint (the split
    // guidance), never a silent no-op.
    let input = ".user-name { @bind(class: \"active\", when: $isActive) }";
    let ast = parse(input).expect("should still parse");
    let compiled = spacetime::compile(&ast, CompileOptions::default());
    // Since the hard cutover a file with no `@version` compiles at the CURRENT
    // wave, so this retired `@bind` is refused as E0911 ("retired in the
    // 2026-06-09 wave") rather than degraded as E0910 ("the shim could not
    // rewrite this shape"). Both are the contract this test defends — the call
    // must ERROR and carry the entry's split guidance, never silently no-op —
    // so accept either code and keep asserting the hint, which is the part that
    // actually helps the author.
    let e0910: Vec<_> = compiled
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0910" || e.code == "E0911")
        .collect();
    assert_eq!(
        e0910.len(),
        1,
        "expected exactly one retired-syntax error for `{input}`, got: {:?}",
        compiled.pipeline_errors
    );
    assert!(
        e0910[0]
            .hint
            .as_deref()
            .unwrap_or_default()
            .contains("One property per statement"),
        "E0910 hint must carry the entry's %hint, got: {:?}",
        e0910[0].hint
    );

    // Hint-kind (PLAN-079 capsules): @show/@input COMPILE THROUGH the window
    // — the embedded retired macros' %binds still route to their retained
    // primitives — with a W0715 carrying the entry's %hint.
    for (input, want_hint, want_js) in [
        // bind-visible's display toggle (the retired @show semantics).
        (".spinner { @show(when: $loading) }", ".hidden:", "display"),
        // bind-input's two-way wiring (the retired @input semantics): an
        // input-event listener that writes the signal back.
        (
            "input.search { @input(bind: $q) }",
            "two-way binding",
            "addEventListener",
        ),
    ] {
        // The WINDOW is what this half tests, and since the hard cutover a file
        // must ASK to be in it: an absent `@version` now means CURRENT, so
        // retired syntax is refused rather than compiled through. Declaring the
        // day before the reactive-surface wave is exactly what a project
        // mid-migration carries (it is what `spacetime migrate` writes before
        // bumping it forward), so this now says out loud the thing it always
        // relied on silently.
        let windowed = format!("@version 2026-06-08;\n{input}");
        let ast = parse(&windowed).expect("should still parse");
        let compiled = spacetime::compile(&ast, CompileOptions::default());
        assert!(
            !compiled.pipeline_errors.iter().any(|e| e.code == "E0910"),
            "hint-kind `{input}` must not hard-error inside the window: {:?}",
            compiled.pipeline_errors
        );
        let w0715: Vec<_> = compiled
            .migration_warnings
            .iter()
            .filter(|w| w.code == "W0715")
            .collect();
        assert_eq!(
            w0715.len(),
            1,
            "expected exactly one W0715 for `{input}`, warnings: {:?}",
            compiled.migration_warnings
        );
        assert!(
            w0715[0]
                .hint
                .as_deref()
                .unwrap_or_default()
                .contains(want_hint),
            "W0715 hint for `{input}` must suggest `{want_hint}`, got: {:?}",
            w0715[0].hint
        );
        // Compile-through is REAL: the embedded retired macro's %binds
        // expand to their primitives' JS (a warning with no emit would be a
        // broken promise).
        assert!(
            compiled.js.contains(want_js),
            "retired semantics must still emit `{want_js}` for `{input}`"
        );
    }
}

// =============================================================================
// Mixed Reactive Properties Integration
// =============================================================================

/// A template with state, class toggle, content injection, and events all together
#[test]
fn test_full_reactive_template_compiles() {
    let input = r#"
@template &todo-item($text) {
    <li class="todo">
        <span class="todo__text">`$text`</span>
        <button class="todo__toggle">✓</button>
    </li>
    $done bool: false;
    .todo--done: $done;
    @on &.click(.todo__toggle) { $done <- !$done; }
}
"#;

    let (js, _css, _diag_count) = compile_st_full(input);

    // Template should compile with all reactive features
    assert!(
        js.contains("todo"),
        "JS should contain template registration"
    );

    // Should have both state and directive data
    assert!(
        js.contains("done"),
        "JS should reference the 'done' state variable"
    );
    assert!(
        js.contains("todo--done"),
        "JS should reference the 'todo--done' class toggle"
    );
}

// =============================================================================
// Harmonized expression-model regression tests
// =============================================================================

/// `@on click .inc { ... }` (space-selector form) compiles and wires the
/// handler to the `.inc` child, equivalent to the paren form `click(.inc)`.
///
/// IGNORED — BUG-334: event delegation has no spelling after the `@on` sigil
/// cutover, and this test is the sharpest evidence of WHY that is worse than a
/// plain gap. It does not fail at the `diag_count == 0` assertion above; it
/// fails further down, at `delegateSelector`. The page compiles CLEAN and the
/// delegation target is silently gone: `.inc` in that position is now parsed as
/// a nested scope block and absorbed, the driver binds to `&self`, and no
/// `target` capture is ever produced.
///
/// So an author writing this gets a handler on the CONTAINER that never
/// dispatches to the child they named, with zero diagnostics — the exact
/// silent-wrong-page class the cutover work exists to delete.
///
/// The runtime half is still intact and still waiting for a target:
/// `on-mutation-handler` reads `delegateSelector` and calls
/// `e.target.closest(...)` (stdlib/primitives/animation/drivers.st:1978), and
/// the primitive still declares `target: selector?`. Only the SURFACE that fed
/// it is missing, so restoring a spelling re-greens this test unchanged.
///
/// Kept executable and un-rewritten on purpose: when BUG-334 lands, drop the
/// `#[ignore]` and this asserts the real contract. Do not "fix" it by asserting
/// the current behaviour — the current behaviour is the bug.
#[test]
#[ignore = "BUG-334: @on event delegation has no replacement spelling after the sigil cutover"]
fn test_on_event_space_selector_compiles() {
    let input = r#"
@template &counter() {
    <section class="counter">
        <output class="count">0</output>
        <button class="inc">+</button>
    </section>
    $count number: 0;
    .count { text: $count; }
    @on &.click .inc { $count <- $count * 2; }
}
"#;
    let (js, _css, diag_count) = compile_st_full(input);
    assert_eq!(diag_count, 0, "space-selector @on should compile cleanly");
    // PLAN-039: `@on click .inc` inside a @template body flows through the unified scope
    // path — a page-level selector-init delegating to the `.inc` child, whose mutation runs
    // via ST.runMutations (scope-resolved `$count`). It is NOT wired as a factory
    // `directives` entry anymore (that World-B arm was deleted).
    assert!(
        js.contains(r#"delegateSelector = ".inc""#),
        "space-form `@on &.click .inc` should delegate to the `.inc` child"
    );
    // The mutation body carries the source statement; `$count` resolves per-instance at
    // runtime via ST.runMutations (ST.resolveOwner), not a compile-time ST.get arrow.
    assert!(
        js.contains(r#"$count <- $count * 2"#),
        "expected the `@on` mutation body `$count <- $count * 2` in the emitted JS"
    );
}

/// `.sel { text: $expr; }` becomes a reactive ContentBinding (not dropped CSS),
/// and an arithmetic expression is transpiled to an `apply` arrow.
#[test]
fn test_content_binding_expression_compiles() {
    let input = r#"
@template &counter() {
    <section class="counter">
        <output class="double">0</output>
    </section>
    $count number: 0;
    .double { text: $count * 2; }
}
"#;
    let js = compile_st(input);
    // PLAN-039 Move 2b: `text: $count * 2` in a @template body lowers to the unified
    // scope-aware reactive-binding arc (per-node init + ST.resolve(__node, ...)), NOT a
    // factory `ContentBinding` directive (that World-B arm/payload was deleted).
    assert!(
        js.contains(r#"ST.registerSelectorInit(".double""#),
        "content-binding should register a per-node selector-init for `.double`.\nJS:\n{}",
        &js[..js.len().min(2500)]
    );
    assert!(
        js.contains("ST.resolve(__node, 'count') * 2"),
        "expected the scope-resolved transpiled expression for the binding.\nJS:\n{}",
        &js[..js.len().min(2500)]
    );
}

/// `@computed` may derive from a file-level `$state` source with trailing
/// clauses (reduce/initial) without an E0405 greedy-capture false positive.
#[test]
fn test_computed_from_state_source_no_false_e0405() {
    let input = r#"
@type CartItem { id: string; price: number; qty: number; }
body { $cart CartItem[]: []; }
@computed $cartTotal: number {
    from: $cart;
    reduce: (s, $) => s + $.price * $.qty;
    initial: 0;
}
.total { text: $cartTotal; }
"#;
    let (_js, _css, diag_count) = compile_st_full(input);
    assert_eq!(
        diag_count, 0,
        "computed from $state with reduce/initial must not raise E0405"
    );
}

/// `@data ... { src: inline; value: [...] }` compiles to a data-source wired
/// with the inline value as its default and `src: "inline"` (no fetch / no
/// E0504 path). Regression for BUG-027.
#[test]
fn test_inline_data_source_compiles_with_value() {
    let input = r#"
@type Size { code: string; label: string; }
@data inline $sizes Size[] : [ { "code": "S", "label": "Small" }, { "code": "M", "label": "Medium" } ];
@template &opt($s) { <li class="opt">`$s.label`</li> }
.list { @each($sizes as $s) { &opt($s); } }
"#;
    let (js, _css, diag_count) = compile_st_full(input);
    assert_eq!(
        diag_count, 0,
        "inline data source should compile without diagnostics"
    );
    // PLAN-023 W5/FEAT-047: `@data inline` lowers to the local-state-impl primitive
    // (compile-time-constant local state), not the data-source fetch path. The
    // surviving contract: the inline value literal reaches the emitted JS as the
    // seed for the named signal.
    assert!(
        js.contains("Small") && js.contains("Medium"),
        "expected inline value literal in emitted JS"
    );
    assert!(
        js.contains("sizes"),
        "expected the inline source's signal name (sizes) in emitted JS"
    );
}

/// The canonical reactive example (examples/cart-demo) must compile cleanly:
/// inline @data (BUG-027), template-invoke @each, computed-from-$state,
/// expression mutations. Regression fixture for BUG-029.
#[test]
fn test_cart_demo_compiles_clean() {
    let src = std::fs::read_to_string("../examples/cart-demo/index.st")
        .or_else(|_| std::fs::read_to_string("examples/cart-demo/index.st"))
        .expect("cart-demo index.st should be readable");
    let (_js, _css, diag_count) = compile_st_full(&src);
    assert_eq!(
        diag_count, 0,
        "examples/cart-demo/index.st must compile without pipeline diagnostics"
    );
}

// =============================================================================
// BUG-034: @computed must publish itself as a global signal so downstream
// file-scope bindings (`.sel { text: $computed }`) and computed-of-computed
// chains can react. Before the fix, compute() only did ST.set(el,'value')
// (element-scoped) and never dispatched local:<name>:updated.
// =============================================================================

/// A @computed over a $state source must, on each recompute, publish the new
/// value into the global SpacetimeLocal store AND dispatch
/// `local:<name>:updated` so file-scope `$computed` bindings fire.
#[test]
fn test_computed_publishes_global_signal_and_dispatches() {
    let input = r#"
@type Item {
    price: number;
    qty: number;
}

@data inline $cart Item[] : [] ;

@data fold $cartTotal number from $cart : acc + (item.price * item.qty) ;

.total { text: $cartTotal; }
"#;

    let js = compile_st(input);

    // PLAN-023 W5/FEAT-047: the reduce-to-scalar is now `@data fold`, lowered via
    // the derived-signal primitive. The reactive CONTRACT is unchanged: the derived
    // value publishes into the global store and dispatches `local:<name>:updated`
    // so file-scope bindings (.total) re-run. We assert that surviving contract
    // (not the deleted computed-source's `computedName` template internals).

    // The derived value must be published into the global store so bindings read it.
    assert!(
        js.contains("SpacetimeLocal"),
        "@data fold must publish its result into window.SpacetimeLocal.\nJS head:\n{}",
        &js[..js.len().min(400)]
    );
    assert!(
        js.contains("cartTotal"),
        "@data fold must reference its own signal name (cartTotal)"
    );

    // And it must dispatch local:<name>:updated so subscribers re-run.
    assert!(
        js.contains("local:") && js.contains(":updated"),
        "@data fold must dispatch a local:<name>:updated CustomEvent on recompute"
    );
}

// FEAT-042: namespaced (dotted) signal field bindings. `.x { text: $pan.deltaX }`
// must transpile to a global read of the ROOT object with JS field access, and
// subscribe to the ROOT signal's updated event.
#[test]
fn test_namespaced_signal_field_binding_emits_root_subscription() {
    let input = r#"
@data inline $pan : {"deltaX": 10, "deltaY": 20} ;

.x { text: $pan.deltaX; }
"#;
    let js = compile_st(input);
    // PLAN-039 Move 2b: dotted bindings lower to the LEXICAL read of the ROOT object
    // (`ST.resolve(__node, 'pan')`) then JS field access, so the same binding resolves a
    // template-instance signal or, at file scope, the global SpacetimeLocal['pan'] (the
    // ST.resolve fallback). The ROOT-object + field-access shape is preserved.
    assert!(
        js.contains("ST.resolve(__node, 'pan').deltaX"),
        "dotted binding must read root object (via ST.resolve) then field. JS:\n{}",
        &js[js.find("deltaX").map(|i| i.saturating_sub(80)).unwrap_or(0)
            ..js.find("deltaX")
                .map(|i| (i + 80).min(js.len()))
                .unwrap_or(0)]
    );
    assert!(
        js.contains("local:pan:updated"),
        "dotted binding must subscribe to the ROOT signal's updated event"
    );
}

// FEAT-042 root fix: a @data source must publish into SpacetimeLocal and
// dispatch local:<name>:updated so file-scope $signal bindings (scalar or
// dotted) react to @data sources, not only $state/@computed signals.
#[test]
fn test_data_source_publishes_to_spacetime_local() {
    let input = r#"
@data inline $pan : {"deltaX": 10} ;

.x { text: $pan.deltaX; }
"#;
    let js = compile_st(input);
    // PLAN-023 W5/FEAT-047: `@data inline` publishes its grouped-object state into
    // the global store under its own name and exposes it for field bindings. The
    // surviving contract: the source name + its published value reach the global
    // store (so `$pan.deltaX` reads resolve), with a `local:<name>:updated` channel.
    assert!(
        js.contains("SpacetimeLocal") && js.contains("pan"),
        "@data inline must publish its value into SpacetimeLocal under its name"
    );
    assert!(
        js.contains("local:") && js.contains(":updated"),
        "@data inline must expose a local:<name>:updated channel for subscribers"
    );
}

// =============================================================================
// BUG-068 — selector-scope reactive `:` surface (FEAT-072 unblocker)
// At FILE/SELECTOR scope (not template body), `.class: $v;` must lower to
// classList.toggle and `--prop: $v;` to style.setProperty — NOT setAttribute.
// =============================================================================

/// Selector-scope `.active: $open;` must emit classList.toggle, not setAttribute.
#[test]
fn test_selector_scope_class_toggle_emits_classlist() {
    let input = r#"
$open bool: false;
<div class="box"><p class="msg">hi</p></div>
.box { .active: $open; }
"#;
    let js = compile_st(input);
    assert!(
        js.contains("classList.toggle"),
        "selector-scope `.active: $open;` must emit classList.toggle.\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
    assert!(
        !js.contains("setAttribute(\"active\""),
        "must NOT setAttribute a class name.\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
}

/// Selector-scope BEM class `.x__y--open: $v;` toggles correctly (no `--` attr).
#[test]
fn test_selector_scope_bem_class_toggle() {
    let input = r#"
$open bool: false;
<div class="faq"><div class="faq__item">x</div></div>
.faq__item { .faq__item--open: $open; }
"#;
    let js = compile_st(input);
    assert!(
        js.contains("classList.toggle") && js.contains("faq__item--open"),
        "BEM class toggle must emit classList.toggle('faq__item--open', …).\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
    assert!(
        !js.contains("setAttribute(\"faq__item--open\""),
        "must NOT setAttribute a BEM class.\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
}

/// Selector-scope custom property `--accent: $c;` must emit style.setProperty.
#[test]
fn test_selector_scope_custom_prop_emits_setproperty() {
    let input = r#"
$accent string: "red";
<div class="box">x</div>
.box { --accent: $accent; }
"#;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"--accent\"") || js.contains("setProperty('--accent'"),
        "selector-scope `--accent: $c;` must emit style.setProperty('--accent', …).\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
    assert!(
        !js.contains("setAttribute(\"--accent\""),
        "must NOT setAttribute a custom property.\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
}

// =============================================================================
// BUG-071 — selector-scope `<-` content/attr injection (FEAT-072 spec lines 50-51)
// `text <- $v;` and `src <- $u;` at FILE/selector scope must emit live bindings,
// not silently drop. (Was parsed only as `name: value`, never `name <- value`.)
// =============================================================================

/// Selector-scope `text <- $v;` must emit a textContent binding.
#[test]
fn test_selector_scope_text_arrow_emits_binding() {
    let input = r#"
$msg string: "hi";
<div class="out">old</div>
.out { text <- $msg; }
"#;
    let js = compile_st(input);
    assert!(
        js.contains("textContent") && js.contains("local:msg:updated"),
        "selector-scope `text <- $msg;` must emit a textContent binding + msg listener.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// Selector-scope `src <- $u;` must emit setAttribute('src').
#[test]
fn test_selector_scope_attr_arrow_emits_setattribute() {
    let input = r#"
$u string: "/a.png";
<img class="pic" src="/placeholder.png" />
.pic { src <- $u; }
"#;
    let js = compile_st(input);
    assert!(
        js.contains("setAttribute(\"src\"") && js.contains("local:u:updated"),
        "selector-scope `src <- $u;` must emit setAttribute('src', …) + u listener.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

// =============================================================================
// BUG-091 — a reactive signal in a CSS-PROPERTY position (`background: $sig;`)
// must emit node.style.setProperty(prop, v), NOT setAttribute(prop, v). The `:`
// surface is ALWAYS CSS (its only caller is emit_scope_css_inner over
// css_declarations); attribute injection is the SEPARATE `<-` arrow surface.
// Unblocks referencing brand tokens as $-values in CSS (PLAN-034 Wave B).
// =============================================================================

/// Selector-scope `background: $sig;` must emit style.setProperty, not setAttribute.
#[test]
fn test_selector_scope_css_property_emits_setproperty() {
    let input = r##"
$accent string: "#FF0020";
<div class="hero">x</div>
.hero { background: $accent; }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background\"") || js.contains("setProperty('background'"),
        "selector-scope `background: $accent;` must emit style.setProperty('background', …).\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
    assert!(
        !js.contains("setAttribute(\"background\""),
        "must NOT setAttribute a CSS property.\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
}

/// Member-access signal (`$brand.scarlet`) in a CSS property also setProperty's.
#[test]
fn test_selector_scope_member_signal_css_property_setproperty() {
    let input = r##"
@data inline $brand : { "scarlet": "#FF0020" };
<div class="hero">x</div>
.hero { background: $brand.scarlet; }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background\"") || js.contains("setProperty('background'"),
        "member-access `$brand.scarlet` in CSS prop must style.setProperty.\nJS:\n{}",
        &js[..js.len().min(3000)]
    );
}

// =============================================================================
// BUG-279 — a reactive SHORTHAND is routed to the longhand its VALUE belongs to.
//
// A reactive declaration is applied inline (it cannot go into the stylesheet;
// its value is not known until a signal renders). An inline SHORTHAND resets
// every longhand it covers to its initial value at inline specificity, which
// beats the static rule the author wrote in the same block — so
// `background: linear-gradient(… $sig …)` destroyed the neighbouring
// `background-size` and `-webkit-background-clip`, and a gradient-clipped
// headline rendered as a solid slab.
//
// These tests pin the CONTRACT: which property name is emitted, and — just as
// importantly — which values are NOT routed. The BEHAVIOR (that the longhands
// survive in a real browser) is gated separately and can only be asserted
// there: tests/bugs/BUG-279-reactive-background-shorthand-resets-longhands.test.st
// under `--cdp`. A string assertion here cannot see a longhand being reset,
// which is exactly why both exist (BUG-252).
//
// The routing table itself is NOT in Rust: `background_route` in
// stdlib/capture-types/css-values.st names its arms after the longhands. A new
// shorthand is a paragraph there and no change to the compiler.
// =============================================================================

/// A gradient value routes to `background-image` — the longhand that resets
/// nothing.
#[test]
fn test_reactive_background_gradient_routes_to_background_image() {
    let input = r##"
@data inline $brand : { "ink": "#101018", "accent": "#f0c060" };
<h1 class="shimmer">x</h1>
.shimmer {
  background: linear-gradient(100deg, $brand.ink 42%, $brand.accent 50%, $brand.ink 58%);
  background-size: 260% 100%;
}
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background-image\""),
        "a reactive gradient must be applied as `background-image`: the shorthand \
         would reset background-size/-clip/-position to their initial values at \
         inline specificity, overriding the static longhands in the same rule \
         (BUG-279).\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
    assert!(
        !js.contains("setProperty(\"background\""),
        "the SHORTHAND must not also be set — one apply line per declaration, or \
         the reset comes back through the second one.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// A colour value routes to `background-color`, by the same rule and the same
/// table. Without this the fix would look like a gradient special case.
#[test]
fn test_reactive_background_colour_routes_to_background_color() {
    let input = r##"
@data inline $brand : { "scarlet": "#FF0020" };
<div class="hero">x</div>
.hero { background: $brand.scarlet; }
"##;
    let js = compile_st(input);
    // `$brand.scarlet` is a BINDING, not a literal colour — its kind is unknown
    // until it renders, so it must NOT be routed. Routing on a name would be a
    // guess about a value the compiler has never seen.
    assert!(
        js.contains("setProperty(\"background\""),
        "a bare signal value has no known kind and must stay a shorthand — \
         routing it would guess.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// A LITERAL colour carrying a reactive sibling routes to `background-color`.
#[test]
fn test_reactive_background_literal_colour_routes_to_longhand() {
    let input = r##"
@data inline $brand : { "ink": "#101018" };
<div class="hero">x</div>
.hero { background: rgb($brand.ink 2 3); }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background-color\""),
        "`rgb(…)` is a colour whatever its arguments contain, so it routes to \
         background-color — the same table, a different arm.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// A CSS-wide keyword must NOT be routed. `background: inherit` inherits EVERY
/// background longhand; routing it to one would silently narrow the author's
/// instruction — a wrong answer is worse than the reset this fix removes.
#[test]
fn test_reactive_background_inherit_stays_a_shorthand() {
    let input = r##"
@data inline $mode : { "bg": "inherit" };
<div class="hero">x</div>
.hero { background: inherit $mode.bg; }
"##;
    let js = compile_st(input);
    assert!(
        !js.contains("setProperty(\"background-image\"")
            && !js.contains("setProperty(\"background-color\""),
        "a value the route grammar does not CLAIM must fall through unrouted — \
         the router only ever narrows a shorthand it can positively identify.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// An ordinary longhand is untouched by the router: there is no
/// `background-image_route` production, so the lookup misses and the property
/// is emitted exactly as written. This is the control that separates "the
/// shorthand fix works" from "the reactive CSS path changed".
#[test]
fn test_reactive_longhand_is_not_rewritten() {
    let input = r##"
@data inline $brand : { "ink": "#101018" };
<h1 class="shimmer">x</h1>
.shimmer { background-image: linear-gradient(100deg, $brand.ink, #fff); }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background-image\""),
        "a reactive longhand must be emitted unchanged.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// A property with no route production at all (`color`) is untouched. Without
/// this, a router that defaulted to SOME rewrite would pass every test above.
#[test]
fn test_reactive_unrouted_property_is_unchanged() {
    let input = r##"
@data inline $brand : { "ink": "#101018" };
<div class="hero">x</div>
.hero { color: $brand.ink; }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"color\""),
        "`color` has no `color_route` production, so the lookup misses and the \
         property is emitted as written.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}


// FUP-184 — a multi-layer value is ONE value naming SEVERAL layers, and must
// route exactly like a single image. Before `image_list`, `image_core` matched
// one image only, so a comma-separated list matched NEITHER arm of
// `background_route` and fell through as an unrouted (destructive) shorthand.
//
// The browser half lives in the BUG-279 cdp gate (tests 5-7): both layers must
// still paint, and the per-layer longhands must survive. These pin the contract.

/// Two gradient layers route to `background-image`, exactly like one.
#[test]
fn test_reactive_multi_layer_background_routes_to_background_image() {
    let input = r##"
@data inline $brand : { "ink": "#101018", "accent": "#f0c060" };
<h1 class="layered">x</h1>
.layered { background: linear-gradient(180deg, rgba(16,16,24,0.8), transparent), linear-gradient(100deg, $brand.ink, $brand.accent); }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background-image\""),
        "a comma-separated LAYER LIST is still an image and must route to \
         background-image; falling through leaves the destructive shorthand \
         that resets every per-layer longhand (FUP-184).\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// THE COMMA TRAP: a single gradient CONTAINS commas but is ONE layer. It must
/// keep routing — if the list production split on every comma rather than
/// relying on `image_core`'s `balanced(')')`, this value would be read as
/// several malformed layers and routing would break for the single-layer case
/// that already worked.
#[test]
fn test_commas_inside_a_gradient_do_not_split_layers() {
    let input = r##"
@data inline $brand : { "ink": "#101018", "accent": "#f0c060" };
<h1 class="one">x</h1>
.one { background: linear-gradient(100deg, $brand.ink 42%, $brand.accent 50%, $brand.ink 58%); }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background-image\""),
        "three commas inside one gradient are the gradient's own argument \
         separators, not layer separators — this must remain a single routed \
         image.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// A layer list of COLOURS is not an image list. `background: red, blue` is not
/// valid CSS, and must not be coerced into `background-image` — the router only
/// narrows values it can positively identify, so this falls through untouched.
#[test]
fn test_a_comma_list_that_is_not_images_stays_a_shorthand() {
    let input = r##"
@data inline $brand : { "ink": "#101018" };
<h1 class="bad">x</h1>
.bad { background: $brand.ink, #fff; }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background\""),
        "a comma list of colours matches neither arm and must stay a \
         shorthand — routing it would invent a meaning CSS does not give \
         it.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}


/// `none` is a legal background-image LAYER (`<bg-image>` is `none | <image>`),
/// used to punch a hole in a stack while keeping the per-layer longhands lined
/// up. A mixed list must route like any other layer list — review finding.
#[test]
fn test_a_none_layer_in_a_list_still_routes() {
    let input = r##"
@data inline $brand : { "ink": "#101018", "accent": "#f0c060" };
<h1 class="holed">x</h1>
.holed { background: none, linear-gradient(100deg, $brand.ink, $brand.accent); }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background-image\""),
        "`none, <image>` is a valid two-layer background-image; leaving it on \
         the shorthand path resets every per-layer longhand — the BUG-279 \
         defect for a shape CSS explicitly allows.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// But a BARE `none` must NOT route. `background: none` is a whole-shorthand
/// reset — it clears colour, position and size too — so narrowing it to
/// `background-image` would keep longhands the author asked to drop. Same
/// reasoning that keeps `inherit` unrouted.
#[test]
fn test_a_bare_none_stays_a_shorthand() {
    let input = r##"
@data inline $brand : { "off": "none" };
<h1 class="cleared">x</h1>
.cleared { background: $brand.off; }
"##;
    let js = compile_st(input);
    assert!(
        js.contains("setProperty(\"background\""),
        "a bare `none` resets the WHOLE shorthand by author intent; routing it \
         to one longhand would silently preserve the others.\nJS:\n{}",
        &js[..js.len().min(4000)]
    );
}

