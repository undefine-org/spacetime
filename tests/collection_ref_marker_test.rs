//! A collection ref's `[]` is GRAMMAR, not part of the selector.
//!
//! `&cards[] .cref-cards { @each(…) }` names `.cref-cards` as a collection ref.
//! `ElementRefStmt::selector_text()` returns the raw selector run, and because
//! `parse_element_ref` stops at the ident, the `[]` marker leaks into it as a
//! leading token: `"[] .cref-cards"`.
//!
//! Exactly one of the four callers stripped it. The template path did not, so a
//! template containing a collection-ref block emitted
//! `querySelector("[] .cref-cards")` — invalid CSS, which threw
//! `Expected name, found ]` out of the runtime's selector parser and took the
//! whole block's `@each` with it. Four tests in
//! `tests/unit/templates/collection-ref-each.test.st` failed on the fallout
//! ("container should have 3 children", then `undefined.classList`).
//!
//! The marker is stripped at the ACCESSOR, so every caller is correct by
//! default and a fifth caller cannot reintroduce the bug by forgetting.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Build a page and return its emitted JS bundle.
fn build_js(src: &str) -> String {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!(
            "crefmark-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    std::fs::write(dir.join("index.st"), src).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        .output()
        .expect("spacetime binary should run");
    assert!(
        out.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(dir.join("dist").join("spacetime.js")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    js
}

const TEMPLATE_PAGE: &str = r#"@template &cref-card($item) {
    <div class="cref-card"></div>
}

@template &cref-dashboard() {
    <section class="cref-dashboard">
        <div class="cref-cards"></div>
    </section>

    &cards[] .cref-cards {
        @each($crefItems as $item) {
            &cref-card($item);
        }
    }
}

<main><div class="host"></div></main>
"#;

/// The reported case: the marker must never reach an emitted selector.
#[test]
fn a_collection_ref_marker_does_not_leak_into_the_emitted_selector() {
    let js = build_js(TEMPLATE_PAGE);
    assert!(
        !js.contains("[] .cref-cards"),
        "the `[]` collection marker leaked into an emitted selector. \
         `querySelector(\"[] .cref-cards\")` is invalid CSS and throws \
         `Expected name, found ]` at runtime, silently taking the block's \
         @each with it."
    );
}

/// …and the real target must still be there. A refusal-shaped assertion alone
/// ("the bad string is absent") is satisfied by emitting nothing at all, so the
/// selector the block actually targets is asserted present.
#[test]
fn the_collection_ref_still_targets_its_container() {
    let js = build_js(TEMPLATE_PAGE);
    assert!(
        js.contains(".cref-cards"),
        "stripping the marker must leave the real CSS target `.cref-cards` — \
         emitting nothing would also pass a bare absence check"
    );
}

/// Over-widening companion. Only a LEADING collection marker is grammar; `[]`
/// inside a real attribute selector is CSS and must survive untouched.
#[test]
fn an_attribute_selector_is_not_stripped() {
    let js = build_js(
        r#"@template &attr-card($item) {
    <div class="attr-card"></div>
}

@template &attr-dash() {
    <section class="attr-dash">
        <div class="attr-cards" data-role="list"></div>
    </section>

    &rows[] .attr-cards[data-role="list"] {
        @each($attrItems as $item) {
            &attr-card($item);
        }
    }
}

<main><div class="host"></div></main>
"#,
    );
    assert!(
        js.contains(r#"[data-role=\"list\"]"#) || js.contains(r#"[data-role="list"]"#),
        "an attribute selector's brackets are CSS, not the collection marker, \
         and must survive: stripping by `[]` anywhere would corrupt it"
    );
}
