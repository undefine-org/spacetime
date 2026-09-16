//! Domain-specific "did you mean?" suggestions for Spacetime diagnostics.
//!
//! Provides suggestion functions for each error domain:
//! - Unknown directives (% meta-directives)
//! - Unknown CSS properties
//! - Unknown easing presets
//! - Unknown element refs (&name)
//! - Unknown data sources (@data)

use super::collector::{find_similar_multiple, format_suggestion};



/// Suggest similar directive names for an unknown meta-directive.
///
/// `unknown` should be the directive name without the `%` prefix.
/// `known_directives` is an iterator of known directive names (also without prefix).
pub fn suggest_directive(unknown: &str, known_directives: &[&str]) -> Option<String> {
    let matches = find_similar_multiple(unknown, known_directives, 3);
    format_suggestion(&matches)
}

/// Suggest similar CSS properties for an unknown property name.
///
/// DELETED (PLAN-136 W7). This read `CSS_PROPERTIES` — 98 hand-written names,
/// documented in its own comment as "not exhaustive", with no type information.
/// It had no callers: measured, `grep -rn suggest_css_property src/` returned
/// only this definition. A hardcoded list, stale by construction because it can
/// never keep pace with CSS, maintained by hand for a function nobody called.
///
/// Its three unit tests went with it — they exercised the list, not a
/// behaviour anyone depended on.
///
/// The question it was meant to answer is now answered by
/// `syntax::property_types::suggest_property`, which reads the map DERIVED from
/// the forms — so it needs no maintenance and covers Spacetime's own properties,
/// where a typo costs most because no upstream parser knows them.

/// Suggest similar easing presets for an unknown preset name.
///
/// DELETED (FUP-181 cutover). This read `EASING_PRESETS` — thirteen hand-written
/// names, and the EIGHTH hand-synced list this arc has removed. Measured before
/// deleting: `grep -rn "suggest_easing_preset\\|EASING_PRESETS" --include=*.rs .`
/// over the WHOLE repository returned the definition plus three `#[cfg(test)]`
/// callers and nothing else — no production caller, no re-export.
///
/// It was worse than unused. Every name in it was spelled for the `&ease-out-expo`
/// SIGIL, retired by SIP-001c (`stdlib/macros/form.st:169`, "an easing has no
/// identity"). So the list would have suggested a retired spelling to an author
/// who typo'd — teaching the form the compiler no longer accepts.
///
/// Easings are now the `easing` `%capture_type` grammar, and a bad one is
/// refused by `capture_type_accepts` with the caret on the value. The presets
/// themselves live in stdlib where they can be read, extended, and versioned.

/// Suggest similar element refs for an unknown `&name` reference.
///
/// `unknown` should be the ref name without the `&` prefix.
/// `known_refs` is the list of element refs declared in the current scope.
pub fn suggest_element_ref(unknown: &str, known_refs: &[&str]) -> Option<String> {
    let matches = find_similar_multiple(unknown, known_refs, 3);
    format_suggestion(&matches)
}

/// Suggest similar data sources for an unknown `@data` reference.
///
/// `unknown` should be the data source name.
/// `known_sources` is the list of `@data` declarations in the current scope.
pub fn suggest_data_source(unknown: &str, known_sources: &[&str]) -> Option<String> {
    let matches = find_similar_multiple(unknown, known_sources, 3);
    format_suggestion(&matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_directive_close_match() {
        let directives = &[
            "primitive",
            "macro",
            "form",
            "emit",
            "bind",
            "binds",
            "derives",
            "states",
            "provides",
            "on",
            "if",
            "for",
        ];
        assert_eq!(
            suggest_directive("primtive", directives),
            Some("did you mean 'primitive'?".to_string())
        );
    }

    #[test]
    fn test_suggest_directive_no_match() {
        let directives = &["primitive", "macro", "form"];
        assert_eq!(suggest_directive("xyzzy", directives), None);
    }

    #[test]
    fn test_suggest_directive_multiple() {
        let directives = &["binds", "bind", "builds", "derives"];
        let result = suggest_directive("bindd", directives);
        assert!(result.is_some());
        let msg = result.unwrap();
        // Should suggest "binds" and "bind" (both distance 1)
        assert!(msg.contains("binds") || msg.contains("bind"));
    }




    #[test]
    fn test_suggest_element_ref() {
        let refs = &["button", "header", "sidebar", "footer"];
        assert_eq!(
            suggest_element_ref("buttn", refs),
            Some("did you mean 'button'?".to_string())
        );
    }

    #[test]
    fn test_suggest_data_source() {
        let sources = &["users", "products", "categories"];
        assert_eq!(
            suggest_data_source("prodcts", sources),
            Some("did you mean 'products'?".to_string())
        );
    }

    // === insta snapshot tests for error message quality ===

    #[test]
    fn snapshot_directive_single_suggestion() {
        let directives = &[
            "primitive",
            "macro",
            "form",
            "emit",
            "bind",
            "binds",
            "derives",
            "states",
            "provides",
            "on",
            "for",
        ];
        let result = suggest_directive("primtive", directives).unwrap();
        insta::assert_snapshot!(result, @"did you mean 'primitive'?");
    }

    #[test]
    fn snapshot_directive_multiple_suggestions() {
        let directives = &["binds", "bind", "builds", "derives", "states"];
        let result = suggest_directive("bindd", directives).unwrap();
        insta::assert_snapshot!(result, @"did you mean one of: 'binds' (1 edit), 'bind' (1 edit)?");
    }


    #[test]
    fn snapshot_element_ref_typo() {
        let refs = &["button", "header", "sidebar", "footer", "modal"];
        let result = suggest_element_ref("headr", refs).unwrap();
        insta::assert_snapshot!(result, @"did you mean 'header'?");
    }

    #[test]
    fn snapshot_data_source_typo() {
        let sources = &["users", "products", "categories", "orders"];
        let result = suggest_data_source("ordrs", sources).unwrap();
        insta::assert_snapshot!(result, @"did you mean 'orders'?");
    }

    #[test]
    fn snapshot_element_ref_multiple() {
        let refs = &["button", "buttons", "bottom", "header"];
        let result = suggest_element_ref("botton", refs).unwrap();
        insta::assert_snapshot!(result, @"did you mean one of: 'button' (1 edit), 'bottom' (1 edit), 'buttons' (2 edits)?");
    }

    #[test]
    fn snapshot_short_name_threshold() {
        // Short names should use stricter threshold (distance < half length)
        let directives = &["on", "if", "for", "fn"];
        let result = suggest_directive("ox", directives);
        insta::assert_snapshot!(format!("{:?}", result), @r#"Some("did you mean 'on'?")"#);
    }
}
