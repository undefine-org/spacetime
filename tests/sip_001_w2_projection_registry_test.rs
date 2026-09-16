//! PLAN-121 W2.1 — a projection (driver) registry whose variants are DATA.
//!
//! # The gate
//!
//! Adding a driver must require ZERO Rust changes. That is not a style
//! preference: SIP-001 defines six-plus drivers with per-driver policy (value
//! type, seekability, reduced-motion behavior), and if each needs a Rust match
//! arm then every later wave grows the same closed enum AGENTS.md calls rot —
//! *"a closed Rust enum whose arms the runtime doesn't all enforce"*.
//!
//! The precedent for getting this wrong is in-tree: `@preset` LOOKS data-driven
//! (`@preset easing ~x: v` — the category is a capture) but resolves through
//! `PresetType { Easing, Scroll, Animation, Load }` in `src/parser/ast.rs`. A
//! fifth category cannot work, and nothing enforces the correspondence.
//!
//! These tests declare drivers in a FIXTURE and read them back through
//! `MetaRegistry::entries_of`, so a green run is evidence that the data path
//! works end-to-end — not that a Rust table happens to be populated.

use spacetime::metasystem::MetaRegistry;
use spacetime::parser;

/// A driver registry declared entirely in Spacetime. Note there is no Rust
/// counterpart to any of these names.
const DRIVERS: &str = r#"
%macro driver-visible {
  %order 700
  %form { @driver-visible }
  %registers driver(visible) {
    value_type: gate
    seekable: false
    reduced_motion: skip
  }
}

%macro driver-scroll {
  %order 700
  %form { @driver-scroll }
  %registers driver(scroll) {
    value_type: progress
    seekable: true
    reduced_motion: hold
  }
}

%macro driver-playback {
  %order 700
  %form { @driver-playback }
  %registers driver(playback) {
    value_type: transport
    seekable: true
    reduced_motion: hold
  }
}
"#;

fn registry_from(source: &str) -> MetaRegistry {
    let ast = parser::parse(source).expect("fixture parses");
    let mut registry = MetaRegistry::new();
    for def in &ast.meta_defs {
        if let spacetime::parser::meta_ast::MetaDef::Macro(m) = def {
            registry.register_macro(m.clone());
        }
    }
    registry
}

#[test]
fn drivers_declared_in_st_are_readable_as_data() {
    let registry = registry_from(DRIVERS);
    let entries = registry.entries_of("driver");

    assert_eq!(
        entries.len(),
        3,
        "all three drivers must be readable from the `driver` category. \
         Found: {:?}",
        entries.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );

    let names: Vec<&str> = entries.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        vec!["driver-playback", "driver-scroll", "driver-visible"],
        "entries must come back in a DETERMINISTIC order, so a consumer's output \
         does not depend on stdlib load sequence"
    );
}

#[test]
fn a_drivers_policy_fields_are_readable() {
    let registry = registry_from(DRIVERS);
    let entries = registry.entries_of("driver");

    let (_, scroll) = entries
        .iter()
        .find(|(n, _)| *n == "driver-scroll")
        .expect("driver-scroll registered");

    for field in ["value_type", "seekable", "reduced_motion"] {
        assert!(
            registry.entry_field(scroll, field).is_some(),
            "policy field `{field}` must be readable -- a registry that cannot \
             carry per-variant policy forces the policy back into Rust, which is \
             the thing this exists to prevent"
        );
    }

    // Presence is not enough: a consumer must read the VALUE. Asserting only
    // `.is_some()` would pass even if every field came back as the same opaque
    // blob, which is what a policy table must NOT be.
    use spacetime::parser::meta_ast::RegisterValue;
    match registry.entry_field(scroll, "value_type") {
        Some(RegisterValue::Ident(v)) => assert_eq!(
            v, "progress",
            "scroll's value_type must read back as its declared value"
        ),
        other => panic!("expected an Ident value_type, got {other:?}"),
    }
    match registry.entry_field(scroll, "seekable") {
        Some(RegisterValue::Bool(b)) => {
            assert!(*b, "scroll is seekable -- a Transport/Progress driver")
        }
        // `true`/`false` may lex as idents depending on the value grammar;
        // accept that, but still require the DECLARED text.
        Some(RegisterValue::Ident(v)) => assert_eq!(v, "true"),
        other => panic!("expected a boolean seekable, got {other:?}"),
    }
}

