//! Fidelity ladder (PLAN-027 W1).
//!
//! A Spacetime test declares — or has inferred — the *fidelity rung* it needs.
//! A backend that cannot provide that rung MUST refuse to run the test (report
//! FAILED with a clear message) rather than execute it against faked browser
//! primitives (e.g. `getComputedStyle = el.style || {}`, synchronous rAF) and
//! report a meaningless green. This permanently closes the silent-false-green
//! class (BUG-051 / BUG-053 family): a layout assertion can no longer pass on a
//! layout-less engine.
//!
//! The ladder is a total order. A test's *effective* rung is
//! `max(inferred_floor, declared)` and a backend runs it iff
//! `effective <= backend_max`.
//!
//! | rung   | world                          | backend            |
//! |--------|--------------------------------|--------------------|
//! | Pure   | no DOM, no time                | V8                 |
//! | Logic  | LinkeDOM + fake rAF            | V8 + LinkeDOM      |
//! | Layout | real CSSOM / getComputedStyle | CDP (W2)           |
//! | Timing | real / seekable clock         | CDP (W2/W5)        |
//! | Paint  | pixels                         | CDP + screenshot   |
//!
//! `%emit build-js` (i18n/locale) is the Pure/Logic rung used at *build* time —
//! the same V8 embed, a different caller. It needs no DOM and no real timing, so
//! it sits at the bottom of the ladder and is unaffected by the gate.

use std::fmt;

/// A fidelity rung. Ordered: `Pure < Logic < Layout < Timing < Paint`.
///
/// `derive(PartialOrd, Ord)` over the declaration order gives the ladder
/// comparison for free — keep the variants in ascending-fidelity order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rung {
    /// No DOM, no time. Pure value/logic (stagger math, `take`, `fold`). V8.
    Pure,
    /// LinkeDOM + fake rAF. Reactivity, signals, event wiring, state graph. V8.
    Logic,
    /// Real CSSOM / `getComputedStyle`. `@balance`, pretext reflow, visibility,
    /// dimensions. Requires a real browser (CDP, W2).
    Layout,
    /// Real or seekable clock. Reveal stagger, easing, transition timing
    /// (W5 virtual clock). Requires a real browser.
    Timing,
    /// Pixels. Visual regression, screenshot diff. Requires a real browser.
    Paint,
}

impl Rung {
    /// The lowest rung. Default floor when nothing else is inferred.
    pub const MIN: Rung = Rung::Pure;

    /// Parse a rung from its lowercase token (`needs <rung>` / `--rung <rung>`).
    pub fn parse(s: &str) -> Option<Rung> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pure" => Some(Rung::Pure),
            "logic" => Some(Rung::Logic),
            "layout" => Some(Rung::Layout),
            "timing" => Some(Rung::Timing),
            "paint" => Some(Rung::Paint),
            _ => None,
        }
    }

    /// Canonical lowercase token.
    pub fn as_str(self) -> &'static str {
        match self {
            Rung::Pure => "pure",
            Rung::Logic => "logic",
            Rung::Layout => "layout",
            Rung::Timing => "timing",
            Rung::Paint => "paint",
        }
    }

    /// All rungs, ascending. Single source of truth for the JS runtime order.
    pub const ALL: [Rung; 5] = [
        Rung::Pure,
        Rung::Logic,
        Rung::Layout,
        Rung::Timing,
        Rung::Paint,
    ];
}

