//! What type does a directive-body property accept? Derived, never written.
//!
//! PLAN-136 W3.
//!
//! `easing:`, `stagger:`, `range:` and `at:` are properties in a language we
//! define, and until this module nothing checked their values — 2,952 corpus
//! declarations with no property->type answer, 1,010 of them `easing`. The
//! obvious fix was a stdlib table pairing each property with a type. It was
//! cancelled, because the pairing is ALREADY WRITTEN:
//!
//!     stdlib/macros/fade-in.st:54   easing: $easing:easing = ease-out
//!     stdlib/macros/loop.st:52      stagger: $stagger:number = 100
//!
//! A `%form` parameter carries its type in its declaration. A table would have
//! been a second statement of that fact, and two sources for one fact is the
//! defect regardless of who writes the second one — the same reason PLAN-122
//! cancelled its own `%css_property` table and deleted seven hand-synced scalar
//! lists.
//!
//! # The key is (directive, property)
//!
//! Measured across stdlib: 16 property NAMES carry more than one type. Scoped to
//! the form that owns them, the disagreements drop to 3, and each of those 3 is
//! a deliberate pair of sibling forms — one productive, one that exists only to
//! report the invalid spelling (`stdlib/macros/data-kind.st:352` says so in
//! prose). The other 13 are different properties that happen to share a name:
//!
//!     size: $size:number = 1       @object — scene units in a 3D scene
//!     size: $size:string = "24px"  @cursor — a CSS length
//!     color: $color:color          @light  — a THREE.js colour
//!     color: $color:string         @cursor — a CSS colour
//!
//! Keying by property alone would have to choose one type per name, forcing 13
//! false reconciliations and breaking the 3D macros. The scope is not an
//! implementation detail; it is the reason the derivation is correct where a
//! flat table could not be.

use std::collections::HashMap;

/// `(directive, property) -> capture type`, derived from every registered form.
pub type PropertyTypeMap = HashMap<(String, String), String>;

/// Build the map by walking the forms the registry already holds.
///
/// Both the parenthesized params (`@fade-in(easing: $easing:easing)`) and the
/// body params (`@on &.scroll { easing: $easing:easing; }`) are read: an author
/// writes a property in either position and means the same thing by it.
///
/// A param with no declared type contributes nothing. `easing: $easing` states
/// a name, not a contract, and inventing one from a sibling declaration is
/// exactly the guess this module exists to avoid.
pub fn property_type_map() -> PropertyTypeMap {
    let registry = &*crate::syntax::stdlib_registry::STDLIB_REGISTRY;
    let mut map = PropertyTypeMap::new();

    for registered in registry.all_forms() {
        // A MIGRATION entry declares the form it is RETIRING, so it can
        // recognize the old spelling and rewrite it. Those declarations are a
        // record of what the language used to accept, not a contract it still
        // offers — and they are frequently the WEAKER spelling, since the whole
        // point of the cutover was to improve on them.
        //
        // Measured: `stdlib/migrations/entries/2026-07-26-on-cutover.st`
        // declares `stagger: $stagger:number` for the retired `@on` arms, and
        // letting it into the map made `stagger: 0.05 first` — authored in 20
        // corpus files today — fail as "not a valid number". A retired form
        // must not be able to refuse code written for the live one.
        // `retired` is the STRUCTURAL marker a `%migration` capsule stamps on
        // the macros it owns. Keying on the source PATH instead (an earlier
        // draft matched `/migrations/`) would silently stop working the day a
        // capsule moved directory, and would wrongly exclude a live macro that
        // merely lived near one.
        if registered.macro_def.retired.is_some() {
            continue;
        }
        let form = &registered.form;
        let directive = form.directive_name.trim_start_matches('@').to_string();
        if directive.is_empty() {
            continue;
        }
        for param in form.params.iter().chain(form.body_params.iter()) {
            // A pseudo-selector body param carries no label; there is no
            // property to key it by.
            if param.name.is_empty() {
                continue;
            }
            let Some(ty) = declared_capture_type(param) else {
                continue;
            };
            // First declaration wins. Within one directive the type is
            // consistent except for the three deliberate sibling-form pairs,
            // where the productive form is registered first and the
            // error-reporting sibling must not overwrite it.
            map.entry((directive.clone(), param.name.clone()))
                .or_insert(ty);
        }
    }
    map
}

