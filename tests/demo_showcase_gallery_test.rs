//! The showcase tree gets a front door.
//!
//! `stdlib/showcases/` ships 11 curated patterns and compiles to ZERO pages —
//! `index.st` is a pure import manifest, so the library that exists to be
//! LOOKED AT could not be looked at. Every pattern is a `@template`, and until
//! now not one of them had a single call site anywhere in the repo: they were
//! authored, restyled (PLAN-139 W2), documented in CATALOG.md, and never once
//! instantiated. A pattern nobody can render is a pattern nobody can trust.
//!
//! This gallery instantiates all of them on one page, so the tree is exercised
//! by the same act that displays it. If a pattern stops compiling, this test
//! goes red — which is the coverage the showcase tree never had.

use std::path::PathBuf;
use std::process::Command;

fn demo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("demos/showcase-gallery")
}

fn build() -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(demo_dir())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(demo_dir().join("dist/index.html")).unwrap_or_default();
    (out.status.success() && !log.contains("Export failed"), log, html)
}

/// Every curated pattern renders. Named individually so a break names its own
/// pattern instead of reporting "the gallery is broken".
#[test]
fn every_curated_pattern_renders() {
    let (ok, log, html) = build();
    assert!(ok, "the gallery must build:\n{log}");

    // An unresolved `&template(...)` invocation renders as LITERAL TEXT —
    // `&amp;showcase-hero-minimal("…")` — and the build still reports success.
    // So a substring search for the class name passes on the source of a
    // pattern that never rendered. Assert the ELEMENT (`class="…"`), and
    // refuse the page outright if any invocation leaked through as text.
    assert!(
        !html.contains("&amp;showcase-"),
        "a template invocation reached the page as literal text — the pattern \
         did not render and the build did not say so (see BUG-344)"
    );

    // One marker per pattern: a class only that pattern's markup emits.
    let patterns = [
        ("hero/minimal", "showcase-hero-min"),
        ("hero/editorial", "showcase-hero-ed"),
        ("hero/maximal", "showcase-hero-max"),
        ("hero/architecture", "showcase-hero-arch"),
        ("hero/motion", "showcase-hero-mtn"),
        ("hero/eastern", "showcase-hero-east"),
        ("nav/minimal", "showcase-nav-min"),
        ("cta/minimal", "showcase-cta-min"),
        ("footer/minimal", "showcase-footer-min"),
        ("features-grid/minimal", "showcase-features-min"),
    ];
    let missing: Vec<&str> = patterns
        .iter()
        .filter(|(_, cls)| !html.contains(&format!("class=\"{cls}\"")))
        .map(|(p, _)| *p)
        .collect();
    assert!(
        missing.is_empty(),
        "these curated patterns did not reach the page: {missing:?}\n\
         Each is a `@template` in stdlib/showcases; the gallery is the only \
         place they are instantiated, so a missing one means the pattern is \
         broken and nothing else would have caught it."
    );
}

/// The gallery must show the pattern's own metadata beside it — a specimen
/// without its provenance is decoration, not a catalog.
#[test]
fn each_specimen_carries_its_metadata() {
    let (ok, log, html) = build();
    assert!(ok, "build:\n{log}");

    for needle in [
        "information-architecture",
        "motion-poetics",
        "eastern-philosophy",
        "experimental",
        "minimalism",
    ] {
        assert!(
            html.contains(needle),
            "school `{needle}` must be shown — the 5-school taxonomy is what \
             makes the tree navigable rather than a pile of heroes"
        );
    }
    assert!(
        html.matches("spec__use").count() >= 10,
        "every specimen needs its `useWhen` guidance rendered: the catalog's \
         job is to answer WHEN to reach for a pattern, not just show it"
    );
}

/// The gallery filters by school without JavaScript — the same registry-driven
/// browse the brand catalog proved, applied to patterns instead of forms.
#[test]
fn the_gallery_filters_by_school() {
    let (ok, log, html) = build();
    assert!(ok, "build:\n{log}");
    assert!(
        html.contains("class=\"chip chip--school\"") || html.contains("chip--school"),
        "a school rail must exist so the tree can be browsed by design language"
    );
    assert!(
        html.contains("data-school"),
        "each specimen must carry its school as data, so filtering is a \
         property of the row and not a hand-maintained list"
    );
}

/// Zero hand-written JavaScript. The house rule for every Spacetime page, and
/// the whole point of showing patterns in the language they are written in.
#[test]
fn the_gallery_writes_no_javascript() {
    let dir = demo_dir();
    for entry in std::fs::read_dir(&dir).expect("read demo dir").flatten() {
        let p = entry.path();
        assert!(
            p.extension().and_then(|e| e.to_str()) != Some("js"),
            "no hand-written JS may live in the demo: {}",
            p.display()
        );
    }
    let (ok, log, html) = build();
    assert!(ok, "build:\n{log}");
    assert!(
        !html.contains("<script>function") && !html.contains("addEventListener("),
        "the page must not carry hand-authored inline script"
    );
}

