//! Scope-resolution primitive tests (PLAN-039 Move 1/2).
//!
//! These pin the runtime mechanism that makes nested constructs (`@on`, `@each`,
//! `@editable`, class-toggles, content-bindings) inside a `@template` body resolve
//! their `$signals` against the INSTANCE scope rather than global page state:
//!
//!   ST.resolve(el, name)        lexical read: walk el + ancestors, then SpacetimeLocal
//!   ST.resolveOwner(el, name)   the nearest ancestor whose OWN store holds a value
//!   ST.resolvePath(el, "a.b")   resolve the head in scope, traverse the dotted tail
//!   ST.watchScoped(el, name,fn) watch the name across el + ancestors (fires immediately)
//!
//! The DOM parent chain IS the signal scope chain; an inner node sees an outer
//! node's signals (lexical scope), an instance shadows the page, and a bare
//! watcher slot created up-chain never shadows a real value owned further out.
//!
//! `ST.get` stays ELEMENT-LOCAL by contract — these tests also pin that boundary
//! (get does NOT walk; resolve does), so the two never blur back together.

use super::context::V8TestContext;

fn ctx() -> V8TestContext {
    let mut c = V8TestContext::new();
    c.set_body_html(
        "<div class=\"outer\"><div class=\"mid\"><div class=\"inner\"></div></div></div>",
    )
    .expect("set body");
    c.eval(include_str!("../../public/runtime/st.js"))
        .expect("load st.js");
    c
}

/// Evaluate a JS boolean expression and assert it is `true`.
fn assert_true(c: &mut V8TestContext, expr: &str) {
    match c.eval(expr) {
        Ok(v) => assert_eq!(
            v.as_bool(),
            Some(true),
            "expected `{}` to be true, got {:?}",
            expr,
            v
        ),
        Err(e) => panic!("eval `{}` failed: {:?}", expr, e),
    }
}

// =============================================================================
// ST.resolve — lexical read up the scope chain
// =============================================================================

#[test]
fn resolve_reads_own_signal() {
    let mut c = ctx();
    c.eval("const inner = document.querySelector('.inner'); ST.set(inner, 'x', 42);")
        .unwrap();
    assert_true(&mut c, "ST.resolve(inner, 'x') === 42");
}

#[test]
fn resolve_walks_to_ancestor_signal() {
    let mut c = ctx();
    // Signal lives on the OUTER node; the inner node resolves it lexically.
    c.eval("const outer = document.querySelector('.outer'); const inner = document.querySelector('.inner'); ST.set(outer, 'theme', 'dark');")
        .unwrap();
    assert_true(&mut c, "ST.resolve(inner, 'theme') === 'dark'");
}

#[test]
fn resolve_nearest_scope_shadows_outer() {
    let mut c = ctx();
    // Both outer and inner own `v`; the inner (nearest) value wins.
    c.eval("const outer = document.querySelector('.outer'); const inner = document.querySelector('.inner'); ST.set(outer, 'v', 'page'); ST.set(inner, 'v', 'instance');")
        .unwrap();
    assert_true(&mut c, "ST.resolve(inner, 'v') === 'instance'");
    // From the mid node (between them) the outer value is what is visible.
    assert_true(
        &mut c,
        "ST.resolve(document.querySelector('.mid'), 'v') === 'page'",
    );
}

#[test]
fn resolve_unbound_is_undefined() {
    let mut c = ctx();
    assert_true(
        &mut c,
        "ST.resolve(document.querySelector('.inner'), 'nope') === undefined",
    );
}

#[test]
fn resolve_falls_back_to_spacetime_local() {
    let mut c = ctx();
    // No element scope owns `g`; it is page-global state.
    c.eval("window._localState = { g: 7 }; window.SpacetimeLocal = window._localState;")
        .unwrap();
    assert_true(
        &mut c,
        "ST.resolve(document.querySelector('.inner'), 'g') === 7",
    );
}

#[test]
fn resolve_skips_bare_watcher_slots() {
    // The crux invariant: ST.watch creates a `{v:undefined}` slot on a node; that
    // placeholder must NOT shadow a real value owned by an ancestor. (watchScoped
    // creates such slots up the chain.)
    let mut c = ctx();
    c.eval("const outer = document.querySelector('.outer'); const inner = document.querySelector('.inner'); ST.set(outer, 'd', 'real'); ST.watch(inner, 'd', () => {});")
        .unwrap();
    // inner now has a bare slot for `d`; resolution must still find outer's value.
    assert_true(&mut c, "ST.resolve(inner, 'd') === 'real'");
}

// =============================================================================
// ST.get vs ST.resolve — the element-local / lexical boundary (one job each)
// =============================================================================