impl fmt::Display for Rung {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The minimum rung an assertion / probe requires.
///
/// This is the *inference* table referenced by FEAT-057: an assertion's
/// vocabulary determines the floor. It is intentionally data — one table, not
/// conditionals scattered through emit — so W3's claim grammar can extend it in
/// one place. Keys are the `@then … should <assertion>` kinds (and a few probe
/// names); anything not listed defaults to [`Rung::Logic`] (needs a DOM but no
/// real layout/timing).
///
/// Rationale for the Layout entries: each of these reads a *rendered* property
/// that LinkeDOM fakes (`getComputedStyle`→`el.style||{}`, `offsetWidth/Height`,
/// `offsetParent`), so on the V8 backend they would silently pass against a lie.
pub fn assertion_min_rung(assertion: &str) -> Rung {
    match assertion {
        // Layout: rendered geometry / computed style / visibility.
        "be_visible" | "be_hidden" | "have_style" | "have_width" | "have_height" => Rung::Layout,
        // Everything else (existence, text, class, value, state, attrs, length,
        // contains, checked/disabled/focused…) is DOM-structural → Logic.
        _ => Rung::Logic,
    }
}

/// A probe/directive's minimum rung (for non-`@then` directives that imply a
/// fidelity floor). Unknown → [`Rung::Logic`] (most directives just need a DOM).
pub fn directive_min_rung(directive: &str) -> Rung {
    match directive {
        // Timing-coupled directives (W5 fills these in with the virtual clock).
        "clock" | "record-timeline" => Rung::Timing,
        // Pixel capture.
        "capture" | "screenshot" => Rung::Paint,
        _ => Rung::Logic,
    }
}

/// The effective rung of a test: the higher of the inferred floor and any
/// explicit `needs` declaration. `needs` may only *raise* the floor.
///
/// Returns `Err` if `declared` is *below* the inferred floor — lowering is a
/// hard error (you cannot claim a layout test only needs logic).
pub fn effective_rung(inferred_floor: Rung, declared: Option<Rung>) -> Result<Rung, RungError> {
    match declared {
        None => Ok(inferred_floor),
        Some(d) if d >= inferred_floor => Ok(d),
        Some(d) => Err(RungError::LowersFloor {
            declared: d,
            floor: inferred_floor,
        }),
    }
}

/// Errors from rung resolution / the refuse-to-fake gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RungError {
    /// `needs <d>` is below the inferred floor `<floor>`.
    LowersFloor { declared: Rung, floor: Rung },
    /// The test's effective rung exceeds the backend's max — the backend must
    /// refuse rather than fake it.
    ExceedsBackend { effective: Rung, backend_max: Rung },
}

impl fmt::Display for RungError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RungError::LowersFloor { declared, floor } => write!(
                f,
                "`needs {declared}` is below this test's inferred floor `{floor}`; \
                 `needs` can only raise fidelity, not lower it"
            ),
            RungError::ExceedsBackend {
                effective,
                backend_max,
            } => write!(
                f,
                "test needs `{effective}` fidelity but the current backend tops out at \
                 `{backend_max}` — run it on a higher-fidelity backend (e.g. CDP) instead \
                 of faking `{effective}`"
            ),
        }
    }
}

/// Does `backend_max` satisfy `effective`? (The gate predicate.)
pub fn backend_satisfies(backend_max: Rung, effective: Rung) -> bool {
    effective <= backend_max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_is_ordered() {
        assert!(Rung::Pure < Rung::Logic);
        assert!(Rung::Logic < Rung::Layout);
        assert!(Rung::Layout < Rung::Timing);
        assert!(Rung::Timing < Rung::Paint);
    }

    #[test]
    fn parse_roundtrip() {
        for r in Rung::ALL {
            assert_eq!(Rung::parse(r.as_str()), Some(r));
        }
        assert_eq!(Rung::parse("LAYOUT"), Some(Rung::Layout));
        assert_eq!(Rung::parse(" timing "), Some(Rung::Timing));
        assert_eq!(Rung::parse("nonsense"), None);
    }

    #[test]
    fn layout_assertions_floor_at_layout() {
        for a in [
            "be_visible",
            "be_hidden",
            "have_style",
            "have_width",
            "have_height",
        ] {
            assert_eq!(
                assertion_min_rung(a),
                Rung::Layout,
                "{a} should need layout"
            );
        }
        for a in [
            "exist",
            "have_text",
            "have_class",
            "have_state",
            "have_length",
        ] {
            assert_eq!(assertion_min_rung(a), Rung::Logic, "{a} should be logic");
        }
    }

    #[test]
    fn effective_takes_the_max() {
        // No declaration → floor.
        assert_eq!(effective_rung(Rung::Logic, None), Ok(Rung::Logic));
        // Declared above floor → declared.
        assert_eq!(
            effective_rung(Rung::Logic, Some(Rung::Layout)),
            Ok(Rung::Layout)
        );
        // Declared equal → ok.
        assert_eq!(
            effective_rung(Rung::Layout, Some(Rung::Layout)),
            Ok(Rung::Layout)
        );
        // Declared below floor → error (cannot lower).
        assert_eq!(
            effective_rung(Rung::Layout, Some(Rung::Logic)),
            Err(RungError::LowersFloor {
                declared: Rung::Logic,
                floor: Rung::Layout
            })
        );
    }

    #[test]
    fn gate_predicate() {
        // Logic backend cannot satisfy a layout test.
        assert!(!backend_satisfies(Rung::Logic, Rung::Layout));
        // Logic backend satisfies logic + pure.
        assert!(backend_satisfies(Rung::Logic, Rung::Logic));
        assert!(backend_satisfies(Rung::Logic, Rung::Pure));
        // A CDP-class backend satisfies everything up to its max.
        assert!(backend_satisfies(Rung::Paint, Rung::Timing));
    }
}
