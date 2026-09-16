//! BUG-327: a `@form style` body may splice another form — and a CYCLE is refused.
//!
//! The behavior half (composition reaches the consumer, siblings survive, depth
//! works) is `tests/bugs/BUG-327-a-form-body-may-splice-a-form.test.st`, asserted
//! on computed style under `--cdp`.
//!
//! This file holds the half that a `.test.st` structurally cannot: what the
//! COMPILER says when composition has no fixed point. A source that fails to
//! compile cannot also carry an assertion about the failure.

use std::process::Command;

fn check(src: &str) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!("bug327-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("index.st");
    std::fs::write(&file, src).expect("write probe");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("run spacetime check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), text)
}

/// A direct cycle has no fixed point. Refuse it, naming the chain — the
/// alternative is a hang (expansion recursing forever) or the silent empty rule
/// that BUG-327 produced before composition was implemented at all.
#[test]
fn a_form_splice_cycle_is_refused_by_name() {
    let (passed, out) = check(
        "@form style --cyc-a { --cyc-b; color: red; }\n\
         @form style --cyc-b { --cyc-a; font-size: 12px; }\n\
         \n\
         <div class=\"x\">X</div>\n\
         .x { --cyc-a; }\n",
    );
    assert!(!passed, "a splice cycle must fail the check:\n{out}");
    assert!(out.contains("E0966"), "must be E0966:\n{out}");
    assert!(
        out.contains("--cyc-a") && out.contains("--cyc-b"),
        "the diagnostic must name the CHAIN, so the author can see which link to break:\n{out}"
    );
}

/// A form that splices ITSELF is the degenerate cycle — same refusal, and worth
/// pinning separately because a depth-1 guard is the easy thing to get wrong.
#[test]
fn a_form_that_splices_itself_is_refused() {
    let (passed, out) = check(
        "@form style --selfish { --selfish; color: red; }\n\
         \n\
         <div class=\"x\">X</div>\n\
         .x { --selfish; }\n",
    );
    assert!(!passed, "a self-splice must fail the check:\n{out}");
    assert!(out.contains("E0966"), "must be E0966:\n{out}");
}

/// The composition this all exists for: a ROLE bound by a school. Compiles, and
/// the leaf's declaration reaches the consumer through two hops.
#[test]
fn a_role_binding_chain_compiles() {
    let (passed, out) = check(
        "@form style --school-title { font-size: 44px; }\n\
         @form style --role-title { --school-title; letter-spacing: -0.02em; }\n\
         \n\
         <h1 class=\"t\">T</h1>\n\
         .t { --role-title; }\n",
    );
    assert!(
        passed,
        "role indirection is the point of composition — it must compile:\n{out}"
    );
}

/// `check` must agree with `build`. Imports are parsed UNVALIDATED (a module's
/// `--name;` is declared by a sibling file it cannot see), so the parse-time
/// E0947s they carry are meaningless until the merge. `build` cleared them at
/// its pipeline seam and `check` did not — so every `stdlib/showcases` pattern
/// reported four "unknown form" errors from `check` while building perfectly.
/// A diagnostic tool that disagrees with the compiler is worse than none.
#[test]
fn check_resolves_form_declarations_across_an_import() {
    let dir = std::env::temp_dir().join(format!("bug327-imp-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(
        dir.join("lib.st"),
        "@form style --lib-title { font-size: 44px; }\n",
    )
    .expect("write lib");
    let page = dir.join("index.st");
    std::fs::write(
        &page,
        "@import \"./lib.st\"\n\n<h1 class=\"t\">T</h1>\n.t { --lib-title; }\n",
    )
    .expect("write page");

    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&page)
        .output()
        .expect("run check");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "a form declared in an IMPORTED file must satisfy the splice — `build` \
         accepts this exact source:\n{text}"
    );
    assert!(
        !text.contains("E0947"),
        "no unknown-form error may survive the merge:\n{text}"
    );
}

/// The payoff: a design school is swapped by REBINDING one role, with no edit
/// to the pattern that splices it. This is what composition bought.
#[test]
fn rebinding_a_role_changes_what_the_pattern_renders() {
    let build = |school_chunk: &str| -> String {
        let dir = std::env::temp_dir().join(format!(
            "bug327-role-{}-{}",
            std::process::id(),
            school_chunk
        ));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("index.st"),
            format!(
                "@form style --school-quiet {{ font-size: 44px; }}\n\
                 @form style --school-loud  {{ font-size: 120px; }}\n\
                 @form style --role-title {{ --{school_chunk}; }}\n\
                 \n\
                 <h1 class=\"t\">T</h1>\n\
                 .t {{ --role-title; }}\n"
            ),
        )
        .expect("write page");
        let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
            .arg("build")
            .arg(&dir)
            .output()
            .expect("run build");
        assert!(
            out.status.success(),
            "build failed:\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let css = std::fs::read_to_string(dir.join("dist").join("spacetime.css"))
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        css
    };

    let quiet = build("school-quiet");
    let loud = build("school-loud");

    assert!(
        quiet.contains("44px"),
        "the quiet school's measure must reach the pattern:\n{quiet}"
    );
    assert!(
        loud.contains("120px"),
        "rebinding the ROLE must change what the pattern renders — with no edit \
         to the pattern selector:\n{loud}"
    );
    assert!(
        !loud.contains("44px"),
        "the old school's value must be GONE, not merged alongside:\n{loud}"
    );
}

/// A CSS custom property is spelled exactly like a splice plus a colon
/// (`--ground: #f8fafc;`). When the splice alternative was tried FIRST it
/// matched the `--ground` token, ignored the rest of the line, and swallowed
/// every declaration after it — a form of five values emitted one empty one.
/// The declaration arm must win whenever a colon follows; this pins that a form
/// can carry BOTH kinds of item, in either order.
#[test]
fn a_form_body_may_carry_custom_properties_and_a_splice() {
    let dir = std::env::temp_dir().join(format!("bug327-cp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(
        dir.join("index.st"),
        "@form style --shared { letter-spacing: -0.02em; }\n\
         @form style --vals {\n\
         \x20 --ground: #f8fafc;\n\
         \x20 --shared;\n\
         \x20 --ink: #0f172a;\n\
         \x20 color: red;\n\
         }\n\
         \n\
         <div class=\"x\">X</div>\n\
         .x { --vals; }\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        .output()
        .expect("run build");
    assert!(
        out.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let css = std::fs::read_to_string(dir.join("dist").join("spacetime.css")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        css.contains("--ground: #f8fafc"),
        "a custom property must keep its VALUE — an empty `--ground: ;` means the \
         splice arm ate the declaration:\n{css}"
    );
    assert!(
        css.contains("--ink: #0f172a"),
        "declarations AFTER the first must survive — losing them is the silent \
         truncation this grammar keeps re-learning:\n{css}"
    );
    assert!(
        css.contains("color: red"),
        "a plain declaration following a splice must survive too:\n{css}"
    );
    assert!(
        css.contains("letter-spacing: -0.02em"),
        "and the SPLICE in the middle must still expand — the fix must not be \
         'stop supporting splices':\n{css}"
    );
}
