//! PLAN-144 W1 — a component hole EXPANDS; a call in markup REFUSES.
//!
//! Four spellings of "put this template here" were probed. Only one worked,
//! and the other three failed SILENTLY with `Pages: 1`, exit 0:
//!
//! | spelling                     | build | renders | result       |
//! |------------------------------|-------|---------|--------------|
//! | `.slot { &t("x"); }`         | ok    | yes     | correct      |
//! | `<main>&t("x")</main>`       | ok    | no      | escaped text |
//! | ``<main>`&t("x")`</main>``   | ok    | no      | vanishes     |
//! | `&t("x")` at file scope      | ok    | no      | vanishes     |
//!
//! Two of them emit NOTHING AT ALL — not even wrong text. And an UNKNOWN name
//! in markup is equally silent, while the same unknown name in a selector
//! block refuses correctly with E0402. The refusal machinery exists; markup
//! scope simply never consults it.
//!
//! The settled design (PLAN-144 Q1 = C, in full):
//!   - the hole form `` `&t("x")` `` EXPANDS the component. `` ` `` keeps one
//!     meaning — interpolate the thing named here — so a value interpolates a
//!     value and a component interpolates a component.
//!   - a bare call in markup or at file scope REFUSES, in the shape `@each`
//!     already uses for the identical rule.
//!   - only the CALL shape `&ident(` is affected. A bare `&name` stays a scope
//!     / score-subject reference, which `stdlib/showcases/_schools.st` depends
//!     on (`&showcase-hero-mtn__title for 35%`). That is the regression this
//!     file guards first, because breaking it silently kills the motion hero.

use std::path::PathBuf;
use std::process::Command;

struct Built {
    ok: bool,
    log: String,
    html: String,
}

fn build_src(name: &str, source: &str) -> Built {
    // Must live inside the workspace: stdlib resolution is workspace-relative
    // (BUG-326), so a /tmp page fails for an unrelated reason.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("scratch").join(format!("w1-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("index.st"), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        .current_dir(&root)
        .output()
        .expect("run build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(dir.join("dist/index.html")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    Built {
        ok: out.status.success() && !log.contains("Export failed"),
        log,
        html,
    }
}

const TPL: &str = "@template &probe-card($label) {\n    <section class=\"probe-card\">`$label`</section>\n}\n\n";

/// THE REGRESSION GUARD. A bare `&name` with no parens is a scope reference —
/// a score subject, a selector target, or literal prose. It must be untouched
/// by any refusal added for the call shape. `_schools.st` scores the motion
/// hero entirely through this spelling.
#[test]
fn a_bare_ampersand_name_is_not_a_call_and_still_works() {
    let b = build_src(
        "bare-name",
        "@import \"../../stdlib/showcases/hero/motion.st\"\n\n\
         <main class=\"page\"><div class=\"slot\"></div></main>\n\
         .page { display: block; }\n\
         .slot { &showcase-hero-motion(\"t\", \"s\", \"a\", \"b\", \"c\"); }\n",
    );
    assert!(
        b.ok,
        "the motion hero scores its subjects with bare `&name` references \
         (`&showcase-hero-mtn__title for 35%`). A refusal scoped to `&ident(` \
         must not touch them:\n{}",
        b.log
    );
    assert!(
        b.html.contains("class=\"showcase-hero-mtn\""),
        "the scored pattern must still render"
    );
}

/// Q1 = C. The hole form expands a component.
#[test]
fn a_component_hole_expands_in_markup() {
    let b = build_src(
        "hole",
        &format!("{TPL}<main class=\"page\">`&probe-card(\"held\")`</main>\n.page {{ display: block; }}\n"),
    );
    assert!(b.ok, "a component hole must compile:\n{}", b.log);
    assert!(
        b.html.contains("class=\"probe-card\""),
        "`` `&probe-card(\"held\")` `` must EXPAND the component. Today it \
         parses (parse_hole_inner emits an ELEMENT_REF) and then vanishes, \
         because the consumer stringifies it as a value:\n{}",
        b.html
    );
    assert!(
        b.html.contains("held"),
        "the argument must reach the expansion"
    );
}

/// A call written directly in markup is refused — the same rule `@each`
/// enforces loudly, applied to the surface that currently accepts silently.
#[test]
fn a_bare_call_in_markup_is_refused() {
    let b = build_src(
        "markup-call",
        &format!("{TPL}<main class=\"page\">&probe-card(\"loose\")</main>\n.page {{ display: block; }}\n"),
    );
    assert!(
        !b.ok,
        "a call in markup must REFUSE. Today it exports clean and ships \
         `&amp;probe-card(\"loose\")` as body copy:\n{}",
        b.log
    );
    assert!(
        b.log.contains("probe-card"),
        "the diagnostic must name the invocation:\n{}",
        b.log
    );
    assert!(
        b.log.to_lowercase().contains("selector"),
        "the diagnostic must state the rule — a directive binds through a \
         selector — the way the `@each` diagnostic does:\n{}",
        b.log
    );
}

/// The same call at file scope currently vanishes without a trace.
#[test]
fn a_bare_call_at_file_scope_is_refused() {
    let b = build_src(
        "file-scope-call",
        &format!("{TPL}&probe-card(\"orphan\")\n\n<main class=\"page\">x</main>\n.page {{ display: block; }}\n"),
    );
    assert!(
        !b.ok,
        "a call at file scope must REFUSE — today it emits nothing at all and \
         reports success:\n{}",
        b.log
    );
}

/// An unknown name in a selector block already refuses with E0402. The same
/// unknown name in markup must not be quieter than that.
#[test]
fn an_unknown_template_in_markup_is_refused() {
    let b = build_src(
        "unknown-markup",
        "<main class=\"page\">&no-such-template(\"x\")</main>\n.page { display: block; }\n",
    );
    assert!(
        !b.ok,
        "an unknown template in markup must refuse — `.slot {{ &no-such(…); }}` \
         already does (E0402), so markup being silent is an inconsistency, not \
         a policy:\n{}",
        b.log
    );
}

/// The spelling that always worked keeps working.
#[test]
fn a_call_in_a_selector_block_still_renders() {
    let b = build_src(
        "slot-call",
        &format!("{TPL}<main class=\"page\"><div class=\"slot\"></div></main>\n.page {{ display: block; }}\n.slot {{ &probe-card(\"slotted\"); }}\n"),
    );
    assert!(b.ok, "the selector form must keep compiling:\n{}", b.log);
    assert!(
        b.html.contains("class=\"probe-card\"") && b.html.contains("slotted"),
        "the selector form must keep rendering:\n{}",
        b.html
    );
}
