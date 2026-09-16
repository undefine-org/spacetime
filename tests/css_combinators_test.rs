//! CSS combinators must survive the `~` (PRESET_REF) retirement.
//!
//! `~` is used for TWO unrelated things:
//!   * PREFIX  — `~preset-name`, the PRESET_REF sigil that SIP-001c retires.
//!   * INFIX   — `.a ~ .b`, CSS's general sibling combinator, which is not going
//!     anywhere and is not Spacetime's to remove.
//!
//! `parse_selector` handles the infix form (`TILDE => parse_selector_part()`,
//! beside `GT` for child and `PLUS` for adjacent sibling), while
//! `parse_preset_ref()` handles the prefix form from SEVEN dispatch sites. An
//! earlier draft of this work assumed TILDE was purely a preset sigil and could
//! be removed wholesale — which would have silently broken every stylesheet
//! using a sibling combinator.
//!
//! This file lands BEFORE the retirement, so the deletion is done against a
//! green assertion rather than an assumption.

use std::process::Command;

fn build_css(source: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "css-comb-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");

    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    assert!(
        out.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let css = std::fs::read_to_string(dir.join("spacetime.css")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    css
}

#[test]
fn general_sibling_combinator_survives() {
    let css = build_css(".a { color: red; }\n.a ~ .b { color: blue; }\n");
    assert!(
        css.contains(".a ~ .b"),
        "`~` is CSS's GENERAL SIBLING combinator. Retiring the `~preset` PREFIX \
         sigil must not touch the infix selector path.\nCSS:\n{css}"
    );
}

#[test]
fn all_four_combinators_survive_together() {
    // Descendant is implicit; the three explicit combinators plus a compound
    // selector, in one stylesheet, so a change to any one path is caught.
    let css = build_css(
        ".a { color: red; }\n\
         .a ~ .b { color: blue; }\n\
         .a + .c { color: green; }\n\
         .a > .d { color: teal; }\n\
         .a .e { color: olive; }\n",
    );
    for sel in [".a ~ .b", ".a + .c", ".a > .d", ".a .e"] {
        assert!(
            css.contains(sel),
            "combinator `{sel}` was lost.\nCSS:\n{css}"
        );
    }
}

#[test]
fn sibling_combinator_chains() {
    let css = build_css(".a ~ .b ~ .c { color: red; }\n");
    assert!(
        css.contains(".a ~ .b ~ .c"),
        "a CHAIN of sibling combinators must survive -- the infix path is \
         recursive, and a partial retirement could truncate it.\nCSS:\n{css}"
    );
}

/// BUG-243 — a NESTED sibling combinator must stay a sibling combinator.
///
/// `parse_body_item` recognized only `GT` and `PLUS` as combinator-led nested
/// scopes; `TILDE` fell through to the preset-reference arm, so `~` was eaten as
/// a (meaningless) preset ref and `.b` parsed as an ordinary nested scope —
/// lowering to `.a .b`, a DESCENDANT selector. The page silently styled the
/// wrong elements, with no diagnostic, while `>` and `+` nested correctly.
///
/// Every case here is nested, because the top-level path was already correct and
/// is what hid this bug: a suite that only tests top level stays green.
#[test]
fn nested_combinators_keep_their_meaning() {
    let css = build_css(
        ".a {\n  color: red;\n  ~ .b { color: blue; }\n  > .c { color: green; }\n  + .d { color: teal; }\n}\n",
    );
    for sel in [".a ~ .b", ".a > .c", ".a + .d"] {
        assert!(
            css.contains(sel),
            "nested combinator `{sel}` was lost or rewritten.\nCSS:\n{css}"
        );
    }
    assert!(
        !css.contains(".a .b"),
        "BUG-243: nested `~ .b` became a DESCENDANT selector `.a .b`, which \
         targets a different set of elements entirely.\nCSS:\n{css}"
    );
}

/// SIP-001c / BUG-263: the `~` PRESET_REF PREFIX is retired, so `~` is now
/// EXCLUSIVELY the CSS general-sibling combinator. There is no `~name`
/// disambiguation left to coexist with — `@preset easing ~smooth` errors loudly
/// at parse. This proves the combinator survives that retirement.
#[test]
fn nested_sibling_combinator_and_preset_ref_coexist() {
    let css = build_css(
        ".a {\n  color: red;\n  ~ .b { color: blue; }\n}\n",
    );
    assert!(
        css.contains(".a ~ .b"),
        "the combinator reading must survive the `~` preset retirement.\nCSS:\n{css}"
    );
}

/// The retired `@preset ... ~name` spelling now FAILS LOUDLY, never silently
/// degrading to a descendant selector.
#[test]
fn retired_preset_ref_errors_loudly() {
    let dir = std::env::temp_dir().join(format!(
        "csscomb-retired-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(
        &file,
        "@preset easing ~smooth: cubic-bezier(0.4, 0, 0.2, 1);\n.a { ~ .b { color: blue; } }\n",
    )
    .expect("write source");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!out.status.success(), "`~name` preset must not compile");
    assert!(
        combined.contains("preset references are retired"),
        "must fail with the loud retirement diagnostic, got:\n{combined}"
    );
}
