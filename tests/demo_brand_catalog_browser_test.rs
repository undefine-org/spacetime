//! PLAN-139 W3: the brand catalog is BROWSABLE — and its rows carry the layer
//! a declaration came from.
//!
//! `@data forms` emits `{name, kind, params, body, doc, source, origin}`. The
//! catalog page narrows one unfiltered list with three signals (search + kind
//! rail + origin rail), so the count and the cards read the same rows and
//! cannot disagree.
//!
//! `origin` is the load-bearing new field: a reader of a 46-form list asks
//! "which of these are MINE" first, and `source` (a file path) answers that
//! only if you already know the tree. It is derived from the declaration's own
//! source file rather than from push order — the overlay's forms are merged
//! into `ast.matches` before the rows are built, so a push-order label would
//! call every `_prelude.st` form a page form (it did, until this was fixed).
//!
//! The BEHAVIOR half — typing narrows the grid, rails compose, reset restores —
//! is asserted in a real browser; this file pins the DATA contract that makes
//! it possible.

use std::process::Command;

fn build_catalog() -> (String, String) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join("demos").join("brand-catalog");
    let dir = std::env::temp_dir().join(format!("catbrowse-{}", std::process::id()));
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
        "catalog build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(dir.join("dist").join("index.html")).expect("read html");
    let css = std::fs::read_to_string(dir.join("dist").join("spacetime.css")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (html, css)
}

/// Every row carries an origin, and the two layers are told apart correctly.
/// The regression this pins: labelling by push order relabelled all 12 overlay
/// forms as `page`, so the "this brand" rail matched nothing at all.
#[test]
fn every_form_row_carries_the_layer_it_was_declared_in() {
    let (html, _) = build_catalog();

    let project = html.matches("fcard__origin\">project").count();
    let stdlib = html.matches("fcard__origin\">stdlib").count();

    assert!(
        project > 0,
        "the brand's own `_prelude.st` declarations must be labelled `project` — \
         zero here means origin is being read from push order rather than from \
         the declaration's source, which silently empties the \"this brand\" rail"
    );
    assert!(
        stdlib > 0,
        "the stdlib preset library must be labelled `stdlib` so a reader can \
         exclude it"
    );
    assert!(
        html.matches("fcard__origin\">").count() == project + stdlib,
        "no row may carry an origin outside the known layers"
    );
}

/// The browser is ONE list narrowed by signals, not six hand-maintained grids.
/// If a future edit reintroduces per-kind duplication, the count and the cards
/// can drift apart — which is the failure this shape exists to prevent.
#[test]
fn the_browser_renders_one_list_with_both_rails() {
    let (html, _) = build_catalog();

    assert!(
        html.contains("cards--all"),
        "the unified grid must be present"
    );
    for kind in ["style", "motion", "easing", "score", "value", "markup"] {
        assert!(
            html.contains(&format!("data-kind=\"{kind}\"")),
            "the kind rail must offer `{kind}`"
        );
    }
    for origin in ["all", "project", "stdlib", "page"] {
        assert!(
            html.contains(&format!("data-origin=\"{origin}\"")),
            "the origin rail must offer `{origin}`"
        );
    }
    assert!(
        html.contains("class=\"search\"") || html.contains("class=\"search "),
        "the search input must be present — it is the primary way in"
    );
}

/// The catalog is built out of the thing it catalogues: its own chrome splices
/// brand forms. A catalog styled by hand-written CSS would be a page that
/// merely TALKS about the design system.
#[test]
fn the_catalog_chrome_is_built_from_the_forms_it_lists() {
    let (_, css) = build_catalog();

    // `--role-title` -> `--aurora-display`: a two-hop composition (BUG-327),
    // so this also proves role indirection survives into a real page.
    assert!(
        css.contains("letter-spacing: -0.02em") || css.contains("letter-spacing:-0.02em"),
        "the browse heading splices `--role-title`, which binds `--aurora-display` \
         — its declarations must reach the stylesheet through BOTH hops:\n{css}"
    );
    // The rail chips splice `--brand-pill`.
    assert!(
        css.contains("999px"),
        "the rail chips splice `--brand-pill`; its pill radius must be emitted:\n{css}"
    );
}
