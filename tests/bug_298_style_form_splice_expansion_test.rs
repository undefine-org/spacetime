//! BUG-298 — statement-position STYLE form splices (`--card-surface;`)
//! validate (BUG-241) but must EXPAND: the declared body's declarations land
//! in the scope's emitted CSS at the splice's source position. Before this
//! fix the body was silently dropped — the page compiled green while the
//! author's declarations vanished (the arc's banned silent-acceptance class).
//!
//! The expansion is authoritative POST-import-merge (compile pipeline), so a
//! form declared in an `@import`ed file or the project's `_prelude.st`
//! resolves — the parse-time file-local E0947 for those was a false positive
//! this same pass clears.
//!
//! Behavior gates (BUG-252): the assertions read the EMITTED CSS, never the
//! source — a page that compiles but drops the body fails here.

use std::process::Command;

fn write_temp(files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "bug298-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    for (name, source) in files {
        std::fs::write(dir.join(name), source).expect("write source");
    }
    dir
}

/// Compile a page and return (success, combined stdout+stderr, emitted CSS text).
fn compile(files: &[(&str, &str)], entry: &str) -> (bool, String, String) {
    let dir = write_temp(files);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(dir.join(entry))
        .output()
        .expect("run spacetime build");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let css = std::fs::read_to_string(dir.join("spacetime.css")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), combined, css)
}

/// THE BUG: a validated style splice must contribute its declarations to the
/// scope's CSS — today they vanish.
#[test]
fn a_style_splice_expands_into_the_scope_css() {
    let src = "@form style --card-surface { background: #121722; border-radius: 9px; }\n\
               .card {\n  --card-surface;\n  color: white;\n}\n";
    let (ok, out, css) = compile(&[("index.st", src)], "index.st");
    assert!(ok, "a declared style form must compile:\n{out}");
    assert!(
        css.contains("background: #121722"),
        "the spliced background must reach the emitted CSS:\n{css}"
    );
    assert!(
        css.contains("border-radius: 9px"),
        "the spliced border-radius must reach the emitted CSS:\n{css}"
    );
    assert!(
        css.contains("color: white"),
        "the authored declaration must survive the splice:\n{css}"
    );
}

/// CSS is order-sensitive: spliced declarations land WHERE the splice sits,
/// so an authored declaration after the splice still wins the cascade.
#[test]
fn spliced_declarations_land_at_the_splice_position() {
    let src = "@form style --base { color: red; opacity: 0.5; }\n\
               .a {\n  --base;\n  color: blue;\n}\n";
    let (ok, out, css) = compile(&[("index.st", src)], "index.st");
    assert!(ok, "must compile:\n{out}");
    let red = css.find("color: red").expect("spliced color: red");
    let blue = css.find("color: blue").expect("authored color: blue");
    assert!(
        red < blue,
        "the spliced declaration must precede the authored one (splice position):\n{css}"
    );
}

/// Parameterized forms: call-site named args override; declared defaults fill
/// the rest. `$pad` substitutes as a TOKEN (`$padding` must not corrupt).
#[test]
fn call_site_args_override_and_defaults_fill() {
    let src = "@form style --surface($pad = 2rem, $padding-x = 1rem) { padding: $pad; padding-left: $padding-x; }\n\
               .a { --surface(pad: 3rem); }\n\
               .b { --surface; }\n";
    let (ok, out, css) = compile(&[("index.st", src)], "index.st");
    assert!(ok, "parameterized forms must compile:\n{out}");
    assert!(css.contains("padding: 3rem"), "named override wins:\n{css}");
    assert!(
        css.contains("padding-left: 1rem"),
        "the untouched default fills, and `$pad` must not corrupt `$padding-x`:\n{css}"
    );
    assert!(css.contains("padding: 2rem"), "the default fills:\n{css}");
}

/// A form declared in an IMPORTED file splices (authoritative validation runs
/// post-merge). Today this is a false-positive E0947 at parse time.
#[test]
fn an_imported_form_splices() {
    let brand = "@form style --brand-surface { background: #0b0e14; }\n";
    let page = "@import \"./brand.st\"\n.card {\n  --brand-surface;\n}\n";
    let (ok, out, css) = compile(&[("index.st", page), ("brand.st", brand)], "index.st");
    assert!(
        ok,
        "a form from an imported file must resolve post-merge:\n{out}"
    );
    assert!(
        css.contains("background: #0b0e14"),
        "the imported form's body must reach the CSS:\n{css}"
    );
}

/// A form declared in the project's `_prelude.st` splices into every page —
/// the overlay is the global brand-attribute surface (PLAN-135 W1).
#[test]
fn a_prelude_form_splices() {
    let prelude = "@form style --brand-surface { background: #0b0e14; }\n";
    let page = ".card {\n  --brand-surface;\n}\n";
    let (ok, out, css) = compile(
        &[("index.st", page), ("_prelude.st", prelude)],
        "index.st",
    );
    assert!(
        ok,
        "a form from the project overlay must resolve:\n{out}"
    );
    assert!(
        css.contains("background: #0b0e14"),
        "the prelude form's body must reach the CSS:\n{css}"
    );
}

/// REGRESSION PIN (BUG-241): an unknown statement-position `--name;` stays a
/// hard E0947 — expansion must not resurrect the silent drop.
#[test]
fn an_unknown_form_is_still_a_hard_error() {
    let src = ".card {\n  --card-suface;\n}\n";
    let (ok, out, _css) = compile(&[("index.st", src)], "index.st");
    assert!(!ok, "an unknown form splice must FAIL build");
    assert!(out.contains("E0947"), "the error must be E0947:\n{out}");
}

/// A non-STYLE form in bare-scope statement position is a KIND error (E0959),
/// not a silent no-op: an easing has no declarations to splice.
#[test]
fn a_non_style_form_in_statement_position_is_a_kind_error() {
    let src = "@form easing --my-curve { cubic-bezier(0.4, 0, 0.2, 1) }\n\
               .card {\n  --my-curve;\n}\n";
    let (ok, out, _css) = compile(&[("index.st", src)], "index.st");
    assert!(
        !ok,
        "an easing form in statement position must FAIL build (E0959)"
    );
    assert!(out.contains("E0959"), "the error must be E0959:\n{out}");
    assert!(
        out.contains("--my-curve"),
        "the error must name the form:\n{out}"
    );
}

/// A call-site named argument the declaration does not have is a hard E0960 —
/// silently ignoring it would drop the author's intent.
#[test]
fn an_unknown_named_argument_is_a_hard_error() {
    let src = "@form style --surface($pad = 2rem) { padding: $pad; }\n\
               .a { --surface(padding: 3rem); }\n";
    let (ok, out, _css) = compile(&[("index.st", src)], "index.st");
    assert!(!ok, "an unknown form argument must FAIL build (E0960)");
    assert!(out.contains("E0960"), "the error must be E0960:\n{out}");
    assert!(
        out.contains("padding"),
        "the error must name the unknown argument:\n{out}"
    );
}