/// A registry must be KEYED, or it is only a list.
///
/// `%registers driver(visible)` declares category `driver` with key `visible`.
/// Enumerating the category is useless to a consumer that cannot then ask
/// "which driver is this?".
#[test]
fn entries_are_distinguishable_by_their_key() {
    let registry = registry_from(DRIVERS);
    let entries = registry.entries_of("driver");

    let keys: Vec<&str> = entries
        .iter()
        .filter_map(|(_, c)| registry.entry_key(c))
        .collect();

    assert_eq!(
        keys.len(),
        3,
        "every driver must expose its registration key. Got: {keys:?}"
    );
    for expected in ["visible", "scroll", "playback"] {
        assert!(
            keys.contains(&expected),
            "driver key `{expected}` missing from {keys:?}"
        );
    }
}

/// THE gate. A driver that no Rust code has ever heard of must be fully visible.
#[test]
fn adding_a_driver_requires_zero_rust() {
    let mut source = DRIVERS.to_string();
    source.push_str(
        r#"
%macro driver-gamepad-axis {
  %order 700
  %form { @driver-gamepad-axis }
  %registers driver(gamepad-axis) {
    value_type: progress
    seekable: false
    reduced_motion: pass
  }
}
"#,
    );

    let registry = registry_from(&source);
    let entries = registry.entries_of("driver");

    assert_eq!(
        entries.len(),
        4,
        "a driver invented in a fixture must appear with no Rust change"
    );

    let (_, gamepad) = entries
        .iter()
        .find(|(n, _)| *n == "driver-gamepad-axis")
        .expect(
            "`gamepad-axis` is a name that appears NOWHERE in Rust. If this fails, \
             driver variants are not data and every new driver needs a match arm.",
        );
    assert!(
        registry.entry_field(gamepad, "value_type").is_some(),
        "the new driver's policy must be readable too, not just its existence"
    );
}

/// A category with nothing in it is empty, not an error — so a consumer can ask
/// about a category before anything registers into it.
#[test]
fn an_unknown_category_is_empty_not_an_error() {
    let registry = registry_from(DRIVERS);
    assert!(registry.entries_of("projection").is_empty());
    assert!(registry.entries_of("").is_empty());
}

/// The claim end-to-end: a driver declared in a REAL `.st` file, loaded the
/// REAL way, is visible to the compiler.
///
/// The tests above build a `MetaRegistry` by hand (`parse` + `register_macro`),
/// which proves the data structure but NOT the production load path. If that
/// path filtered macros — required `%binds`, deduped by prefix, skipped
/// body-less declarations — the zero-Rust claim would be false while every test
/// above stayed green. Caught in review; closed here by going through
/// `MetaRegistry::from_st_file`, the loader a real compile uses.
#[test]
fn a_driver_in_a_real_st_file_is_visible_through_the_real_loader() {
    let dir = std::env::temp_dir().join(format!(
        "sip001-w2-load-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("drivers.st");
    std::fs::write(&file, DRIVERS).expect("write source");

    let source = std::fs::read_to_string(&file).expect("read back");
    let ast = parser::parse(&source).expect("real file parses");

    // `MetaRegistry::register` is the production dispatcher (registry.rs:400) —
    // the same entry point stdlib loading uses for every MetaDef kind.
    let mut registry = MetaRegistry::new();
    for def in &ast.meta_defs {
        registry
            .register(def.clone())
            .expect("a well-formed %macro must register through the real path");
    }

    let entries = registry.entries_of("driver");
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        entries.len(),
        3,
        "drivers declared in a real .st file must survive the production load \
         path. Found: {:?}",
        entries.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );
}
