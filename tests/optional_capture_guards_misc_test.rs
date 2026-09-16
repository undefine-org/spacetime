//! Guard fan-out for optional directive captures (BUG-264) — the MISC family.
//!
//! This is the companion suite to `optional_capture_audit_test.rs` (the two
//! LANDED guards in portal.st + data-kind.st). It covers the optional captures
//! in the nine stdlib macro files assigned to this track: drag.st, websocket.st,
//! each.st, editable.st, bindings.st, fade-in.st, host.st, on-event.st, form.st.
//!
//! The sibling mechanism fires only when the PRODUCTIVE form goes LENIENT — i.e.
//! the matcher flags a clause it could not bind. Form compiler leniency is
//! detected in three structural places (form_compiler.rs):
//!   1. an ARG_LIST param whose group leaves unbound tokens (`has_dropped_tokens`,
//!      lines ~1078 / ~1142),
//!   2. a BODY param (no body_capture) that leaves unbound tokens (line ~1305), and
//!   3. post-inline trailing tokens after the inline/arg-list segment of a
//!      body-less form (the `): T` run's leftover; step 3.6's else-if). This was
//!      the matcher gap that kept class 3's sibling dead — now closed (below).
//!
//! So a guard is possible only for an optional capture whose TYPE CAN REJECT a
//! plausible near-miss. `:expr` never rejects (ExprExtractor greedily binds any
//! non-empty token run), so an `:expr?` capture can never go lenient and no
//! sibling can ever fire — those are documented as NO-PLAUSIBLE-NEAR-MISS below
//! rather than given dead siblings.
//!
//! ## Per-capture disposition
//!
//! ### GUARDED (firing sibling added this track)
//! - on-event.st `on-mutation-handler`: `target: $target:selector?`.
//!   `target: 5` / `target: submitBtn` are refused by `:selector?`, drop
//!   silently (lenient), and the `on-mutation-target-invalid` sibling reports
//!   E0955. Tested below (neg + pos).
//! - websocket.st `presence`: `as $name:ident : $type:typeref? = any` — class 3
//!   (formerly a matcher gap, closed this track). The `: $type` slot is
//!   POST-INLINE (after the `@presence(...)` arg-list); `as viewers: 5` leaves
//!   `5` as a post-inline leftover. form_compiler's step 3.6 else-if now flags
//!   that leftover as lenient (site 3 above), so the productive form no longer
//!   matches cleanly and the `presence-type-invalid` sibling (declared after
//!   `presence`) fires E0955. Tested below (neg + pos).
//!
//! ### NO-PLAUSIBLE-NEAR-MISS (sibling structurally impossible; documented)
//! `:expr` greedily binds any non-empty value, so the productive form never goes
//! lenient on a present value and a sibling can never fire:
//! - drag.st `@swipe`: `on-left/on-right/on-up/on-down: $…:expr?` (4). Verified:
//!   `on-left: 5`, `on-left: &handle`, `on-left: :hover` all bind cleanly.
//! - websocket.st `@realtime`: `query: $query:expr?`, `order: $order:expr?` (2).
//! - each.st `@each` / `@each-filtered`: `key: $key:expr?` (2).
//! - editable.st `@editable` / `@richtext`: `marks: $marks:expr?`, `blocks: $blocks:expr?`.
//! - bindings.st `@localStorage`: `$default:expr?`.
//! - host.st `host-http-opts`: `headers: $headers:object?` — `object` is not a
//!   defined `%capture_type`, so it resolves to the Custom→Expr fallback (fully
//!   lenient); `headers: 5` binds cleanly.
//!
//! And a capture whose type is PERMISSIVE on the plausible inputs (no wrong-shape
//! near-miss an author would actually write):
//! - fade-in.st `@fade-in`: `$customKeyframes:keyframes?` — `keyframes` accepts
//!   `prop: value;` and `selector { … }` bodies; the plausible custom-animation
//!   bodies (`{ scale: 0.95 -> 1 }`, `{ opacity: 0 -> 1 }`) all bind. Only a
//!   bare-token body (`{ 5 }`) is rejected-and-dropped, which is not a plausible
//!   authoring near-miss.
//!
//! ### ALREADY-LOUD (fallback parse already errors; no sibling needed)
//! - form.st the six kind macros (`style`/`value`/`motion`/`easing`/`score`/`markup`):
//!   `$params:param_list?`. A malformed param list (`--f(5)`, `--f($x, 5)`,
//!   `--f(5, )`) makes the `)` literal fail after the kind/name commit, which the
//!   matcher reports as E0946 (CaptureTypeMismatch). Verified: all three error
//!   loudly; empty `--f()` and valid `--f($x, &sel)` compile. No silent drop.