/// The type `@directive`'s `property:` accepts, if the form declares one.
///
/// `None` means "no answer", which is not the same as "invalid": an unknown
/// directive, an unlabelled param, or a param declared without a type all land
/// here, and a caller must stay silent rather than refuse what it cannot judge.
pub fn property_capture_type(directive: &str, property: &str) -> Option<String> {
    property_type_map()
        .get(&(directive.to_string(), property.to_string()))
        .cloned()
}

/// Read the capture type out of a form param's elements.
fn declared_capture_type(param: &crate::parser::meta_ast::FormParam) -> Option<String> {
    use crate::parser::meta_ast::FormInlineElement;
    param.elements.iter().find_map(|el| match el {
        FormInlineElement::Capture(capture, _) => capture_type_name(&capture.capture_type),
        _ => None,
    })
}

/// The SOURCE SPELLING of a capture type.
///
/// The map's whole purpose is to hand a type name to `capture_type_accepts`,
/// which resolves names — so the enum must be rendered back to the spelling the
/// author wrote. `Custom(name)` already is that spelling; the builtin variants
/// each have exactly one.
///
/// Structural types (`Balanced`, `Union`, `PatternMatch`, block captures) are
/// deliberately absent: they describe a SHAPE the property parser consumes, not
/// a scalar value a property can be checked against, and reporting one here
/// would invite W4 to enforce a body capture as if it were a value.
fn capture_type_name(ct: &crate::parser::meta_ast::CaptureType) -> Option<String> {
    use crate::parser::meta_ast::CaptureType as C;
    let name = match ct {
        C::Custom(name) => return Some(name.clone()),
        C::Ident => "ident",
        C::DashedIdent => "dashed_ident",
        C::EventName => "event_name",
        C::String => "string",
        C::Number => "number",
        C::Bool => "bool",
        C::Time => "time",
        C::Length => "length",
        C::Duration => "duration",
        C::Easing => "easing",
        C::Color => "color",
        C::Typeref => "typeref",
        C::Binding => "binding",
        C::Event => "event",
        C::Expr => "expr",
        C::Selector => "selector",
        C::Element => "element",
        _ => return None,
    };
    Some(name.to_string())
}

/// The property `@directive` declares that `unknown` was probably meant to be.
///
/// PLAN-136 W7. The derived map already knows every property each directive
/// declares, so "did you mean?" is the same fact answering a second question —
/// no list to maintain, and it covers Spacetime's own properties, which is
/// where a typo costs most because no upstream parser knows them.
///
/// Scoped to the directive for the same reason the map is: `@object` has a
/// `size` and `@fade-in-up` does not. Offering one inside the other would send
/// the author to a property that does not exist there.
pub fn suggest_property(directive: &str, unknown: &str) -> Option<String> {
    let map = property_type_map();
    let unknown_lc = unknown.to_ascii_lowercase();

    let mut best: Option<(usize, String)> = None;
    for (d, property) in map.keys() {
        if d != directive {
            continue;
        }
        let candidate = property.to_ascii_lowercase();
        // An exact match is not a typo — suggesting the word the author already
        // wrote is noise, and noise trains people to ignore the suggestion that
        // matters.
        if candidate == unknown_lc {
            return None;
        }
        let d = edit_distance(&unknown_lc, &candidate);
        // Tolerate one edit per four characters, at least one and at most three.
        // A looser bound starts matching unrelated words; a tighter one misses
        // a transposed pair.
        let budget = (candidate.len() / 4).clamp(1, 3);
        if d <= budget && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
            best = Some((d, property.clone()));
        }
    }
    best.map(|(_, name)| name)
}

