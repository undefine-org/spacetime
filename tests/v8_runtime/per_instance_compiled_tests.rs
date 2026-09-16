//! Compiled per-instance regression tests (FUP-051) — the PLAN-039 cutover net.
//!
//! The runtime PRIMITIVES (`ST.resolve`/`watchScoped`/scoped `runMutations`) are
//! covered by `scope_resolution_tests.rs`. THESE pin the *compiled* path end to
//! end: a `@template` body → `register-template` → invoke ×N → per-instance state,
//! the exact surface FEAT-115 reshapes when it sources the factory payload
//! (`states`/`exports`/`html`) from the body SCOPE instead of
//! `reify_component_body`. If the unified scope path regresses (a selector leak, a
//! global shadow, a watcher leak, a wrong-instance write), one of these goes red.
//!
//! Fidelity note: test 1 is a full behavioral E2E (compile → boot → click →
//! assert `ST.get` per instance) and is the strongest guard — it exercises the
//! per-instance STATE seed that FEAT-115 moves. Tests 2/3 pin the compiled
//! per-instance PAYLOAD shape (each-source resolves the instance param; the
//! editable bind path is per-instance) because the full headless RENDER of a
//! nested `@each` in a template body and a live `contenteditable` are blocked on
//! BUG-105-P3 / the layout-less backend (fidelity ladder) — those land as
//! `--cdp`/native `.test.st` gates once BUG-105 clears.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");
const TEMPLATES_JS: &str = include_str!("../../public/runtime/templates.js");

fn compile_page(source: &str) -> spacetime::CompiledSpacetime {
    let ast = spacetime::parse(source).expect("test source should parse");
    spacetime::Compiler::from_ast(&ast).compile()
}

/// Boot a compiled page into a fresh V8 + LinkeDOM context: load the runtimes,
/// install the page HTML, eval the bundle, fire DOMContentLoaded (mounts
/// instances). Returns the live context for interaction + assertion.
fn boot(compiled: &spacetime::CompiledSpacetime) -> V8TestContext {
    let mut c = V8TestContext::new();
    c.eval(ST_JS).expect("load st.js");
    c.eval(TEMPLATES_JS).expect("load templates.js");
    c.set_body_html(&compiled.html).expect("install page html");
    c.eval(&compiled.js).expect("eval page bundle");
    c.trigger_dom_ready().expect("DOMContentLoaded");
    c
}

fn num(c: &mut V8TestContext, expr: &str) -> Option<f64> {
    c.eval(expr).ok().and_then(|v| v.as_f64())
}

// =============================================================================
// 1. @on per-instance isolation (compiled + run, full E2E)
//    Two `&counter()` instances; clicking instance A's button must mutate ONLY
//    A's `$count`. This is the per-instance STATE path FEAT-115 reshapes.
// =============================================================================

#[test]
fn on_event_state_is_per_instance_across_two_compiled_instances() {
    let compiled = compile_page(
        r#"
@template &counter() {
  <div class="counter"><button class="inc">+</button><span class="n">0</span></div>
  $count number: 0;
  .inc { @on &.click { $count <- $count + 1; } }
}
<main><div class="a"></div><div class="b"></div></main>
.a { &counter(); }
.b { &counter(); }
"#,
    );
    let mut c = boot(&compiled);

    // Both instances mounted.
    assert_eq!(
        num(&mut c, "document.querySelectorAll('.counter').length"),
        Some(2.0),
        "two &counter() invocations should mount two instances"
    );

    // Click instance A's button twice, instance B's once.
    c.eval(
        "const b = document.querySelectorAll('.counter .inc');\
         b[0].click(); b[0].click(); b[1].click();",
    )
    .expect("dispatch clicks");

    let a = num(
        &mut c,
        "ST.get(document.querySelectorAll('.counter')[0], 'count')",
    );
    let b = num(
        &mut c,
        "ST.get(document.querySelectorAll('.counter')[1], 'count')",
    );

    // The whole point: independent state. A==2, B==1 — NOT shared, NOT global.
    assert_eq!(
        a,
        Some(2.0),
        "instance A's $count must be its own (2 clicks)"
    );
    assert_eq!(
        b,
        Some(1.0),
        "instance B's $count must be its own (1 click)"
    );

    // And nothing leaked to a page-global slot.
    assert_eq!(
        c.eval("typeof SpacetimeLocal !== 'undefined' && SpacetimeLocal.count")
            .ok()
            .and_then(|v| v.as_f64()),
        None,
        "template-body $count must NOT leak to page-global SpacetimeLocal"
    );
}