/// THE POINT OF W3. The specimen list is GENERATED from the tree, so adding a
/// pattern file makes a specimen appear with no page edit. A hand-maintained
/// list is exactly the index `@data declarations` exists to abolish — so the
/// acceptance test is the property, not the current count.
#[test]
fn a_new_pattern_appears_with_no_page_edit() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let before = {
        let (ok, log, html) = build();
        assert!(ok, "baseline build:\n{log}");
        html.matches("class=\"spec\"").count()
    };

    // Add a pattern to the tree and REGISTER it in the tree's own manifest
    // (`stdlib/showcases/index.st`). There are no glob imports, so that one
    // line is the irreducible act of publishing a pattern — it is how the tree
    // says "this is part of the library".
    //
    // What must NOT be needed is a GALLERY edit: no new `<section>`, no entry
    // in a list of specimens, no filter change. The page is generated from the
    // declarations, so publishing a pattern is the whole job.
    let manifest_path = root.join("stdlib/showcases/index.st");
    let manifest_before = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let gallery_path = root.join("demos/showcase-gallery/index.st");
    let gallery_before = std::fs::read_to_string(&gallery_path).expect("read gallery");

    let new_pattern = root.join("stdlib/showcases/cta/generated-probe.st");
    std::fs::write(
        &new_pattern,
        "/// scenario: cta\n\
         /// style: minimal\n\
         /// school: experimental\n\
         /// description: A probe pattern added by a test to prove the gallery generates itself.\n\
         /// dependencies: none\n\
         /// screenshot: (pending)\n\
         /// useWhen: Never — this exists only to prove no page edit is needed.\n\
         \n\
         @template &showcase-probe-generated($label) {\n    \
             <section class=\"showcase-probe-gen\">`$label`</section>\n\
         }\n\
         \n\
         .showcase-probe-gen { padding: 24px; }\n",
    )
    .expect("write probe pattern");
    std::fs::write(
        &manifest_path,
        format!("{manifest_before}@import \"./cta/generated-probe.st\"\n"),
    )
    .expect("register in manifest");

    let (ok, log, html) = build();
    let after = html.matches("class=\"spec\"").count();
    let names_it = html.contains("showcase-probe-generated");

    // Restore the tree BEFORE asserting, so a failure never leaves the repo dirty.
    let _ = std::fs::remove_file(&new_pattern);
    std::fs::write(&manifest_path, &manifest_before).expect("restore manifest");
    let gallery_after = std::fs::read_to_string(&gallery_path).expect("read gallery");

    assert_eq!(
        gallery_before, gallery_after,
        "the gallery page must not have been touched — it generates its \
         specimens from the declarations"
    );
    assert!(ok, "the gallery must build with the new pattern:\n{log}");
    assert_eq!(
        after,
        before + 1,
        "publishing a pattern (a file + its manifest line) must add a specimen \
         with NO gallery edit — that is the whole reason the list is generated \
         rather than hand-written"
    );
    assert!(
        names_it,
        "the new pattern must be named on the page, from its own declaration"
    );
}

/// A specimen must be shown WHOLE. Patterns declare `min-height: 100vh` and
/// are composed for a real viewport, so a fixed frame with `overflow: hidden`
/// shows their top-left corner — a 112px display headline crops to three
/// letters, which is what a user reported seeing. The frame now holds a
/// full-size stage that is SCALED, so the pattern still lays out at 1440x900
/// and the viewer sees all of it.
///
/// This asserts the mechanism is present and coherent; the no-crop property
/// itself is measured in a real browser (a computed-layout claim cannot be
/// made from source, and asserting the CSS text would prove nothing).
#[test]
fn a_specimen_is_scaled_rather_than_cropped() {
    let (ok, log, html) = build();
    assert!(ok, "build:\n{log}");
    assert!(
        html.matches("spec__stage").count() >= 13,
        "every specimen needs a stage — without one the pattern lays out at \
         frame width and is cropped instead of scaled:\n{log}"
    );
    let prelude = std::fs::read_to_string(demo_dir().join("_prelude.st")).expect("prelude");
    assert!(
        prelude.contains("transform: scale(") && prelude.contains("width: 1440px"),
        "the stage must lay out at a real desktop width and scale down; a \
         percentage-width stage would reflow the pattern instead of previewing it"
    );
}

/// The interactive patterns must actually be interactive: each declares a
/// signal, writes it from an event, and has more than one reader. A pattern
/// that merely LOOKS like a control is worse than no pattern — the whole point
/// of adding these was to show behaviour, not composition.
#[test]
fn the_interactive_patterns_declare_real_behaviour() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (file, signal, readers) in [
        ("stdlib/showcases/pricing/interactive.st", "$annual", 5),
        ("stdlib/showcases/features-grid/tabbed.st", "$panel", 5),
    ] {
        let src = std::fs::read_to_string(root.join(file)).expect(file);
        assert!(
            src.contains(&format!("{signal} <-")),
            "{file} must WRITE {signal} from an event — otherwise the control \
             is decoration"
        );
        let reads = src.matches(signal).count();
        assert!(
            reads >= readers,
            "{file}: {signal} should drive several consequences (found {reads} \
             mentions, want >= {readers}). One signal with many readers is the \
             thing these patterns exist to demonstrate."
        );
    }
    let mag = std::fs::read_to_string(root.join("stdlib/showcases/cta/magnetic.st")).expect("mag");
    assert!(
        mag.contains("@magnetic(") && mag.contains("@reveal("),
        "the magnetic CTA must declare both pointer physics and text \
         choreography — that pairing is what it is for"
    );
}