/// Levenshtein distance, for the did-you-mean above.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three same-directive disagreements are deliberate sibling forms, and
    /// the productive one must win. `@data fetch` declares `refresh:duration?`
    /// in the form that works and `refresh:expr` in the sibling that exists to
    /// report the invalid spelling (stdlib/macros/data-kind.st:352).
    ///
    /// If this ever reports `expr`, the error-reporting sibling has overwritten
    /// the real contract and W4 would enforce "anything goes" for `refresh`.
    #[test]
    fn a_sibling_error_form_does_not_overwrite_the_productive_one() {
        // `@data`'s options live in a BODY GROUP (`@data fetch $x : $src {
        // refresh: ... }`), which this derivation does not read — body groups
        // are a raw pattern AST, not named `FormParam`s, so there is nothing
        // here to key by a property name. Measured: the map holds ZERO `data`
        // entries.
        //
        // This test previously wrote `if let Some(ty) = ...`, which passed
        // whether or not the entry existed — a gate that could not fail, and it
        // hid exactly this gap until W4b tried to spend the map on a real file.
        // Asserting the absence keeps the limitation VISIBLE and turns this
        // into a red test the moment body groups start contributing, which is
        // when the sibling-precedence question becomes real.
        assert_eq!(
            property_capture_type("data", "refresh"),
            None,
            "@data's options are body-group captures, not named params. If this \
             now resolves, the derivation reads body groups — re-establish the \
             sibling-precedence assertion: the productive form declares \
             `refresh: $refresh:duration?` and the error-reporting sibling \
             declares `:expr`, which must NOT win."
        );
    }

    /// The derivation reads named params, so a form that declares one is in the
    /// map with the type it declared. Without this the module could return an
    /// empty map and every other test here would still pass.
    #[test]
    fn a_named_param_is_in_the_map_with_its_declared_type() {
        assert_eq!(
            property_capture_type("fade-in-up", "distance").as_deref(),
            Some("length"),
            "@fade-in-up declares `distance: $distance:length` (stdlib/macros/fade-in.st:53)"
        );
    }

    /// `capture_type_name` maps enum variants back to source spellings by hand,
    /// and it FAILS OPEN: a variant it does not know returns `None`, so that
    /// property silently leaves the map and stops being checked. A new
    /// `CaptureType` would therefore weaken validation with no build error and
    /// no failing test — the exact silence this arc exists to remove.
    ///
    /// This pins the spellings that MUST resolve. It cannot see a variant
    /// nobody added yet, but it does fail the moment one of these is dropped or
    /// renamed, which is the half that has actually gone wrong before.
    #[test]
    fn every_value_bearing_capture_type_has_a_source_spelling() {
        use crate::parser::meta_ast::CaptureType as C;
        for (ct, expected) in [
            (C::Ident, "ident"),
            (C::DashedIdent, "dashed_ident"),
            (C::String, "string"),
            (C::Number, "number"),
            (C::Bool, "bool"),
            (C::Time, "time"),
            (C::Length, "length"),
            (C::Duration, "duration"),
            (C::Easing, "easing"),
            (C::Color, "color"),
            (C::Expr, "expr"),
            (C::Binding, "binding"),
            (C::Selector, "selector"),
        ] {
            assert_eq!(
                super::capture_type_name(&ct).as_deref(),
                Some(expected),
                "{ct:?} must round-trip to its source spelling; without it every \
                 property declared with this type silently leaves the map"
            );
        }
        // A stdlib production carries its own name and must pass through.
        assert_eq!(
            super::capture_type_name(&C::Custom("stagger_value".into())).as_deref(),
            Some("stagger_value")
        );
    }

    /// Derivation must not invent a type for a param that declares none.
    #[test]
    fn an_untyped_param_contributes_nothing() {
        let map = property_type_map();
        for ((directive, property), ty) in &map {
            assert!(
                !ty.is_empty(),
                "@{directive}'s `{property}` mapped to an empty type"
            );
        }
    }
}





