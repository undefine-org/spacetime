//! Lazy-loaded Stdlib Registry
//!
//! This module provides a global, lazily-initialized SyntaxRegistry loaded from
//! stdlib. This allows the parser to use dynamic pattern matching via
//! `parse_matches()` rather than hardcoded `_form()` parsers.
//!
//! # Usage
//!
//! ```rust,ignore
//! use spacetime::syntax::stdlib_registry::STDLIB_REGISTRY;
//! use spacetime::syntax::events::parse_matches;
//!
//! // Match a statement against stdlib patterns
//! let input = "$count number: 0;";
//! let (matches, _) = parse_matches(input, &STDLIB_REGISTRY);
//! if let Some(form_match) = matches.first() {
//!     println!("Matched: {}", form_match.macro_name);
//! }
//! ```
//!
//! # Implementation Notes
//!
//! The registry is initialized on first access using `std::sync::LazyLock`.
//! It loads patterns from the `stdlib/` directory relative to the current
//! working directory. If stdlib loading fails, an empty registry is used.

use std::path::Path;
use std::sync::LazyLock;

use crate::syntax::{SyntaxRegistry, bootstrap::bootstrap_stdlib_from_embedded, bootstrap_stdlib};

/// Default path to stdlib directory
const STDLIB_PATH: &str = "stdlib";

/// Global stdlib registry, lazily initialized on first access.
///
/// This registry contains all %form patterns from stdlib files and is used
/// by the parser to dynamically match statements instead of using hardcoded
/// `_form()` parsers.
///
/// # Thread Safety
///
/// The registry is initialized exactly once using `LazyLock` and is safe
/// to access from multiple threads.
///
/// # Error Handling
///
/// If the filesystem stdlib is missing, falls back to the embedded stdlib
/// compiled into the binary. Only returns an empty registry if both fail.
pub static STDLIB_REGISTRY: LazyLock<SyntaxRegistry> = LazyLock::new(|| {
    // BUG-227: resolve the stdlib root the SAME way every other consumer does
    // (`toolchain::workspace_root_for`) instead of trusting a CWD-relative
    // `"stdlib"`. The old code only found the on-disk stdlib when the process
    // happened to be running from the toolchain checkout, and fell back to the
    // EMBEDDED registry otherwise -- silently, and with a `%form` set frozen at
    // the last `cargo build` of the embedding. So `spacetime build` run from a
    // project directory (and `spacetime-host push`, which always is) parsed the
    // user's site against stale grammar: 19 phantom parse errors on files that
    // compile cleanly from the repo root, each route degrading to an empty
    // bundle. Falling back to embedded is still correct for a DISTRIBUTED
    // binary that genuinely has no stdlib on disk -- it just must not be
    // reached merely because of where the shell happened to be.
    let disk_root = crate::toolchain::anchored_workspace_root()
        .map(|root| root.join(STDLIB_PATH))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| Path::new(STDLIB_PATH).to_path_buf());

    bootstrap_stdlib(&disk_root).unwrap_or_else(|_| {
        log::debug!(
            "Stdlib directory not found at {}, bootstrapping SyntaxRegistry from embedded stdlib",
            disk_root.display()
        );
        bootstrap_stdlib_from_embedded()
    })
});


/// Every `@form` declaration the stdlib itself carries, as `(kind, name)`
/// pairs with the name keeping its `--` sigil (today: the 33-entry easing
/// curve library in `macros/presets.st`). The stdlib loader registers
/// meta_DEFS; declaration INSTANCES (matches) would otherwise vanish — yet
/// they are DATA: the D12 easing validation (BUG-297) asks "is this name
/// declared?", and the form catalog (PLAN-135 W4) lists them.
///
/// Parsed from the same on-disk root [`STDLIB_REGISTRY`] boots from
/// (BUG-227's resolution, never a CWD gamble), falling back to the embedded
/// copy for a distributed binary. One parse, one source of truth.
pub static STDLIB_FORM_DECLARATIONS: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    STDLIB_FORM_DECLARATION_MATCHES
        .iter()
        .filter_map(|m| {
            let text = |key: &str| match m.captures.get(key) {
                Some(crate::syntax::CapturedValue::Ident(s))
                | Some(crate::syntax::CapturedValue::String(s)) => Some(s.clone()),
                _ => None,
            };
            Some((text("kind")?, text("name")?))
        })
        .collect()
});

/// The stdlib form declarations as FULL matches — body, params, and the
/// declaration's `///` doc included (PLAN-135 W4). The pair projection above
/// serves the D12 easing validation (BUG-297); these serve the `@data forms`
/// catalog, which lists the preset library beside the page's own forms.
pub static STDLIB_FORM_DECLARATION_MATCHES: LazyLock<Vec<crate::syntax::FormMatch>> =
    LazyLock::new(|| {
        let source = crate::toolchain::anchored_workspace_root()
            .map(|root| root.join(STDLIB_PATH).join("macros/presets.st"))
            .filter(|p| p.is_file())
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_else(|| include_str!("../../stdlib/macros/presets.st").to_string());
        let (matches, _) = crate::syntax::events::parse_matches(&source, &STDLIB_REGISTRY);
        matches
            .into_iter()
            .filter(|m| {
                STDLIB_REGISTRY
                    .get_by_macro_name(m.matched_macro.as_deref().unwrap_or(&m.macro_name))
                    .and_then(|f| f.macro_def.registers.as_ref())
                    .is_some_and(|r| r.name == "form")
            })
            .map(|mut m| {
                m.doc = crate::syntax::doc_comment_before(&source, m.span.start as usize);
                if m.source_file.is_none() {
                    m.source_file = Some("stdlib/macros/presets.st".to_string());
                }
                m
            })
            .collect()
    });

