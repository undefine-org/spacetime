//! Baseline snapshot tests for INIT-034 parser migration.
//!
//! These tests capture the current parser output (CST structure + FormMatch captures)
//! and serve as the contract the new event-based parser must satisfy.
//!
//! Test categories:
//! 1. CST structure snapshots for each directive family (11 families)
//! 2. Meta-clause parsing snapshots (8 clause types)
//! 3. Event-based parser validation

use crate::syntax::cst::{self, SyntaxNode};

/// Pretty-print a Rowan CST for snapshot comparison.
fn dump_cst(node: &SyntaxNode, indent: usize) -> String {
    let mut out = String::new();
    let prefix = "  ".repeat(indent);
    out.push_str(&format!("{}{:?}", prefix, node.kind()));

    // Count children to decide if this is a leaf-like node
    let children: Vec<_> = node.children_with_tokens().collect();
    if children.iter().all(|c| c.as_token().is_some()) {
        // All children are tokens — show text inline
        let text: String = children
            .iter()
            .filter_map(|c| c.as_token())
            .map(|t| t.text().to_string())
            .collect();
        let text_trimmed = text.trim();
        if !text_trimmed.is_empty() {
            out.push_str(&format!(" \"{}\"", text_trimmed));
        }
        out.push('\n');
    } else {
        out.push('\n');
        for child in children {
            match child {
                rowan::NodeOrToken::Node(n) => {
                    out.push_str(&dump_cst(&n, indent + 1));
                }
                rowan::NodeOrToken::Token(t) => {
                    if !t.kind().is_trivia() {
                        out.push_str(&format!("{}  {:?} {:?}\n", prefix, t.kind(), t.text()));
                    }
                }
            }
        }
    }
    out
}

/// Parse source and return CST dump string.
fn parse_cst(source: &str) -> String {
    let result = cst::parse(source);
    let mut out = dump_cst(&result.root, 0);
    if !result.errors.is_empty() {
        out.push_str(&format!("\n--- {} error(s) ---\n", result.errors.len()));
        for err in &result.errors {
            out.push_str(&format!("  offset {}: {}\n", err.offset, err.message));
        }
    }
    out
}

// =====================================================
// 1. CST Structure Snapshots — Directive Families
// =====================================================

mod cst_directives {
    use super::*;

