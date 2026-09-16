//! Integration tests for the stdlib/text module (PLAN-024 W4).
//!
//! Verifies the pretext fusion at the EMIT level — the layer that works today.
//! (Real-DOM .test.st gating is blocked on BUG-051 / PLAN-026.)
//!
//! Asserts:
//! - @measure expands to the measure-text primitive (pretext calls present)
//! - the vendored pretext IIFE is demand-injected when @measure fires
//! - a page importing text but NOT measuring ships ZERO pretext bytes (lean law)
//! - @reveal split:"lines" carries the pretext-backed path (layoutWithLines)
//!   with a graceful offsetTop fallback

use spacetime::compiler::Compiler;
use std::path::Path;

/// Compile a source string at the repo root (so `@import "stdlib/text"` resolves
/// against the on-disk stdlib + committed vendor bundle).
fn compile(src: &str, stem: &str) -> spacetime::compiler::CompiledSpacetime {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = root.join(format!("target/tmp-text-{stem}.st"));
    std::fs::create_dir_all(p.parent().unwrap()).ok();
    std::fs::write(&p, src).unwrap();
    let compiled = Compiler::from_file(&p, root).unwrap().compile();
    std::fs::remove_file(&p).ok();
    assert!(
        compiled.pipeline_errors.is_empty(),
        "pipeline errors: {:?}",
        compiled.pipeline_errors
    );
    compiled
}

#[test]
fn measure_expands_to_pretext_primitive() {
    let js = compile(
        "@import \"stdlib/text\"\n\n.headline { @measure(lineHeight: 1.5) }\n",
        "measure",
    )
    .js;
    // The primitive body references pretext's measurement surface.
    assert!(
        js.contains("prepareWithSegments"),
        "measure-text should call pretext.prepareWithSegments"
    );
    assert!(
        js.contains("measureNaturalWidth"),
        "measure-text should call pretext.measureNaturalWidth"
    );
    assert!(
        js.contains("ResizeObserver"),
        "measure should re-measure on resize"
    );
}

#[test]
fn measure_demand_injects_pretext_bundle() {
    // Only assert injection when the committed bundle exists on disk.
    if !Path::new("stdlib/text/vendor/pretext.bundle.js").exists() {
        return;
    }
    let js = compile(
        "@import \"stdlib/text\"\n\n.headline { @measure(lineHeight: 1.5) }\n",
        "inject",
    )
    .js;
    assert!(
        js.contains("globalThis.pretext"),
        "vendored pretext IIFE should be demand-injected when @measure fires"
    );
    assert!(
        js.contains("vendored: pretext"),
        "provenance header should precede the bundle"
    );
}

#[test]
fn unused_text_import_ships_no_pretext() {
    // Lean law: importing the module but never invoking a text primitive ships
    // zero pretext bytes.
    let js = compile(
        "@import \"stdlib/text\"\n\n.plain { color: red; font-size: 2rem; }\n",
        "unused",
    )
    .js;
    assert!(
        !js.contains("globalThis.pretext"),
        "unused vendor must not be injected"
    );
    assert!(
        !js.contains("prepareWithSegments"),
        "unused primitive must not emit"
    );
}

#[test]
fn balance_expands_to_pretext_binary_search() {
    let js = compile(
        "@import \"stdlib/text\"\n\n.title { @balance(lineHeight: 1.2) }\n",
        "balance",
    )
    .js;
    assert!(
        js.contains("prepareWithSegments"),
        "balance should prepare via pretext"
    );
    assert!(
        js.contains("maxInlineSize"),
        "balance should cap max-inline-size"
    );
    if Path::new("stdlib/text/vendor/pretext.bundle.js").exists() {
        assert!(
            js.contains("globalThis.pretext"),
            "balance page injects pretext bundle"
        );
    }
}

#[test]
fn fit_expands_to_pretext_measure() {
    let js = compile(
        "@import \"stdlib/text\"\n\n.fit { @fit(min: 16, max: 120) }\n",
        "fit",
    )
    .js;
    assert!(
        js.contains("measureNaturalWidth"),
        "fit should measure natural width"
    );
    assert!(js.contains("fontSize"), "fit should set font-size");
}