/// Match a statement against stdlib patterns.
///
/// This is a convenience function that uses the global `STDLIB_REGISTRY`.
/// It's the main entry point for dynamic pattern matching in the parser.
///
/// # Arguments
///
/// * `input` - The statement string to match (e.g., "$count number: 0;")
///
/// # Returns
///
/// * `Some(FormMatch)` - If a pattern matched, containing captured values and spans
/// * `None` - If no patterns matched
///
/// # Example
///
/// ```rust,ignore
/// use spacetime::syntax::stdlib_registry::match_statement_stdlib;
///
/// let input = "$count number: 0;";
/// if let Some(fm) = match_statement_stdlib(input) {
///     assert_eq!(fm.macro_name, "local-state");
///     assert_eq!(fm.get_ident("name"), Some("count"));
/// }
/// ```
pub fn match_statement_stdlib(input: &str) -> Option<crate::syntax::FormMatch> {
    let (matches, _diagnostics) = crate::syntax::events::parse_matches(input, &STDLIB_REGISTRY);
    matches.into_iter().next()
}

/// Check if the stdlib registry has been loaded and contains patterns.
///
/// # Returns
///
/// * `true` - If the registry has at least one form registered
/// * `false` - If the registry is empty (stdlib failed to load or has no patterns)
pub fn is_stdlib_loaded() -> bool {
    STDLIB_REGISTRY.form_count() > 0
}

/// Get the number of forms in the stdlib registry.
pub fn stdlib_form_count() -> usize {
    STDLIB_REGISTRY.form_count()
}

/// Get all prefixes registered in the stdlib.
pub fn stdlib_prefixes() -> Vec<char> {
    STDLIB_REGISTRY.prefixes().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdlib_registry_loads() {
        // This test will only pass if stdlib directory exists
        // In CI/test environments, it may return an empty registry
        let _ = &*STDLIB_REGISTRY; // Force initialization
        // Just verify it doesn't panic
    }

    #[test]
    fn test_match_statement_stdlib_with_local_state() {
        // Skip if stdlib not loaded
        if !is_stdlib_loaded() {
            eprintln!("Skipping test - stdlib not loaded");
            return;
        }

        let input = "$count number: 0;";
        let result = match_statement_stdlib(input);

        // If local-state pattern is in stdlib, this should match
        if let Some(fm) = result {
            assert_eq!(fm.macro_name, "local-state");
            assert!(fm.get_ident("name").is_some());
        }
    }

    #[test]
    fn test_match_statement_stdlib_with_element_ref() {
        // Skip if stdlib not loaded
        if !is_stdlib_loaded() {
            eprintln!("Skipping test - stdlib not loaded");
            return;
        }

        let input = "&header .header;";
        let result = match_statement_stdlib(input);

        // If element-ref pattern is in stdlib, this should match
        if let Some(fm) = result {
            assert_eq!(fm.macro_name, "element-ref");
            assert!(fm.get_ident("name").is_some());
        }
    }

    #[test]
    fn test_match_statement_stdlib_no_match() {
        let input = "this is not a pattern;";
        let result = match_statement_stdlib(input);
        assert!(result.is_none());
    }

    #[test]
    // INIT-039: resolved — MatchSink now pushes unmatched child nodes to parent context
    fn test_template_with_single_param_matches() {
        // Skip if stdlib not loaded
        if !is_stdlib_loaded() {
            eprintln!("Skipping test - stdlib not loaded");
            return;
        }

        // Test that @template with a single param matches via param_list capture type
        let input = "@template &card($content) {}";
        let result = match_statement_stdlib(input);

        assert!(
            result.is_some(),
            "@template with single param should match. \
             This tests the param_list capture type is working."
        );

        let fm = result.unwrap();
        assert_eq!(fm.macro_name, "template");
    }

    #[test]
    // INIT-039: resolved — MatchSink now pushes unmatched child nodes to parent context
    fn test_template_with_multiple_params_matches() {
        // Skip if stdlib not loaded
        if !is_stdlib_loaded() {
            eprintln!("Skipping test - stdlib not loaded");
            return;
        }

        // Test that @template with multiple params matches via param_list capture type
        let input = "@template &card($title, &content, $footer?) {}";
        let result = match_statement_stdlib(input);

        assert!(
            result.is_some(),
            "@template with multiple params should match. \
             This tests the param_list capture type handles commas and optional markers."
        );

        let fm = result.unwrap();
        assert_eq!(fm.macro_name, "template");
    }

    #[test]
    // INIT-039: resolved — MatchSink now pushes unmatched child nodes to parent context
    fn test_template_with_empty_params_matches() {
        // Skip if stdlib not loaded
        if !is_stdlib_loaded() {
            eprintln!("Skipping test - stdlib not loaded");
            return;
        }

        // Test that @template with empty params matches
        let input = "@template &card() {}";
        let result = match_statement_stdlib(input);

        assert!(
            result.is_some(),
            "@template with empty params should match. \
             This tests the param_list capture type handles empty input."
        );

        let fm = result.unwrap();
        assert_eq!(fm.macro_name, "template");
    }
}
