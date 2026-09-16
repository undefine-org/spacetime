//! PLAN-124 W3.1 — the stdlib DRIVER REGISTRY is data, enumerable from Rust
//! with zero Rust knowledge of any driver name.
//!
//! `@on <driver>` resolves its member through `entries_of("driver")` (W2.1).
//! These gates prove the registry stdlib ships (stdlib/macros/drivers.st)
//! declares the SIP-001 projection table, with the fields the @on resolution
//! and the reduced-motion/render policies read — through the PRODUCTION
//! registration path, never a hand-built one.

use spacetime::metasystem::MetaRegistry;
use spacetime::parser::meta_ast::{RegisterValue, RegistersClause};

/// The SIP-001 §2 projection table, as data this test owns. A driver added to
/// drivers.st without updating this table fails — the table is the contract.
const EXPECTED: &[(&str, &str, &str, bool, &str, &str)] = &[
    // (key, primitive, value_type, seekable, reduced_motion, on_kinds)
    ("visible", "intersection", "progress", false, "skip", "elem"),
    ("hover", "event-driver", "event", false, "allow", "elem"),
    ("click", "event-driver", "event", false, "allow", "elem"),
    ("focus", "event-driver", "event", false, "allow", "elem"),
    ("submit", "event-driver", "event", false, "allow", "elem"),
    ("key", "event-driver", "event", false, "allow", "elem"),
    (
        "scroll",
        "scroll-driver",
        "progress",
        false,
        "allow",
        "elem",
    ),
    ("mouse", "mouse-driver", "progress", false, "allow", "elem"),
    (
        "pointer",
        "mouse-driver",
        "progress",
        false,
        "allow",
        "elem",
    ),
    ("time", "time-driver", "transport", true, "skip", "elem"),
    ("loop", "loop-driver", "transport", true, "hold", "elem"),
    ("load", "load-driver", "event", false, "skip", "elem"),
    (
        "steps",
        "planned:steps-driver",
        "transport",
        true,
        "skip",
        "elem",
    ),
    (
        "playback",
        "planned:media-driver",
        "transport",
        true,
        "hold",
        "media",
    ),
    (
        "clip",
        "planned:clip-driver",
        "transport",
        true,
        "skip",
        "clip",
    ),
    ("change", "change-driver", "event", false, "allow", "signal"),
    (
        "text-change",
        "text-change-driver",
        "event",
        false,
        "allow",
        "elem",
    ),
    (
        "running",
        "planned:running-driver",
        "bool",
        false,
        "allow",
        "signal",
    ),
];

/// PLAN-126 VALUE FACETS (D26): typed READS, not drivers — no `primitive`
/// field (the body position rejects them as E0950; statement/arms positions
/// lower them). (key, lowers_to?, facet_primitive?, value_type, on_kinds)
const EXPECTED_FACETS: &[(&str, Option<&str>, Option<&str>, &str, &str)] = &[
    ("done", Some("($%subject >= 1)"), None, "bool", "signal"),
    ("prev", None, Some("prev-signal"), "value", "signal"),
];

/// The production stdlib MetaRegistry (the same one the compiler caches).
fn stdlib_registry() -> &'static MetaRegistry {
    use std::sync::OnceLock;
    static REG: OnceLock<MetaRegistry> = OnceLock::new();
    REG.get_or_init(|| spacetime::compiler::cached_stdlib_registry().0)
}

/// (key, clause) pairs from the stdlib registry, keyed via entry_key — the
/// entry NAME is the macro name (`driver-visible`); the DRIVER name is the key.
fn stdlib_drivers() -> Vec<(String, RegistersClause)> {
    stdlib_registry()
        .entries_of("driver")
        .into_iter()
        .map(|(_, clause)| {
            (
                stdlib_registry()
                    .entry_key(clause)
                    .expect("driver entry keyed")
                    .to_string(),
                clause.clone(),
            )
        })
        .collect()
}

fn ident_field(reg: &MetaRegistry, clause: &RegistersClause, field: &str) -> String {
    match reg.entry_field(clause, field) {
        Some(RegisterValue::Ident(s)) => s.clone(),
        other => panic!("field `{field}` must be an Ident, got {other:?}"),
    }
}

fn bool_field(reg: &MetaRegistry, clause: &RegistersClause, field: &str) -> bool {
    match reg.entry_field(clause, field) {
        Some(RegisterValue::Bool(b)) => *b,
        other => panic!("field `{field}` must be a Bool, got {other:?}"),
    }
}

#[test]
fn the_registry_declares_the_sip_projection_table() {
    let mut actual: Vec<String> = stdlib_drivers().into_iter().map(|(k, _)| k).collect();
    actual.sort();
    let mut expected: Vec<&str> = EXPECTED
        .iter()
        .map(|e| e.0)
        .chain(EXPECTED_FACETS.iter().map(|e| e.0))
        .collect();
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "the driver registry must declare exactly the SIP-001 projection table"
    );
}