#[test]
fn balance_unused_ships_no_pretext() {
    // Importing text + using only @fit must NOT pull balance's behavior, and a
    // page using neither still ships nothing.
    let js = compile(
        "@import \"stdlib/text\"\n\n.plain { color: blue; }\n",
        "layout-unused",
    )
    .js;
    assert!(
        !js.contains("maxInlineSize"),
        "unused balance must not emit"
    );
    assert!(
        !js.contains("globalThis.pretext"),
        "no vendor for non-text page"
    );
}

#[test]
fn reveal_lines_uses_pretext_with_offsettop_fallback() {
    // @reveal split:"lines" is a CORE primitive that progressively enhances with
    // pretext when present. Both paths must be in the emitted primitive.
    let js = compile(
        "@import \"stdlib/text\"\n\n.kinetic { @reveal(split: \"lines\", trigger: load) }\n",
        "reveal",
    )
    .js;
    // Preferred path: pretext line layout.
    assert!(
        js.contains("layoutWithLines"),
        "reveal lines should use pretext.layoutWithLines when available"
    );
    // Graceful fallback: offsetTop grouping.
    assert!(
        js.contains("offsetTop"),
        "reveal lines should retain the offsetTop fallback"
    );
    // And the pretext bundle is injected because the primitive references the global.
    if Path::new("stdlib/text/vendor/pretext.bundle.js").exists() {
        assert!(
            js.contains("globalThis.pretext"),
            "pretext bundle injected for reveal-lines page"
        );
    }
}

#[test]
fn reveal_lines_without_text_import_keeps_fallback_only() {
    // @reveal is core: usable WITHOUT importing stdlib/text. The pretext branch is
    // guarded by `typeof pretext !== 'undefined'`, so no bundle is shipped.
    let js = compile(
        "@import \"stdlib\"\n\n.kinetic { @reveal(split: \"lines\", trigger: load) }\n",
        "reveal-core",
    )
    .js;
    assert!(
        js.contains("offsetTop"),
        "core reveal retains offsetTop line grouping"
    );
    assert!(
        !js.contains("globalThis.pretext"),
        "core-only page ships no pretext bundle"
    );
}

/// FUP-078: a bodyless (no-`;`) `@import` immediately followed by a newline-led
/// element-selector scope must NOT swallow that scope. Before the parser guard,
/// the first scope (and any directive inside it) vanished, collapsing the whole
/// page's runtime JS to 0 bytes — silently (`check` stayed green, page blank).
///
/// This is the end-to-end emit proof of the symptom: the directive-bearing
/// leading scope must produce a non-empty runtime, byte-identical in spirit to
/// the same page with a plain CSS rule placed first (the documented workaround).
#[test]
fn fup078_leading_directive_scope_after_bare_import_emits_runtime() {
    // The reported case: directive scope is the FIRST construct after the import.
    let lead = compile(
        "@import \"stdlib/text\"\n.headline { @measure(lineHeight: 1.5) }\n",
        "fup078-lead",
    )
    .js;
    assert!(
        !lead.is_empty(),
        "leading directive scope after a bare @import must emit a non-empty runtime (was 0 bytes)"
    );
    // The measure primitive must actually be present — proves the scope's
    // directive was FormMatch'd, not merely that SOME bytes were emitted.
    assert!(
        lead.contains("measureNaturalWidth"),
        "the @measure directive in the leading scope must be lowered, got {} bytes",
        lead.len()
    );

    // Control: the documented workaround (plain rule first) must emit the same
    // measure primitive — the two pages are now equivalent.
    let workaround = compile(
        "@import \"stdlib/text\"\nbody { margin: 0 }\n.headline { @measure(lineHeight: 1.5) }\n",
        "fup078-workaround",
    )
    .js;
    assert!(
        workaround.contains("measureNaturalWidth"),
        "control page (plain rule first) must also lower @measure"
    );
}

/// FUP-078 element-selector coverage: a bare element scope (`h1 { … }`) — not a
/// `.class`/`#id` — is the exact shape the original guards (BUG-047/070) missed.
#[test]
fn fup078_bare_element_scope_after_import_survives() {
    let js = compile(
        "@import \"stdlib/text\"\nh1 { @measure(lineHeight: 1.2) }\n",
        "fup078-elem",
    )
    .js;
    assert!(
        js.contains("measureNaturalWidth"),
        "a bare element scope after a no-semicolon @import must lower its directive"
    );
}