#[test]
fn get_is_element_local_resolve_is_lexical() {
    let mut c = ctx();
    c.eval("const outer = document.querySelector('.outer'); const inner = document.querySelector('.inner'); ST.set(outer, 'a', 1);")
        .unwrap();
    // get does NOT walk: the inner node has no own `a`.
    assert_true(&mut c, "ST.get(inner, 'a') === undefined");
    // resolve DOES walk: it finds the ancestor's `a`.
    assert_true(&mut c, "ST.resolve(inner, 'a') === 1");
}

// =============================================================================
// ST.resolveOwner — the write target for a scoped mutation
// =============================================================================

#[test]
fn resolve_owner_is_the_value_holding_node() {
    let mut c = ctx();
    c.eval("const outer = document.querySelector('.outer'); const inner = document.querySelector('.inner'); ST.set(outer, 'o', true);")
        .unwrap();
    assert_true(
        &mut c,
        "ST.resolveOwner(inner, 'o') === document.querySelector('.outer')",
    );
}

#[test]
fn resolve_owner_null_when_global() {
    let mut c = ctx();
    c.eval("window._localState = { g: 1 }; window.SpacetimeLocal = window._localState;")
        .unwrap();
    // No element owns it -> null (caller writes through the global path).
    assert_true(
        &mut c,
        "ST.resolveOwner(document.querySelector('.inner'), 'g') === null",
    );
}

#[test]
fn resolve_owner_skips_bare_watcher_slot() {
    let mut c = ctx();
    c.eval("const outer = document.querySelector('.outer'); const inner = document.querySelector('.inner'); ST.set(outer, 'w', 5); ST.watch(inner, 'w', () => {});")
        .unwrap();
    assert_true(
        &mut c,
        "ST.resolveOwner(inner, 'w') === document.querySelector('.outer')",
    );
}

// =============================================================================
// ST.resolvePath — dotted path over a scoped head
// =============================================================================

#[test]
fn resolve_path_traverses_dotted_tail() {
    let mut c = ctx();
    c.eval("const outer = document.querySelector('.outer'); ST.set(outer, 'doc', { value: { type: 'doc' } });")
        .unwrap();
    assert_true(
        &mut c,
        "ST.resolvePath(document.querySelector('.inner'), 'doc.value.type') === 'doc'",
    );
}

#[test]
fn resolve_path_bare_head_equals_resolve() {
    let mut c = ctx();
    c.eval("const inner = document.querySelector('.inner'); ST.set(inner, 'n', 9);")
        .unwrap();
    assert_true(&mut c, "ST.resolvePath(inner, 'n') === 9");
}

#[test]
fn resolve_path_undefined_when_segment_missing() {
    let mut c = ctx();
    c.eval(
        "const outer = document.querySelector('.outer'); ST.set(outer, 'doc', { value: null });",
    )
    .unwrap();
    // Traversal stops at the null segment and yields a nullish result (null here,
    // since `.value` is null) — never throws on the `.type` access past it.
    assert_true(
        &mut c,
        "ST.resolvePath(document.querySelector('.inner'), 'doc.value.type') == null",
    );
}

// =============================================================================
// ST.watchScoped — watch a name across the scope chain
// =============================================================================

#[test]
fn watch_scoped_fires_immediately_when_present() {
    let mut c = ctx();
    c.eval(
        r#"
        const outer = document.querySelector('.outer');
        const inner = document.querySelector('.inner');
        ST.set(outer, 's', 'ready');
        globalThis.__seen = null;
        ST.watchScoped(inner, 's', (v) => { globalThis.__seen = v; });
        "#,
    )
    .unwrap();
    // ST.watch invokes the callback immediately for an already-present value.
    assert_true(&mut c, "globalThis.__seen === 'ready'");
}

#[test]
fn watch_scoped_fires_on_later_set_anywhere_in_scope() {
    let mut c = ctx();
    c.eval(
        r#"
        const outer = document.querySelector('.outer');
        const inner = document.querySelector('.inner');
        globalThis.__count = 0;
        globalThis.__last = null;
        ST.watchScoped(inner, 'late', (v) => { globalThis.__count++; globalThis.__last = v; });
        // The value lands on an ANCESTOR after the watch was attached (the
        // selector-init / factory-seed race). watchScoped must still catch it.
        ST.set(outer, 'late', 'arrived');
        "#,
    )
    .unwrap();
    assert_true(&mut c, "globalThis.__last === 'arrived'");
}

// =============================================================================
// Scope-aware mutations (ST.runMutations) — the @on-in-template path
// =============================================================================