use std::process::Command;

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn check(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .current_dir(REPO_ROOT)
        .args(["check", entry.to_str().expect("utf-8 path")])
        .output()
        .expect("spacetime check runs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn on_mutation_target_with_a_bare_number_is_refused_not_dropped() {
    // `target: $target:selector?` refuses a bare number; without the sibling the
    // productive `on-mutation-handler` form drops it LENIENTLY (silent). The
    // `on-mutation-target-invalid` sibling reports E0955 naming the repair.
    let text = check(
        r#"@version 2026-06-09;
.c { @on-mutation(event: click, target: 5, mutations: []); }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("target: .btn"),
        "a bare-number target must be refused with E0955 naming the selector repair; \
         otherwise the lenient productive match silently drops the delegation target. Got:\n{text}"
    );
}

#[test]
fn on_mutation_target_with_a_non_element_ident_is_refused_not_dropped() {
    // `submitBtn` is not a selector start (the selector capture requires `.`/`#`/
    // `[`/combinator or an HTML element name), so it too drops leniently and the
    // sibling must report it.
    let text = check(
        r#"@version 2026-06-09;
.c { @on-mutation(event: click, target: submitBtn, mutations: []); }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("target:"),
        "a non-element-ident target must be refused with E0955. Got:\n{text}"
    );
}

#[test]
fn on_mutation_target_with_a_valid_selector_still_compiles() {
    // The sibling must not steal a valid delegation target.
    let text = check(
        r#"@version 2026-06-09;
.c { @on-mutation(event: click, target: .btn, mutations: []); }
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a valid `.btn` selector must compile clean; the error sibling must not \
         fire on a legitimate target. Got:\n{text}"
    );
}

#[test]
fn on_mutation_target_with_an_element_name_still_compiles() {
    // An HTML element name is a valid selector start.
    let text = check(
        r#"@version 2026-06-09;
.c { @on-mutation(event: click, target: button, mutations: []); }
"#,
    );
    assert!(
        !text.contains("E0955"),
        "an element-name target must compile clean. Got:\n{text}"
    );
}

#[test]
fn on_mutation_with_target_omitted_still_compiles() {
    // The optional target may be omitted entirely (defaults to "").
    let text = check(
        r#"@version 2026-06-09;
.c { @on-mutation(event: click, mutations: []); }
"#,
    );
    assert!(
        !text.contains("E0955"),
        "an omitted target must compile clean. Got:\n{text}"
    );
}

#[test]
fn form_malformed_params_error_loudly_already_loud() {
    // form.st's six `$params:param_list?` captures: a malformed param list is
    // already refused by the matcher (E0946, CaptureTypeMismatch after the
    // kind/name commit) — no silent drop, so no sibling is added.
    let text = check(
        r#"@version 2026-06-09;
@form value --f(5) { result: 1; }
"#,
    );
    assert!(
        text.contains("E0946"),
        "a malformed @form param list must already error loudly (E0946), not drop \
         silently. Got:\n{text}"
    );
}

#[test]
fn form_valid_and_empty_params_compile() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(
        &entry,
        r#"@version 2026-06-09;
@form value --double($x) { result: $x * 2; }
@form value --plain() { result: 1; }
"#,
    )
    .expect("fixture writes");
    let output = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .current_dir(REPO_ROOT)
        .args(["check", entry.to_str().expect("utf-8 path")])
        .output()
        .expect("spacetime check runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !text.contains("E0946") && !text.contains("E0955"),
        "valid and empty param lists must compile clean. Got:\n{text}"
    );
}

#[test]
fn presence_post_inline_type_near_miss_is_refused_not_dropped() {
    // Class 3 (matcher gap closed this track): `as $name:ident : $type:typeref?`
    // — the `: $type` slot is POST-INLINE (after the `@presence(...)` arg-list),
    // so `as viewers: 5` leaves `5` as a post-inline leftover. Without the
    // step-3.6 else-if leniency the productive `presence` form matched CLEANLY and
    // dropped the `: 5` silently (the sibling stayed dead). Now the productive
    // form goes lenient and `presence-type-invalid` fires E0955 naming the repair.
    let text = check(
        r#"@version 2026-06-09;
.c { @presence(room: "doc", throttle: 50ms) as viewers: 5; }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("Viewer[]"),
        "a non-type value in the post-inline `: $type` slot must be refused with \
         E0955 naming the TYPE repair; otherwise the lenient productive match \
         silently drops the `: 5`. Got:\n{text}"
    );
}

#[test]
fn presence_with_a_valid_type_still_compiles() {
    // The sibling must not steal a valid typeref. `$name:ident` takes a bare
    // identifier (or `$`-prefixed binding); `: Viewer[]` binds the typeref
    // cleanly, the productive form matches cleanly and wins.
    let text = check(
        r#"@version 2026-06-09;
.c { @presence(room: "doc") as $viewers: Viewer[]; }
"#,
    );
    assert!(
        !text.contains("E0955") && !text.contains("E0946"),
        "a valid `as $viewers: Viewer[]` type must compile clean. Got:\n{text}"
    );
}

#[test]
fn presence_with_type_omitted_still_compiles() {
    // The optional `: $type:typeref?` (default `= any`) may be omitted entirely.
    let text = check(
        r#"@version 2026-06-09;
.c { @presence(room: "doc") as $viewers; }
"#,
    );
    assert!(
        !text.contains("E0955") && !text.contains("E0946"),
        "an omitted optional type must compile clean. Got:\n{text}"
    );
}
