//! A directive inside a `@template` body must never render as page text.
//!
//! `reconstruct_template_body_html` builds a template's HTML by span-subtraction:
//! it walks the body's children, keeps `HTML_ELEMENT` and bare holes, and CUTS
//! every construct (scope blocks, directives). That walk is one level deep —
//! `node.children()` — so it only ever sees a directive written as a DIRECT child
//! of the template body.
//!
//! A directive nested inside markup, which is where a conditional naturally goes:
//!
//! ```st
//! @template &card($t, &slot?) {
//!   <article class="card">
//!     @if &slot { <div class="actions">…</div> }
//!   </article>
//! }
//! ```
//!
//! lives inside the `<article>` element, which is KEPT wholesale — so the
//! directive's source survives into the template's HTML and is compiled into a
//! text node. The author sees the literal string `@if &slot {` on the page.
//!
//! Silent in every check: the template registers, the page renders, `check`
//! reports nothing. `stdlib/macros/template.st` documents this exact shape in
//! three examples, so the documented way to write a template produced text.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn build_js(src: &str) -> String {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!(
            "tpldir-{}-{}",
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

/// A conditional nested inside the template's markup.
const NESTED_IF: &str = r#"@template &card($t, &slot?) {
  <article class="card">
    <h2>`$t`</h2>
    @if &slot { <div class="card__actions">`&slot`</div> }
  </article>
}

<main class="p"></main>

.p { &c &card("B"); }
"#;

/// An `@each` nested inside the template's markup — same shape, different
/// directive, so the gate describes the CLASS rather than one keyword.
const NESTED_EACH: &str = r#"@template &list($t) {
  <article class="list">
    <h2>`$t`</h2>
    @each($items as $i) { <span class="e">e</span> }
  </article>
}

<main class="p"></main>

.p { &c &list("B"); }
"#;

/// Check a page and return the compiler's combined output.
fn check_output(src: &str) -> String {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!(
            "tplchk-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let file = dir.join("index.st");
    std::fs::write(&file, src).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("spacetime binary should run");
    let _ = std::fs::remove_dir_all(&dir);
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text
}

/// The author must be TOLD. Emitting the directive as page text with no
/// diagnostic is the silent-drop shape this repo keeps paying for: the template
/// registers, the page renders, `check` reports nothing, and the reader sees
/// `@if &slot {` in their markup.
///
/// Cutting the span is not currently possible here — HTML content is lexed as
/// flat `HTML_RAW` tokens, so a directive nested in markup is never a CST child
/// of the template body and the construct-cut cannot see it. Until that changes
/// (BUG-358), the contract is a LOUD refusal, which is the same bar `@each` and
/// `@view` already meet through W0712.
#[test]
fn a_nested_if_is_reported_not_silently_rendered() {
    let out = check_output(NESTED_IF);
    assert!(
        out.contains("W0712"),
        "`@if` inside a template's markup renders as page TEXT, so it must be \
         reported — `@each` and `@view` already are. Got:\n{out}"
    );
}

#[test]
fn a_nested_each_is_reported_not_silently_rendered() {
    let out = check_output(NESTED_EACH);
    assert!(
        out.contains("W0712"),
        "`@each` nested in template markup must be reported — same one-level cut, \
         so the diagnostic must describe the CLASS, not one keyword. Got:\n{out}"
    );
}

/// The paren spelling leaks identically and must be reported identically.
/// `stdlib/macros/template.st` documents THIS form in three examples, so it is
/// the shape an author following the docs actually writes.
#[test]
fn the_documented_paren_spelling_is_reported_too() {
    let out = check_output(
        r#"@template &card($t, &slot?) {
  <article class="card">
    @if(&slot) { <div class="card__actions">`&slot`</div> }
  </article>
}

<main class="p"></main>

.p { &c &card("B"); }
"#,
    );
    assert!(
        out.contains("W0712"),
        "`@if(&slot)` is the spelling stdlib's own docs use inside a template \
         body; it leaks the same way and must be reported. Got:\n{out}"
    );
}

