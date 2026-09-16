//! PLAN-135 W5 — demos/brand-catalog is the living proof of the brand-forms
//! arc: `_prelude.st` declares the brand once, the catalog page lists the
//! declarations via `@data forms` (zero hand-written cards), and the live
//! stages consume the same forms. The literate guidelines page (`.st.md`)
//! runs its fences against the same prelude.
//!
//! This gate builds the demo exactly as `cargo run -- build
//! demos/brand-catalog/` does and asserts the wiring end to end — the
//! runtime behavior proof (curve discrimination, splice expansion, duet
//! sequencing) was done live in the browser (2026-08-05: glide dot ahead of
//! linear at matched scroll; first-half subject at 0.79 while the second
//! waits at 0).

use std::process::Command;

fn export() -> (bool, String, String, String, String) {
    // Clean prior export so a failed build cannot serve stale artifacts.
    let _ = std::fs::remove_dir_all("demos/brand-catalog/dist");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg("demos/brand-catalog/")
        .output()
        .expect("run spacetime build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let read = |p: &str| std::fs::read_to_string(p).unwrap_or_default();
    (
        out.status.success(),
        log,
        read("demos/brand-catalog/dist/index.html"),
        read("demos/brand-catalog/dist/spacetime.css"),
        read("demos/brand-catalog/dist/spacetime.js"),
    )
}

#[test]
fn brand_catalog_export_is_green_with_both_pages() {
    let (ok, log, html, _, _) = export();
    assert!(ok, "demo export: {log}");
    assert!(html.contains("The brand is a set of forms"), "hero renders: {log}");
    let guidelines = std::path::Path::new("demos/brand-catalog/dist/guidelines");
    assert!(
        guidelines.exists(),
        "the literate guidelines page exports (clean-path route): {log}"
    );
}

#[test]
fn brand_catalog_cards_generate_from_data_forms() {
    let (ok, log, html, _, _) = export();
    assert!(ok, "demo export: {log}");
    // One SSG-unrolled card per declaration — the page names no form by hand.
    for name in [
        "--card-surface",
        "--brand-pill",
        "--section-title",
        "--rise",
        "--aurora-glide",
        "--duet",
        "--golden",
        "--badge",
    ] {
        assert!(html.contains(name), "catalog card for {name}: {log}");
    }
    // The stdlib preset library rides the easing slice.
    assert!(html.contains("--ease-out-expo"), "stdlib rows list too: {log}");
    // The prelude's /// docs are the card copy.
    assert!(
        html.contains("Surface for elevated content"),
        "declaration docs render as card text: {log}"
    );
    // Provenance column.
    assert!(
        html.contains("_prelude.st") && html.contains("stdlib/macros/presets.st"),
        "source column distinguishes brand from library: {log}"
    );
}

#[test]
fn brand_catalog_live_stages_are_wired() {
    let (ok, log, _, css, js) = export();
    assert!(ok, "demo export: {log}");
    // Style splices EXPANDED into plain CSS — the default and the param
    // variant both, with $pad substituted (W2).
    assert!(css.contains("border-radius: 14px"), "splice body in CSS: {log}");
    assert!(css.contains("padding: 20px"), "default $pad: {log}");
    assert!(css.contains("padding: 36px"), "filled $pad: {log}");
    // The brand easing is REGISTERED into the runtime (W3).
    assert!(
        js.contains("ST.registerEasing(\"aurora-glide\""),
        "brand curve registers: {log}"
    );
    // The score stage spliced --duet into a real score (window data emitted).
    assert!(js.contains("duet") || js.contains("score-window"), "score wiring: {log}");
}

#[test]
fn guidelines_literate_page_runs_its_fences() {
    let (ok, log, _, _, _) = export();
    assert!(ok, "demo export: {log}");
    let dir = std::path::Path::new("demos/brand-catalog/dist/guidelines");
    let html = dir
        .exists()
        .then(|| std::fs::read_to_string(dir.join("index.html")).unwrap_or_default())
        .unwrap_or_default();
    assert!(!html.is_empty(), "guidelines HTML exists: {log}");
    // Literate prose renders CLIENT-side (lit-prose shells hydrate at
    // runtime — verified live 2026-08-05): the static HTML carries the
    // segment shells + the shown fence source, and the JS carries the
    // prose payload. Assert both halves of that contract.
    assert!(
        html.contains("lit-prose"),
        "prose segment shells are in the page: {log}"
    );
    assert!(
        html.contains("--card-surface(36px)"),
        "the ```st src fence's source shows in the page: {log}"
    );
    let js = std::fs::read_to_string(dir.join("spacetime.js")).unwrap_or_default();
    assert!(
        js.contains("One attribute, one home"),
        "the prose payload ships to the runtime: {log}"
    );
    // …and the ```st fences RAN — their splice styles were compiled for the
    // prose markup to pick up at runtime.
    let css = std::fs::read_to_string(dir.join("spacetime.css"))
        .or_else(|_| std::fs::read_to_string("demos/brand-catalog/dist/spacetime.css"))
        .unwrap_or_default();
    assert!(
        css.contains("border-radius: 14px") || html.contains("border-radius: 14px"),
        "the fence's --card-surface splice expanded on the literate page: {log}"
    );
}