// =============================================================================
// 2. Nested @each resolves the INSTANCE param (compiled payload shape)
//    The factory must resolve the each source from the template param `$xs`
//    via the scope chain (ST.resolve / watchScoped), NOT a global data channel.
//    Full per-instance ROW render is a BUG-105-P3 headless gap; here we pin the
//    per-instance RESOLUTION wiring that the row render depends on.
// =============================================================================

#[test]
fn nested_each_source_resolves_from_instance_param_not_global() {
    let compiled = compile_page(
        r#"
@template &row($x) { <li class="row">x</li> }
@template &list($xs) {
  <ul class="list"></ul>
  @each($xs as $x) { &row($x); }
}
<main><div class="a"></div></main>
.a { &list([1,2,3]); }
"#,
    );
    let js = &compiled.js;

    // The each wiring must resolve its source through the SCOPE chain so a
    // template-param source (`$xs`, an instance signal) is found per-instance —
    // the BUG-085 fix. A purely-global lookup would be the regression.
    assert!(
        js.contains("watchScoped") || js.contains("ST.resolve"),
        "nested @each must resolve its source via the scope chain \
         (ST.resolve/watchScoped) so a template-param source resolves per-instance"
    );

    // Boot must not throw, and the container mounts per instance (the row render
    // itself is BUG-105-P3; the container + wiring is what we gate here).
    let mut c = boot(&compiled);
    assert_eq!(
        num(&mut c, "document.querySelectorAll('.a .list').length"),
        Some(1.0),
        "the @each container must mount inside the instance"
    );
    assert!(
        c.get_errors().is_empty(),
        "booting a nested-@each template must not throw: {:?}",
        c.get_errors()
    );
}

// =============================================================================
// 3. @editable bind is per-instance (compiled payload shape)
//    Each `&fld($f)` instance must carry an editable bound to ITS OWN `$f.value`
//    via the scope path (the backtick-hole arg unwrap + ST.resolvePath). Two
//    instances must mount independent editable surfaces. Live caret/IME editing
//    is a `--cdp` concern (LinkeDOM has no selection) per the fidelity ladder.
// =============================================================================