#[test]
fn every_entry_carries_its_policy_as_typed_data() {
    let reg = stdlib_registry();
    let entries = stdlib_drivers();
    for (key, primitive, value_type, seekable, reduced_motion, on_kinds) in EXPECTED {
        let (_, clause) = entries
            .iter()
            .find(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("driver `{key}` registered"));
        assert_eq!(
            &ident_field(reg, clause, "primitive"),
            primitive,
            "{key}.primitive"
        );
        assert_eq!(
            &ident_field(reg, clause, "value_type"),
            value_type,
            "{key}.value_type"
        );
        assert_eq!(
            bool_field(reg, clause, "seekable"),
            *seekable,
            "{key}.seekable"
        );
        assert_eq!(
            &ident_field(reg, clause, "reduced_motion"),
            reduced_motion,
            "{key}.reduced_motion"
        );
        assert_eq!(
            &ident_field(reg, clause, "on_kinds"),
            on_kinds,
            "{key}.on_kinds"
        );
    }
    for (key, lowers_to, facet_primitive, value_type, on_kinds) in EXPECTED_FACETS {
        let (_, clause) = entries
            .iter()
            .find(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("facet `{key}` registered"));
        // A value facet carries NO primitive (it is not a driver) — the
        // lowering fields are its dispatch, as data.
        assert!(
            reg.entry_field(clause, "primitive").is_none(),
            "{key}.primitive must be ABSENT — a value facet is not a driver"
        );
        match lowers_to {
            Some(t) => match reg.entry_field(clause, "lowers_to") {
                Some(RegisterValue::String(actual)) => assert_eq!(actual, t, "{key}.lowers_to"),
                other => panic!("{key}.lowers_to must be a String, got {other:?}"),
            },
            None => assert!(
                reg.entry_field(clause, "lowers_to").is_none(),
                "{key}.lowers_to must be absent"
            ),
        }
        match facet_primitive {
            Some(p) => assert_eq!(
                &ident_field(reg, clause, "facet_primitive"),
                p,
                "{key}.facet_primitive"
            ),
            None => assert!(
                reg.entry_field(clause, "facet_primitive").is_none(),
                "{key}.facet_primitive must be absent"
            ),
        }
        assert_eq!(
            &ident_field(reg, clause, "value_type"),
            value_type,
            "{key}.value_type"
        );
        assert_eq!(
            &ident_field(reg, clause, "on_kinds"),
            on_kinds,
            "{key}.on_kinds"
        );
    }
}

/// The zero-Rust gate: a driver this test file makes up (a string that
/// appears nowhere in the compiler) is declared and read back with its
/// policy, through the production registration path — the same claim W2.1
/// proved for the fixture, now proven against the REAL registry shape W3.2
/// will consume.
#[test]
fn a_driver_appearing_nowhere_in_rust_is_readable() {
    let src = r#"
%macro driver-gamepad-axis {
  %scope file
  %order 700
  %form { @driver-gamepad-axis }
  %registers driver(gamepad-axis) {
    primitive: planned:gamepad-driver
    value_type: progress
    seekable: false
    reduced_motion: allow
    on_kinds: elem
  }
}
"#;
    let ast = spacetime::parser::parse(src).expect("fixture parses");
    let mut registry = spacetime::metasystem::MetaRegistry::new();
    for def in &ast.meta_defs {
        registry
            .register(def.clone())
            .expect("production registration path");
    }
    let entries = registry.entries_of("driver");
    let (_, clause) = entries
        .iter()
        .find(|(_, c)| registry.entry_key(c) == Some("gamepad-axis"))
        .expect("the made-up driver is registered");
    assert_eq!(
        &ident_field(&registry, clause, "primitive"),
        "planned:gamepad-driver"
    );
    assert_eq!(&ident_field(&registry, clause, "value_type"), "progress");
}

/// The `planned:` marker is DATA the @on resolution must refuse loudly: a
/// driver whose primitive does not exist yet must never silently no-op.
/// This gate pins the marker's SHAPE so W3.2's resolution can match on it.
#[test]
fn planned_primitives_are_distinguishable_from_bound_ones() {
    let reg = stdlib_registry();
    let mut planned: Vec<String> = stdlib_drivers()
        .into_iter()
        // Value facets carry no primitive (PLAN-126) — skip them, don't panic.
        .filter(|(_, c)| reg.entry_field(c, "primitive").is_some())
        .filter(|(_, c)| ident_field(reg, c, "primitive").starts_with("planned:"))
        .map(|(k, _)| k)
        .collect();
    planned.sort();
    assert_eq!(
        planned,
        ["clip", "playback", "running", "steps"],
        "exactly the four future-wave drivers are planned (done/prev became value facets, PLAN-126)"
    );
}
