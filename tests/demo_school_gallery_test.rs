//! PLAN-139 W4: one markup tree, five design languages, switched live.
//!
//! `demos/school-gallery` is the payoff of the role indirection W2 unlocked. A
//! pattern splices ROLES (`--role-title`, `--role-surface`); a school answers
//! what those roles mean. Compile-time that makes switching one edit; this demo
//! makes it live, by having the roles read CSS custom properties and binding a
//! `data-school` attribute to a signal.
//!
//! WHY BOTH LAYERS ARE TESTED HERE. The interesting failure is not "the CSS is
//! wrong" — it is "the page renders identically in every school", which a
//! screenshot-free suite will happily call green. So these gates assert that
//! the schools' answers DIFFER, not merely that they exist.
//!
//! The behavior half (clicking a switch actually repaints the page) is asserted
//! in a real browser: measured minimalism → experimental → architecture →
//! motion → eastern giving five distinct grounds, families, display sizes and
//! action treatments.

use std::process::Command;

fn build_gallery() -> (String, String) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("demos").join("school-gallery");
    let dir = std::env::temp_dir().join(format!("gallery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    for f in ["index.st", "_prelude.st"] {
        std::fs::copy(src.join(f), dir.join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        .output()
        .expect("run build");
    assert!(
        out.status.success(),
        "gallery build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(dir.join("dist").join("index.html")).expect("html");
    let css = std::fs::read_to_string(dir.join("dist").join("spacetime.css")).expect("css");
    let _ = std::fs::remove_dir_all(&dir);
    (html, css)
}

const SCHOOLS: [&str; 5] = [
    "minimalism",
    "experimental",
    "architecture",
    "motion",
    "eastern",
];

/// Each school's value block reaches the stylesheet under its own attribute
/// selector. This is the switch's whole mechanism: no block, no school.
#[test]
fn every_school_emits_a_value_block() {
    let (_, css) = build_gallery();
    for school in SCHOOLS {
        assert!(
            css.contains(&format!("[data-school=\"{school}\"]")),
            "`{school}` must bind its values under its own attribute selector:\n{css}"
        );
    }
}

/// The regression that made this demo look broken on first run: a form body of
/// CUSTOM PROPERTIES parsed as one empty declaration, because the splice
/// alternative in the `properties` grammar matched `--ground` and swallowed the
/// rest of the line. Every school rendered as unstyled browser default.
///
/// So: the values must be PRESENT, and they must DIFFER between schools.
/// Asserting only presence would pass against a page where all five blocks
/// carry the same values, which is the same page five times.
#[test]
fn the_schools_answer_the_roles_differently() {
    let (_, css) = build_gallery();

    assert!(
        !css.contains("--ground: ;"),
        "an empty custom property means the form body was truncated — every \
         school would render as unstyled default:\n{css}"
    );

    // Grounds: five schools, and no two of these may coincide.
    for ground in ["#f8fafc", "#0b0b10", "#ffffff", "#0a0a0c", "#f7f3ec"] {
        assert!(
            css.contains(ground),
            "the school grounds must all reach the stylesheet — missing `{ground}`"
        );
    }
    // Display voices that could not be confused for one another.
    assert!(
        css.contains("uppercase"),
        "the experimental school shouts in uppercase; without it the display \
         voices are not really distinct"
    );
    assert!(
        css.contains("EB Garamond") && css.contains("Georgia"),
        "the eastern and architecture schools are serif-led — their families \
         must survive"
    );
    // Action treatments: a pill, a squared block, and a borderless underline.
    assert!(
        css.contains("999px") && css.contains("--cta-radius: 0px"),
        "a school must be able to say `pill` and another `square`:\n{css}"
    );
    assert!(
        css.contains("underline"),
        "the eastern school's action is an underlined phrase, not a button — a \
         school that cannot refuse button chrome is only a colour scheme"
    );
}

/// The point of the exercise: the markup is written ONCE. If a future edit
/// starts branching the tree per school, the demo stops demonstrating anything.
#[test]
fn the_markup_is_written_once_for_all_five_schools() {
    let (html, _) = build_gallery();

    assert_eq!(
        html.matches("hero__title").count(),
        1,
        "the hero title must appear exactly once in the document — more than \
         one means the page is carrying a per-school copy, which is the thing \
         roles exist to avoid"
    );
    for school in SCHOOLS {
        assert!(
            html.contains(&format!("data-school=\"{school}\"")),
            "the switcher must offer `{school}`"
        );
    }
}

/// Layout must not carry brand decisions. A colour or family written at the
/// layout level cannot be answered by a school, so it silently survives the
/// switch — the exact leak that had the switcher chips rendering in the wrong
/// family until the schools were given `--text-family`.
#[test]
fn layout_rules_defer_brand_decisions_to_the_roles() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = std::fs::read_to_string(root.join("demos/school-gallery/index.st")).expect("read");

    // The layout half starts after the markup; scan it for hardcoded hex.
    let layout = page
        .split("LAYOUT — the only thing that does NOT change")
        .nth(1)
        .expect("layout section marker");

    assert!(
        !layout.contains('#'),
        "no hex colour may be written in the layout section — a colour there \
         cannot be answered by a school and will survive every switch:\n{layout}"
    );
}