#[test]
fn editable_bind_is_per_instance_across_two_compiled_instances() {
    let compiled = compile_page(
        r#"
@import "stdlib/editable"

@template &fld($f) {
  <div class="fld"></div>
  .fld { @editable(bind: `$f.value`); }
}
<main><div class="a"></div><div class="b"></div></main>
.a { &fld({value: "alpha"}); }
.b { &fld({value: "beta"}); }
"#,
    );
    // NB: this test used to assert on the emitted JS TEXT — that it contained
    // `f.value` and did NOT contain the literal backtick hole `` `$f.value` ``.
    // Both assertions were unsound, and the second failed for a reason that has
    // nothing to do with the feature:
    //
    // A page's JS bundles INTO the same file as the runtime and the stdlib, so
    // a text search cannot tell author code from library code. The forbidden
    // string `` `$f.value` `` appears in a stdlib SOURCE COMMENT —
    // stdlib/editable/primitives/editable-surface.st:61 documents the two bind
    // shapes using that exact spelling — so the gate failed on library prose
    // while the lowering was correct. It would equally have PASSED had the
    // lowering broken in any way that avoided those characters.
    //
    // What actually matters is BEHAVIOR: two instances, each bound to its OWN
    // argument. That is asserted below by reading the mounted DOM.

    // Two independent editable hosts mount, one per instance.
    let mut c = boot(&compiled);
    assert_eq!(
        num(&mut c, "document.querySelectorAll('.fld').length"),
        Some(2.0),
        "each &fld() instance must mount its own editable surface"
    );
    assert!(
        c.get_errors().is_empty(),
        "booting per-instance @editable must not throw: {:?}",
        c.get_errors()
    );

    // PER-INSTANCE is the actual claim: instance `.a` was passed
    // `{value: "alpha"}` and `.b` was passed `{value: "beta"}`, so the two
    // surfaces must not resolve to the same value. A shared/global binding —
    // the defect this test exists to catch — would give both the same text.
    // `1` when the two surfaces carry DIFFERENT bound content, `0` when they
    // are the same — i.e. when the bind leaked into shared state.
    let distinct = num(
        &mut c,
        "(function(){\
           var a = document.querySelector('.a .fld');\
           var b = document.querySelector('.b .fld');\
           if (!a || !b) return -1;\
           var av = (a.getAttribute('data-st-bind') || '') + '|' + (a.textContent || '');\
           var bv = (b.getAttribute('data-st-bind') || '') + '|' + (b.textContent || '');\
           return av === bv ? 0 : 1;\
         })()",
    );
    assert_eq!(
        distinct,
        Some(1.0),
        "each instance must bind to ITS OWN `$f.value` (`.a` got \"alpha\", `.b` \
         got \"beta\"). Identical bound content on both surfaces is exactly what \
         a shared, non-per-instance bind looks like; -1 means a surface did not \
         mount at all."
    );
}
// =============================================================================
// 4. Template body HTML is CLEAN — trailing constructs (a `.sel { @on … }` block,
//    a state decl) must NOT leak into the rendered instance DOM. (FEAT-115 S3:
//    html is now sourced from the CST body via span-subtraction, not reify's
//    text-segmentation which leaked construct segments as a literal CSS text node
//    + a spurious display:contents wrapper — a live shipping bug.)
// =============================================================================

#[test]
fn template_body_html_does_not_leak_construct_segments_into_dom() {
    let compiled = compile_page(
        r#"
@template &c() {
  <div class="c"><span class="v">0</span></div>
  $o bool: false;
  .c { @on &.click { $o <- !$o; } }
}
<main><div class="h"></div></main>
.h { &c(); }
"#,
    );
    let mut c = boot(&compiled);
    let inner = c
        .eval("document.querySelector('.h').innerHTML")
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // The instance must render ONLY its HTML — no leaked CSS/directive text, no
    // spurious display:contents wrapper.
    assert!(
        !inner.contains("@on &.click"),
        "the `.c {{ @on … }}` block must NOT render as literal text in the DOM, got: {}",
        inner
    );
    assert!(
        !inner.contains("display:contents") && !inner.contains("display: contents"),
        "clean single-root html must NOT be wrapped in a display:contents shim, got: {}",
        inner
    );
    assert!(
        inner.contains("<div class=\"c\">") && inner.contains("<span class=\"v\">"),
        "the actual template HTML must render, got: {}",
        inner
    );
}

// =============================================================================
// 5. Multi-root template body (adjacent sibling elements, no wrapper) renders
//    ALL roots. The CST mis-splits adjacent siblings (drops a `<dd ...>` open tag
//    into trivia); span-subtraction over the verbatim body source recovers it.
// =============================================================================

#[test]
fn multi_root_template_body_renders_all_sibling_roots() {
    let compiled = compile_page(
        r#"
@template &row($name, $val) {
  <dt class="k">`$name`</dt><dd class="v">`$val`</dd>
}
<main><div class="host"></div></main>
.host { &row("Age", "30"); }
"#,
    );
    let mut c = boot(&compiled);
    assert_eq!(
        c.query_text(".host .k").as_deref(),
        Some("Age"),
        "first sibling root (<dt>) must render its hole"
    );
    assert_eq!(
        c.query_text(".host .v").as_deref(),
        Some("30"),
        "second sibling root (<dd>) must render — its open tag survives CST trivia loss"
    );
}