/// Over-widening companion #1: cutting the directive must not cut the markup
/// AROUND it. The element that hosts the directive, and its siblings, stay.
#[test]
fn the_surrounding_markup_survives_the_cut() {
    let js = build_js(NESTED_IF);
    assert!(
        js.contains("article") && js.contains("card"),
        "the host element must survive — a cut that removes the enclosing \
         markup would also pass a bare 'directive is absent' check"
    );
    assert!(
        js.contains("h2"),
        "a sibling of the directive must survive the cut"
    );
}

/// Over-widening companion #2: an `@` in ordinary page TEXT is not a directive
/// and must not be cut. Otherwise "remove directives" silently eats content.
#[test]
fn an_at_sign_in_template_text_is_not_cut() {
    let js = build_js(
        r#"@template &card($t) {
  <article class="card">
    <p class="email">write to us @ support</p>
  </article>
}

<main class="p"></main>

.p { &c &card("B"); }
"#,
    );
    assert!(
        js.contains("write to us @ support"),
        "a literal `@` in template text is content, not a directive, and must \
         survive"
    );
}

// ── `@if` is a STUB, and the compiler must keep saying so (BUG-362) ────────
//
// The template-markup leak above is a SYMPTOM. The disease is that `@if` has
// no implementation anywhere: `%macro conditional` (stdlib/macros/each.st)
// declares a `%form` and a `%when` clause, but no `%emit` block and no
// primitive — so at page level, outside any template, the compiler refuses it:
//
//     error[E0956]: unknown primitive: conditional
//
// That refusal is the ONLY thing standing between an author and a construct
// that silently renders nothing. A conditional that emits nothing satisfies
// every `should not_exist` assertion, so the false branch always looks
// correct — which is exactly how this survived with zero usages in stdlib,
// demos and examples.
//
// This gate pins the refusal so it cannot be quietly weakened. When `@if` is
// really implemented, this test MUST be replaced by a behavior gate (renders
// on true, not on false, and TOGGLES when the signal changes) — not deleted.

/// `@if` in a selector scope at page level — the position where a conditional
/// most obviously belongs, and nowhere near a `@template`.
const PAGE_LEVEL_IF: &str = r#"@import "stdlib"
<main class="h1"></main>
body { $show bool: true; }
.h1 { @if($show) { > .yes { "Y" } } }
"#;

/// The same shape with `@each`, which IS implemented. This is the control: it
/// proves the surrounding grammar and the scope position are fine, so the
/// refusal above is about `conditional` specifically and not about where it
/// was written.
const PAGE_LEVEL_EACH: &str = r#"@import "stdlib"
<main class="h3"></main>
body { $items array: [1, 2, 3]; }
.h3 { @each($items as $i) { > .row { "row" } } }
"#;

// Reuses the `check_output` helper defined above — same job, so one copy.

#[test]
fn an_unimplemented_if_is_refused_not_silently_dropped() {
    let out = check_output(PAGE_LEVEL_IF);
    assert!(
        out.contains("E0956"),
        "`@if` has no %emit block and no primitive, so it must be REFUSED. \
         Silently accepting it renders nothing while satisfying every \
         `not_exist` assertion — the failure mode that hid this. \
         If `@if` was just implemented, replace this gate with a behavior \
         test (true renders, false does not, and it TOGGLES). Got:\n{out}"
    );
}

#[test]
fn the_implemented_sibling_in_the_same_position_compiles() {
    // Control. Without this, the gate above would also pass if scope bodies
    // were broken wholesale, or if `check` had started failing everything.
    let out = check_output(PAGE_LEVEL_EACH);
    assert!(
        !out.contains("E0956"),
        "`@each` IS implemented and must compile in the very position where \
         `@if` is refused — otherwise the refusal above is about the position, \
         not about `conditional`. Got:\n{out}"
    );
}