/// `$open <- !$open` fired against an inner node whose enclosing scope owns
/// `open` must mutate THAT instance signal (read + write both scope-resolved),
/// not a phantom global.
#[test]
fn run_mutations_writes_instance_signal() {
    let mut c = ctx();
    c.eval(
        r#"
        const outer = document.querySelector('.outer');   // the "instance root"
        const inner = document.querySelector('.inner');   // the @on target
        ST.set(outer, 'open', false);
        ST.runMutations(inner, { js_statements: ["$open <- !$open"] });
        "#,
    )
    .unwrap();
    // Written to the instance (outer), not SpacetimeLocal.
    assert_true(
        &mut c,
        "ST.get(document.querySelector('.outer'), 'open') === true",
    );
    assert_true(
        &mut c,
        "typeof SpacetimeLocal === 'undefined' || SpacetimeLocal.open === undefined",
    );
}

/// Two sibling instances each own their `open`; mutating one leaves the other
/// untouched (per-instance isolation — the whole point of scope resolution).
#[test]
fn run_mutations_is_per_instance() {
    let mut c = V8TestContext::new();
    c.set_body_html(
        "<div class=\"a inst\"><span class=\"hit\"></span></div><div class=\"b inst\"><span class=\"hit\"></span></div>",
    )
    .expect("set body");
    c.eval(include_str!("../../public/runtime/st.js"))
        .expect("load st.js");
    c.eval(
        r#"
        const a = document.querySelector('.a');
        const b = document.querySelector('.b');
        ST.set(a, 'open', false);
        ST.set(b, 'open', false);
        // Fire the mutation from inside instance A only.
        ST.runMutations(a.querySelector('.hit'), { js_statements: ["$open <- !$open"] });
        "#,
    )
    .unwrap();
    assert_true(
        &mut c,
        "ST.get(document.querySelector('.a'), 'open') === true",
    );
    assert_true(
        &mut c,
        "ST.get(document.querySelector('.b'), 'open') === false",
    );
}

/// A file-scope mutation (no element owns the signal) still falls through to the
/// global SpacetimeLocal path — the scope-aware change must not regress it.
#[test]
fn run_mutations_global_fallback_intact() {
    let mut c = ctx();
    c.eval(
        r#"
        // Establish a page-global signal via the local-state surface.
        window._localState = { count: 0 };
        window.SpacetimeLocal = window._localState;
        const inner = document.querySelector('.inner'); // owns no 'count'
        ST.runMutations(inner, { js_statements: ["$count <- $count + 1"] });
        "#,
    )
    .unwrap();
    assert_true(&mut c, "window._localState.count === 1");
}

// =============================================================================
// Scope-aware reactive binding arc (compiled) — idempotent init, bounded watchers
//
// emit_reactive_binding_js registers a per-node `init` that subscribes a `render`
// closure via ST.watchScoped. init MUST be idempotent: re-running it (selector-init
// flush, dynamic-node observer, or a global `local:<dep>:updated` re-scan) must NOT
// re-add watchers — else every dep update leaks a fresh closure (unbounded growth +
// quadratic fan-out). These compile the real .st and run the emitted arc.
// =============================================================================

use spacetime::Compiler;

fn compile(src: &str) -> spacetime::CompiledSpacetime {
    let ast = spacetime::parse(src).expect("parse");
    Compiler::from_ast(&ast).without_runtime().compile()
}

#[test]
fn reactive_binding_init_is_idempotent_no_watcher_leak() {
    let compiled =
        compile("$open bool: false;\n<div class=\"box\">x</div>\n.box { .box--open: $open; }");
    let mut c = V8TestContext::new();
    c.eval("globalThis.SpacetimeLocal = { open: false }; globalThis.window = globalThis; void 0;")
        .unwrap();
    c.set_body_html(&compiled.html).unwrap();
    c.eval(include_str!("../../public/runtime/st.js")).unwrap();
    let _ = c.eval("void 0;");
    let mut c = c.load_compiled(&compiled.js).unwrap();
    c.trigger_dom_ready().unwrap();
    let _ = c.eval("if (ST._flushInit) ST._flushInit(); void 0;");

    // Dispatch the dep-update many times (each previously re-ran init -> re-added a watcher).
    c.eval(
        r#"
        for (let i = 0; i < 25; i++) {
          SpacetimeLocal.open = (i % 2 === 0);
          document.dispatchEvent(new CustomEvent('local:open:updated', { detail: SpacetimeLocal.open }));
        }
        void 0;
        "#,
    )
    .unwrap();

    // The node's dependency Set for `open` must stay bounded (exactly 1 render watcher),
    // not grow with the number of dispatches.
    let watcher_count = c
        .eval("(() => { const b = document.querySelector('.box'); const s = ST.signals.get(b); return s && s['open'] ? s['open'].d.size : 0; })()")
        .expect("eval");
    let n = watcher_count.as_number().and_then(|x| x.as_i64());
    assert_eq!(
        n,
        Some(1),
        "binding must install exactly ONE watcher on the node, not grow per dep update (got {:?})",
        watcher_count
    );

    // And it still reacts: class tracks the final $open value.
    assert_true(
        &mut c,
        "document.querySelector('.box').classList.contains('box--open') === SpacetimeLocal.open",
    );
}