    #[test]
    fn snapshot_on_hover() {
        let cst = parse_cst("@on &.hover { opacity: 0 -> 1; }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_on_click_with_args() {
        let cst = parse_cst("@on &.click(0.3s) { scale: 0.95 -> 1; }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_scroll_progress() {
        let cst = parse_cst("@scroll page-progress { opacity: 0 -> 1; }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_load_animation() {
        let cst = parse_cst(
            "@load(0.5s) { opacity: 0 -> 1; transform: translateY(20px) -> translateY(0); }",
        );
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_loop_animation() {
        let cst = parse_cst("@loop(2s) { opacity: 0 -> 1 -> 0; }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_each_iteration() {
        let cst = parse_cst("@each $item in $items { .card { } }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_data_source() {
        let cst = parse_cst("@data users from \"/api/users\"");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_type_def() {
        let cst = parse_cst("@type User { id: number; name: string; }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_fn_def() {
        let cst = parse_cst("@fn greet($name) { \"Hello, \" + $name }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_computed() {
        let cst = parse_cst("@computed total($a, $b) { $a + $b }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_let_binding() {
        let cst = parse_cst("$count number: 0;");
        insta::assert_snapshot!(cst);
    }
}

// =====================================================
// 2. Meta-clause Parsing Snapshots (was section 3)
// =====================================================

mod meta_clause_snapshots {
    use super::*;

    #[test]
    fn snapshot_meta_primitive() {
        let cst = parse_cst("%primitive button { }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_macro() {
        let cst = parse_cst("%macro fade-in { }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_form() {
        let cst = parse_cst("%form { @on $event:ident }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_capture_type() {
        let cst = parse_cst("%capture_type easing { }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_bind() {
        let cst = parse_cst("%bind { event <- $event; }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_emit_js() {
        let cst = parse_cst("%emit js { console.log('hello'); }");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_yield() {
        let cst = parse_cst("%yield animation;");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_meta_cleanup() {
        let cst = parse_cst("%cleanup { removeEventListener(); }");
        insta::assert_snapshot!(cst);
    }
}

// =====================================================
// 3. Event-based Parser Validation (parse_matches)
// =====================================================

/// Tests that verify the event-based parser (MatchSink) produces equivalent
/// FormMatches to the old CST extraction path. These are the gate tests
/// for the dissolve-ast switchover.
mod event_parser_validation {
    use crate::syntax::bootstrap::bootstrap_stdlib;
    use crate::syntax::events;
    use crate::syntax::form_match::FormMatch;
    use crate::syntax::registry::SyntaxRegistry;
    use std::path::Path;

    fn get_stdlib_registry() -> Option<SyntaxRegistry> {
        let stdlib_path = Path::new("./stdlib");
        if !stdlib_path.exists() {
            return None;
        }
        bootstrap_stdlib(stdlib_path).ok()
    }

    /// Format FormMatch for comparison (sorted captures, no spans).
    fn format_match(fm: &FormMatch) -> String {
        let mut out = format!("macro: {}\n", fm.macro_name);
        let mut keys: Vec<_> = fm.captures.keys().collect();
        keys.sort();
        for key in keys {
            let value = &fm.captures[key];
            out.push_str(&format!("  {}: {:?}\n", key, value));
        }
        out
    }

    /// Run the event-based parser and return matches.
    fn event_parse(source: &str, registry: &SyntaxRegistry) -> Vec<FormMatch> {
        // Use catch_unwind since the event parser may panic on complex inputs
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (matches, _diags) = events::parse_matches(source, registry);
            matches
        }))
        .unwrap_or_default()
    }

    #[test]
    fn event_parser_simple_directive() {
        let registry = match get_stdlib_registry() {
            Some(r) => r,
            None => {
                eprintln!("Skipping: stdlib not found");
                return;
            }
        };

        // @on &.hover { ... } is a driver-body directive (no animation name, no parens)
        // The sigil head on-driver-body answers it, carrying the event as driver.member.
        let source = "@on &.hover { opacity: 0 -> 1; }";
        let matches = event_parse(source, &registry);

        // Should produce at least one match
        assert!(
            !matches.is_empty(),
            "Event parser should produce at least one match for: {source}"
        );

        let first = &matches[0];

        // Should have a 'driver' capture (the sigil ELEMENT_REF head; the
        // retired bare-event shape captured 'event' instead)
        assert!(
            first.captures.contains_key("driver"),
            "Expected 'driver' capture, got keys: {:?}",
            first.captures.keys().collect::<Vec<_>>()
        );

        // Verify the macro name uses the %form directive name
        assert_eq!(
            first.macro_name, "on",
            "Expected on (from %form @on ...), got: {}",
            first.macro_name
        );
    }

    #[test]
    fn event_parser_data_directive() {
        let registry = match get_stdlib_registry() {
            Some(r) => r,
            None => {
                eprintln!("Skipping: stdlib not found");
                return;
            }
        };

        // Unified @data <kind> surface (PLAN-023 W5/FEAT-047): file-scope
        // `@data fetch $name T : "url"` (the legacy selector-scoped paren form
        // `@data(src:, as:)` was removed).
        let source = "@data fetch $users User[] : \"/api/users\"";
        let matches = event_parse(source, &registry);

        assert!(
            !matches.is_empty(),
            "Event parser should produce at least one match for: {source}"
        );

        let first = &matches[0];
        insta::assert_snapshot!("event_parser_data_users", format_match(first));
    }

    #[test]
    fn event_parser_variable_declaration() {
        let registry = match get_stdlib_registry() {
            Some(r) => r,
            None => {
                eprintln!("Skipping: stdlib not found");
                return;
            }
        };

        // Check that the registry has forms for '$' prefix
        let dollar_forms = registry.forms_for_prefix('$');
        assert!(
            !dollar_forms.is_empty(),
            "Registry should have forms for '$' prefix"
        );

        let source = "$count number: 0;";
        let (matches, diags) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            events::parse_matches(source, &registry)
        }))
        .expect("Event parser should not panic");

        assert!(
            !matches.is_empty(),
            "Event parser should produce at least one match for: {source}\n\
             Diagnostics: {:?}\n\
             Dollar forms: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>(),
            dollar_forms
                .iter()
                .map(|f| (&f.macro_name, &f.form.directive_name))
                .collect::<Vec<_>>()
        );

        let first = &matches[0];
        insta::assert_snapshot!("event_parser_local_state", format_match(first));
    }

    #[test]
    fn event_parser_element_ref() {
        let registry = match get_stdlib_registry() {
            Some(r) => r,
            None => {
                eprintln!("Skipping: stdlib not found");
                return;
            }
        };

        // Check that the registry has forms for '&' prefix
        let amp_forms = registry.forms_for_prefix('&');
        assert!(
            !amp_forms.is_empty(),
            "Registry should have forms for '&' prefix"
        );

        let source = "&myButton .btn-primary;";
        let (matches, diags) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            events::parse_matches(source, &registry)
        }))
        .expect("Event parser should not panic");

        assert!(
            !matches.is_empty(),
            "Event parser should produce at least one match for: {source}\n\
             Diagnostics: {:?}\n\
             Amp forms: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>(),
            amp_forms
                .iter()
                .map(|f| (&f.macro_name, &f.form.directive_name))
                .collect::<Vec<_>>()
        );

        let first = &matches[0];
        insta::assert_snapshot!("event_parser_element_ref", format_match(first));
    }

    #[test]
    fn event_parser_scoped_directive() {
        let registry = match get_stdlib_registry() {
            Some(r) => r,
            None => {
                eprintln!("Skipping: stdlib not found");
                return;
            }
        };

        let source = ".card {\n  @on &.hover { opacity: 0 -> 1; }\n}";
        let matches = event_parse(source, &registry);

        // Should produce a match for the @on directive inside the scope
        assert!(
            !matches.is_empty(),
            "Event parser should produce matches for scoped directive"
        );

        let on_match = matches.iter().find(|m| m.macro_name.contains("on"));
        assert!(
            on_match.is_some(),
            "Should have an on-* match in scoped directive. Got: {:?}",
            matches.iter().map(|m| &m.macro_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn event_parser_no_panic_on_stdlib_files() {
        let registry = match get_stdlib_registry() {
            Some(r) => r,
            None => {
                eprintln!("Skipping: stdlib not found");
                return;
            }
        };

        // Test some representative stdlib-like patterns that previously caused panics
        let test_cases = [
            "@state_machine(initial: \"viewing\")",
            "@each($items) { }",
            "@fn double($x) { $x * 2 }",
            "@type User { name: string; age: number; }",
            "@load(0.5s) { opacity: 0 -> 1; }",
            "@loop pulse(2s) { scale: 1 -> 1.05 -> 1; }",
            "@scroll page-progress { opacity: 0 -> 1; }",
        ];

        for source in &test_cases {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                events::parse_matches(source, &registry)
            }));
            assert!(result.is_ok(), "Event parser panicked on: {source}");
        }
    }
}

// =====================================================
// 4. Test Body Expression Snapshots (INIT-039)
// =====================================================
//
// These capture CST output for expression patterns that appear
// in .test.st files, ensuring the parser handles them correctly.

mod test_body_expressions {
    use super::*;

    #[test]
    fn snapshot_raw_js_const_array() {
        let cst = parse_cst("@test \"t\" {\n  const arr = Array.of(1, 2, 3);\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_let_simple_assignment() {
        let cst = parse_cst("@test \"t\" {\n  @let (x = 42)\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_assert_strict_equality() {
        let cst = parse_cst("@test \"t\" {\n  @assert (x === 42)\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_assert_strict_inequality() {
        let cst = parse_cst("@test \"t\" {\n  @assert (x !== 'hello')\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_assert_method_call() {
        let cst = parse_cst("@test \"t\" {\n  @assert (arr.includes(2))\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_raw_js_query_selector() {
        let cst = parse_cst("@test \"t\" {\n  const el = document.querySelector('div');\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_raw_js_signal_set() {
        let cst = parse_cst("@test \"t\" {\n  ST.set(el, 'count', 0);\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_increment_assign() {
        let cst = parse_cst("@test \"t\" {\n  x += 1;\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_fixture_with_html() {
        let cst =
            parse_cst("@test \"t\" {\n  @fixture {\n    <div class=\"test\">Hello</div>\n  }\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_then_assertion() {
        let cst = parse_cst("@test \"t\" {\n  @then .box should exist\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_when_action() {
        let cst = parse_cst("@test \"t\" {\n  @when .btn click\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_eval_simple() {
        let cst = parse_cst("@test \"t\" {\n  @eval (window.__flag = 'yes')\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_eval_with_eval_string() {
        // The eval("...") trick for complex JS
        let cst = parse_cst(
            "@test \"t\" {\n  @eval (eval(\"setTimeout(function() { x = 1; }, 50)\"))\n}",
        );
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_wait_until_with_strict_eq() {
        let cst = parse_cst("@test \"t\" {\n  @wait_until __flag === 'yes' timeout: 500ms\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_assert_typeof() {
        let cst = parse_cst("@test \"t\" {\n  @assert (typeof __ctx === 'object')\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_assert_instanceof() {
        let cst = parse_cst("@test \"t\" {\n  @assert (d instanceof Date)\n}");
        insta::assert_snapshot!(cst);
    }

    #[test]
    fn snapshot_assert_bracket_property_access() {
        let cst = parse_cst("@test \"t\" {\n  @assert (window.__snaps[\"key\"] !== undefined)\n}");
        insta::assert_snapshot!(cst);
    }

    // Negative case — single-parameter arrows remain unsupported (the structured
    // parser currently recognizes only `(params) => body`), but FAT_ARROW must
    // surface as one lossless error token rather than two unrelated errors.
    #[test]
    fn snapshot_arrow_fn_in_assert_mangled() {
        // Parenthesize this arrow to use the supported structured-arrow path.
        // The parse tree is mangled but doesn't crash. This snapshot captures
        // the current (broken) behavior for regression tracking.
        let cst = parse_cst("@test \"t\" {\n  @assert (arr.filter(x => x > 3).length === 2)\n}");
        insta::assert_snapshot!(cst);
    }
}

#[cfg(test)]
mod bodyless_directive_css_boundary {
    use super::*;

    /// A bodyless directive (`@local-pointer`, a macro invocation) in a style body
    /// owns NO following tokens: the declarations after it are CSS_PROPERTY nodes of
    /// the body, never the directive's args. Regression: the inline-arg loop used to
    /// swallow `--aaa: $localX;` wholesale (its tokens — IDENT, COLON, VARIABLE_REF —
    /// are all legal inline args), so the FIRST declaration after a bodyless directive
    /// silently vanished — the property dropped from the CSS AND its signal binding
    /// never emitted (found live: a `--local-cursor-x: $localX` binding that never
    /// reached the browser, so the cursor dot only moved on Y). The FUP-078 guards
    /// covered `.`/`#`/`<`/element-scope/`&` siblings; declarations needed the
    /// IDENT+COLON twin PLUS seeding the newline flag from the name→first-arg trivia
    /// (the existing guards only fire from the SECOND arg on).
    #[test]
    fn bodyless_directive_does_not_swallow_following_declarations() {
        let cst = parse_cst(
            "@page p {\n  .box {\n    @local-pointer\n    --aaa: $localX;\n    --bbb: $localY;\n    color: red;\n  }\n}",
        );
        // All three declarations are CSS_PROPERTY nodes…
        assert_eq!(cst.matches("CSS_PROPERTY").count(), 3, "{cst}");
        // …and none leaked into the directive as an ARG.
        assert!(!cst.contains("ARG \"--aaa\""), "{cst}");
        assert!(!cst.contains("--- "), "no parse errors expected: {cst}");
    }

    /// The guard is newline-keyed: same-line declarations after a bodyless directive
    /// head still parse (they were never at risk — the swallow needs the newline-led
    /// arg run), and a same-line `ident:` inline arg (`@data name: Type`) is NOT a
    /// declaration and must keep parsing as args.
    #[test]
    fn same_line_directive_args_unaffected() {
        let cst = parse_cst("@page p {\n  .box {\n    @local-pointer --aaa: $localX;\n  }\n}\n");
        // Same-line tokens after the directive name remain the directive's args
        // (no newline boundary) — the guard must not fire mid-line.
        assert!(cst.contains("ARG \"--aaa\""), "{cst}");
    }
}

#[test]
fn snapshot_gh35_object_initializer_cst() {
    let cst = parse_cst("@import \"stdlib\"\nbody {\n  $user object: { name: \"Ada\" };\n  $flag bool: true;\n}\n");
    let mut settings = insta::Settings::new();
    settings.set_snapshot_path("snapshots");
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| {
        insta::assert_snapshot!("w3_gh35_object_initializer_cst", cst);
    });
}