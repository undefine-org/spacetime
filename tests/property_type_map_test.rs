//! What type does `easing:` accept? Ask the forms — they already said.
//!
//! PLAN-136 W3. The first shape of this work was a stdlib table:
//!
//!     %css_property easing  { accepts: easing }
//!     %css_property stagger { accepts: duration }
//!
//! It was cancelled, because the fact is already written down. Every one of
//! these properties is a `%form` parameter, and a form parameter carries its
//! type in its declaration:
//!
//!     stdlib/macros/loop.st:51     easing:  $easing:ident   = ease-out
//!     stdlib/macros/fade-in.st:54  easing:  $easing:easing  = ease-out
//!
//! A table would have been a SECOND statement of that fact, and two sources for
//! one fact is the defect — whoever writes the second one. So the map is DERIVED
//! from `SyntaxRegistry::all_forms()`, and no file in the repo lists a property
//! beside a type.
//!
//! KEYED BY (directive, property), NOT by property. Measured: 16 properties are
//! declared with different types in different forms, but scoped to the owning
//! form the disagreements drop to 3 — and all 3 are deliberate sibling forms
//! (one productive, one whose only job is to report the invalid spelling; see
//! stdlib/macros/data-kind.st:352). The other 13 are DIFFERENT PROPERTIES THAT
//! SHARE A NAME:
//!
//!     size: $size:number = 1        in @object  — scene units (3D)
//!     size: $size:string = "24px"   in @cursor  — a CSS length
//!
//! A global property->type map would have forced 13 false reconciliations and
//! broken the 3D macros. The scope is not a detail of the implementation; it is
//! the reason the derivation is right and the table was not.

use spacetime::syntax::property_types::{property_capture_type, property_type_map};

/// The headline: the map exists and answers from what the forms declare.
#[test]
fn the_map_is_derived_from_registered_forms() {
    let map = property_type_map();
    assert!(
        map.len() > 100,
        "the stdlib declares 409 typed form params; a map of {} entries means \
         the derivation is not walking the registry",
        map.len()
    );
}

/// The measured case that motivated the whole arc.
///
/// Each directive answers from its OWN declaration, including when the answer
/// is the weak one. `@fade-in` really does say `:ident`
/// (stdlib/macros/fade-in.st:25) while its sibling `@fade-in-up` says `:easing`
/// (:54) — they are separate directives, not one directive contradicting
/// itself.
///
/// W5 HAS NOW TIGHTENED IT, and this test is how that is recorded. It was
/// written asserting `ident` — the weak spelling — with a note saying it would
/// fail on purpose once the tightening landed. It did, on the run after W5, and
/// the assertion moved to `easing` rather than being deleted: a test that
/// changes with the fact it pins is doing its job; one that is removed when it
/// goes red is not.
#[test]
fn each_directive_answers_from_its_own_declaration() {
    assert_eq!(
        property_capture_type("fade-in", "easing").as_deref(),
        Some("easing"),
        "@fade-in declared `easing: $easing:ident` until W5; it now declares \
         `:easing`, the grammar that can actually refuse `not-a-curve`"
    );
    assert_eq!(
        property_capture_type("fade-in-up", "easing").as_deref(),
        Some("easing"),
        "@fade-in-up declares `easing: $easing:easing` (stdlib/macros/fade-in.st:54)"
    );
}

/// The scope is the point: same property name, two different types, both right.
#[test]
fn the_same_property_name_may_be_two_types_in_two_directives() {
    let object_size = property_capture_type("object", "size");
    let cursor_size = property_capture_type("cursor", "size");

    assert_eq!(
        object_size.as_deref(),
        Some("number"),
        "@object's `size` is scene units (stdlib/3d/macros/object.st:42)"
    );
    assert_eq!(
        cursor_size.as_deref(),
        Some("string"),
        "@cursor's `size` is a CSS length written as a string \
         (stdlib/macros/cursor.st:26)"
    );
    assert_ne!(
        object_size, cursor_size,
        "these are different properties that share a name; a map keyed by \
         property alone would have to pick one and break the other"
    );
}

/// An unknown directive or property has no answer, and that is not an error —
/// it is what keeps the enforcement in W4 from refusing things it cannot judge.
#[test]
fn an_unknown_key_has_no_answer() {
    assert_eq!(property_capture_type("no-such-directive", "easing"), None);
    assert_eq!(property_capture_type("fade-in", "no-such-property"), None);
}

/// Every type the map reports must be one the compiler can actually check,
/// otherwise W4 would enforce against a name nothing resolves.
#[test]
fn every_derived_type_is_a_type_the_compiler_knows() {
    use spacetime::syntax::events::capture_type_accepts;
    for ((directive, property), ty) in property_type_map() {
        // A type the compiler cannot resolve accepts everything, including a
        // string that could not be any real value. That is the signature of a
        // name nothing backs.
        let sentinel = "\u{1}not a value\u{1}";
        // `expr` and `string` are DELIBERATELY permissive: `expr` means "hand
        // this to JS" and a string literal is checked by the lexer, not by a
        // value grammar.
        //
        // `array` and `object` are structural shapes with no grammar behind
        // them — `%capture_type array` does not exist. They describe how a
        // param is parsed, not what a value may be, so W4 must skip them rather
        // than enforce a name nothing backs. Recorded here so the exemption is
        // a stated decision instead of a silent hole; see
        // `w4_skips_types_that_resolve_to_nothing`.
        const UNCHECKABLE: &[&str] = &["expr", "string", "array", "object"];
        if capture_type_accepts(&ty, sentinel) && !UNCHECKABLE.contains(&ty.as_str()) {
            panic!(
                "@{directive}'s `{property}` is declared `{ty}`, but `{ty}` \
                 resolves to nothing — it accepts even {sentinel:?}. Enforcing \
                 against it in W4 would be enforcing nothing. Either give `{ty}` \
                 a grammar, or add it to UNCHECKABLE with a reason."
            );
        }
    }
}
