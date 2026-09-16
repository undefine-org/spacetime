//! Integration Tests for the Pipeline
//!
//! These tests demonstrate the full pipeline flow from FormMatch to output.

#[cfg(test)]
mod tests {
    use crate::metasystem::MetaRegistry;
    use crate::parser::SourceSpan;
    use crate::pipeline::{CompileContext, compile};
    use crate::syntax::{CapturedValue, FormMatch};
    use std::collections::HashMap;

    fn make_form_match(
        macro_name: &str,
        captures: HashMap<String, CapturedValue>,
        selector: Option<String>,
    ) -> FormMatch {
        FormMatch {
            macro_name: macro_name.to_string(),
            matched_macro: None,
            captures,
            capture_spans: HashMap::new(),
            selector,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        }
    }

    #[test]
    fn test_pipeline_with_runtime_wrapper() {
        let context = crate::pipeline::probe_context(&["local-state"]);
        // include_runtime is true by default

        let form = make_form_match(
            "local-state",
            {
                let mut map = HashMap::new();
                map.insert(
                    "name".to_string(),
                    CapturedValue::Ident("value".to_string()),
                );
                map
            },
            None,
        );

        let result = compile(&[form], &context);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Should have real ST runtime (from public/runtime/st.js)
        assert!(
            output.js.contains("Spacetime Core Runtime"),
            "Should include ST runtime header"
        );
        assert!(
            output.js.contains("global.ST = ST"),
            "Should export ST global"
        );
    }

    #[test]
    fn test_pipeline_empty_input() {
        let context = CompileContext::default();
        let result = compile(&[], &context);

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.js, "");
        assert_eq!(output.css, "");
    }

    #[test]
    fn test_pipeline_preserves_span_info() {
        let context = crate::pipeline::probe_context(&["data-fetch"]).without_runtime();

        let span = SourceSpan { start: 10, end: 50 };

        let form = FormMatch {
            macro_name: "data-fetch".to_string(),
            matched_macro: None,
            captures: HashMap::new(),
            capture_spans: HashMap::new(),
            selector: None,
            span,
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        // Compile should succeed and preserve span info internally
        // (even though we can't directly verify it in the output)
        let result = compile(&[form], &context);
        assert!(result.is_ok());
    }

    // Tests for adapter conversion removed - adapter.rs functionality now internal to convert.rs
    // The FormMatch collection is now handled directly during parsing in convert_file()

    // ==========================================================================
    // Integration tests that parse real .st content through the pipeline
    // These tests ensure the full flow works: parse -> FormMatch -> pipeline
    // ==========================================================================

    use crate::parser::parse as parse_to_ast;

    #[test]
    fn test_parse_scroll_macro_in_scope() {
        // Parse @scroll inside a scope block - this was the original bug
        let input = r#"
.my-element {
    @scroll reveal(start: 0.1, end: 0.3) {
        opacity: 0 -> 1;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        // Verify FormMatch was collected
        assert!(
            !ast.matches.is_empty(),
            "Should have at least one FormMatch"
        );

        // Find the scroll FormMatch
        let scroll_match = ast
            .matches
            .iter()
            .find(|m| m.macro_name == "scroll")
            .expect("Should have a 'scroll' FormMatch");

        // Verify the "name" capture is set correctly (not "arg0")
        assert!(
            scroll_match.captures.contains_key("name"),
            "Should have 'name' capture, got: {:?}",
            scroll_match.captures.keys().collect::<Vec<_>>()
        );

        // Verify the name value is "reveal"
        if let CapturedValue::Ident(name) = scroll_match.captures.get("name").unwrap() {
            assert_eq!(name, "reveal", "Name should be 'reveal'");
        } else {
            panic!("Name capture should be an Ident");
        }

        // Verify selector is captured
        assert_eq!(
            scroll_match.selector.as_deref(),
            Some(".my-element"),
            "Should have selector"
        );
    }

    #[test]
    fn test_parse_on_hover_macro_in_scope() {
        // Parse @on with event + name inside a scope
        let input = r#"
.button {
    @on &.hover(name: lift, duration: 350ms) {
        translate-y: 0 -> -6px;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let on_match = ast.matches.iter().find(|m| m.macro_name == "on");
        // PLAN-124 W3.4: the bare `@on hover <name>(…)` dispatcher was RETIRED;
        // the sigil head `@on-driver-body` answers `@on &.hover(name: …, …)`.
        // The driver capture now carries the event as its member, and the
        // timeline name is a named driver param.
        let on_match = on_match.expect("Should have an 'on' FormMatch");
        assert_eq!(
            on_match.matched_macro.as_deref(),
            Some("on-driver-body"),
            "@on &.hover should match the driver body head"
        );

        // The event is the driver member: `&.hover` → member = "hover".
        let driver = match on_match.captures.get("driver") {
            Some(CapturedValue::Named(m)) => m,
            _ => panic!("Should have a Named 'driver' capture"),
        };
        match driver.get("member") {
            Some(CapturedValue::Ident(member)) => assert_eq!(member, "hover"),
            _ => panic!("driver.member should be an Ident"),
        }

        // The timeline name is a named driver param: `name: lift`.
        let params = match driver.get("driver_params") {
            Some(CapturedValue::Array(a)) => a,
            _ => panic!("driver.driver_params should be an Array"),
        };
        let name_val = params.iter().find_map(|p| match p {
            CapturedValue::Named(n) => match (n.get("name"), n.get("value")) {
                (Some(CapturedValue::Ident(k)), Some(v)) if k == "name" => Some(v),
                _ => None,
            },
            _ => None,
        });
        match name_val {
            Some(CapturedValue::Expr(v)) => assert_eq!(v, "lift", "Name param should be 'lift'"),
            _ => panic!("driver param 'name' should hold Expr(\"lift\")"),
        }
    }

    #[test]
    fn test_parse_on_click_mutation_in_scope() {
        // Parse @on click mutation handler (no animation, just state mutations)
        let input = r#"
.pack-card {
    @on &.click {
        $currentId <- $.dataset.packId;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let on_match = ast.matches.iter().find(|m| m.macro_name == "on");
        // The sigil body head (`@on-driver-body`) answers this form; the bare
        // event dispatcher (on-mutation-dispatcher) was RETIRED in the SIP-001
        // cutover. The event is now the driver member.
        let on_match = on_match.expect("Should have an 'on' FormMatch");
        assert_eq!(
            on_match.matched_macro.as_deref(),
            Some("on-driver-body"),
            "@on &.click mutation should match the driver body head"
        );

        // Verify the driver capture carries the event as its member (`&.click`)
        let driver = match on_match.captures.get("driver") {
            Some(CapturedValue::Named(m)) => m,
            _ => panic!("Should have a Named 'driver' capture"),
        };
        match driver.get("member") {
            Some(CapturedValue::Ident(member)) => assert_eq!(member, "click"),
            _ => panic!("driver.member should be an Ident"),
        }

        // Verify body is captured (contains js_statements with mutations)
        assert!(
            on_match.captures.contains_key("body"),
            "Should have 'body' capture"
        );

        // Verify selector
        assert_eq!(on_match.selector.as_deref(), Some(".pack-card"));
    }

    #[test]
    fn test_parse_on_click_in_nested_scope_keeps_compound_selector() {
        // BUG-206 regression: a directive FormMatch sitting in a DEEPLY nested
        // plain (non-template) selector scope must keep the COMPOUND selector down
        // to the innermost scope (`.counter > .controls > button.inc`), the same
        // path the CSS pipeline composes for text/style bindings. Pre-fix the match
        // was hoisted to the OUTERMOST scope (`.counter`), so the event binding
        // registered on `.counter` and one click fired every handler bound under
        // it (inc + dec -> net zero; against a server: double-dispatch).
        let input = r#"
.counter {
    > .controls {
        > button.inc {
            @on &.click {
                $count <- $count + 1;
            }
        }
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let on_match = ast.matches.iter().find(|m| m.macro_name == "on");
        let on_match = on_match.expect("Should have an 'on' FormMatch");
        assert_eq!(
            on_match.matched_macro.as_deref(),
            Some("on-driver-body"),
            "@on &.click should match the driver body head"
        );

        // The COMPOUND path down to the innermost scope (root → .controls → button.inc),
        // NOT the hoisted outer `.counter`. Asserted without baking the exact
        // combinator: the CST preserves `>` on the first hop (`.counter > .controls`)
        // but the composer faithfully mirrors whatever leaf text the CST stores, so
        // we check the path is compound + non-hoisted rather than a brittle literal.
        let sel = on_match
            .selector
            .as_deref()
            .expect("@on must have a selector");
        assert!(
            sel.contains(".counter") && sel.contains(".controls") && sel.contains("button.inc"),
            "BUG-206: nested @on must carry the compound path to the inner element, got {sel:?}"
        );
        assert_ne!(
            sel, ".counter",
            "BUG-206: nested @on must NOT hoist to the outer scope"
        );
    }

    #[test]
    fn test_parse_multiple_scope_macros() {
        // Multiple macros in same scope
        let input = r#"
.card {
    @scroll card-reveal(start: 0.1, end: 0.3) {
        opacity: 0 -> 1;
    }

    @on hover card-lift(250ms) {
        translate-y: 0 -> -8px;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        // Should have both scroll and on FormMatches
        let scroll_count = ast
            .matches
            .iter()
            .filter(|m| m.macro_name == "scroll")
            .count();
        let on_count = ast.matches.iter().filter(|m| m.macro_name == "on").count();

        assert_eq!(scroll_count, 1, "Should have one scroll FormMatch");
        assert_eq!(on_count, 1, "Should have one on FormMatch");
    }

    /// IGNORED — FEATURE GAP opened by the SIP-001 cutover (bare-event
    /// `@on <event>` → sigil `@on &.driver`). The old `@on click .child { … }`
    /// delegation spelling was declared by the RETIRED `on-mutation-dispatcher`,
    /// whose surviving head `on-driver-body` (`@on $driver $as? { $body }`) has
    /// NO delegation-target slot between the driver and the body. A selector in
    /// that position (`@on &.click [data-option-id] { … }`) now parses as a
    /// NESTED SCOPE BLOCK inside the body and the driver binds to `&self` — the
    /// delegation target is silently absorbed, never captured. This needs a
    /// ticket (delegated-event binding on descendants is a real feature).
    #[ignore = "FEATURE GAP: @on-driver-body has no delegation-target slot; the selector is absorbed into the body. Needs a ticket."]
    #[test]
    fn test_parse_on_mutation_with_delegation() {
        // @on with event delegation (nested attribute selector from jallete pattern)
        // These are mutation handlers, not animation timelines
        let input = r#"
.variant-grid {
    @on &.click [data-option-id] {
        $selectedVariant <- $.dataset.optionId;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let on_match = ast.matches.iter().find(|m| m.macro_name == "on");
        let on_match = on_match.expect("Should have an 'on' FormMatch");
        // The driver body head answers this form, but WITHOUT a delegation
        // target: `[data-option-id]` is parsed as a nested body scope block and
        // the driver binds to `&self`. This is the FEATURE GAP the #[ignore]
        // documents — the old `on-mutation-dispatcher` had a target slot.
        assert_eq!(
            on_match.matched_macro.as_deref(),
            Some("on-driver-body"),
            "@on &.click <target> matches the driver body head (target absorbed)"
        );

        // The event survives as the driver member (`&.click`).
        let driver = match on_match.captures.get("driver") {
            Some(CapturedValue::Named(m)) => m,
            _ => panic!("Should have a Named 'driver' capture"),
        };
        match driver.get("member") {
            Some(CapturedValue::Ident(member)) => assert_eq!(member, "click"),
            _ => panic!("driver.member should be an Ident"),
        }

        // GAP: there is NO delegation-target capture — the old `target` capture
        // is gone with on-mutation-dispatcher. The selector is absorbed into the
        // body scope, so no descendant delegation is expressed.
        assert!(
            !on_match.captures.contains_key("target"),
            "GAP: driver form has no delegation 'target' capture (selector absorbed into body)"
        );
    }

    #[test]
    fn test_full_pipeline_with_parsed_scroll() {
        // Full integration: parse -> FormMatch -> pipeline compile
        let input = r#"
.element {
    @scroll fade-in(start: 0.1, end: 0.5) {
        opacity: 0 -> 1;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");
        let context = CompileContext::new(MetaRegistry::new()).without_runtime();

        // This should not panic
        let result = compile(&ast.matches, &context);

        // We expect this to succeed or return an error (not panic)
        match result {
            Ok(output) => {
                // Success - verify we got some output
                assert!(
                    !output.js.is_empty() || !output.css.is_empty() || ast.matches.is_empty(),
                    "Should produce some output"
                );
            }
            Err(e) => {
                // Error is acceptable (e.g., macro not fully defined in stdlib)
                // The important thing is we didn't panic
                println!("Pipeline error (acceptable): {:?}", e);
            }
        }
    }

    // ==========================================================================
    // @loop positional duration tests (ITEM-031)
    // ==========================================================================

    #[test]
    fn test_loop_positional_duration() {
        // @loop spin(6s) { ... } should parse with positional duration
        let input = r#"
.spinner {
    @loop spin(6s) {
        opacity: 0 -> 1;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse @loop with positional duration");

        let loop_match = ast
            .matches
            .iter()
            .find(|m| m.macro_name == "loop")
            .expect("Should have a 'loop' FormMatch");

        // Verify "name" capture
        assert!(
            loop_match.captures.contains_key("name"),
            "Should have 'name' capture, got: {:?}",
            loop_match.captures.keys().collect::<Vec<_>>()
        );

        if let CapturedValue::Ident(name) = loop_match.captures.get("name").unwrap() {
            assert_eq!(name, "spin", "Name should be 'spin'");
        } else {
            panic!("Name capture should be an Ident");
        }

        // Verify "duration" is captured (positional arg)
        assert!(
            loop_match.captures.contains_key("duration"),
            "Should have 'duration' capture for positional time arg, got: {:?}",
            loop_match.captures.keys().collect::<Vec<_>>()
        );

        // PLAN-122 W1.2: a scalar's captured repr is its SOURCE TEXT, not a
        // pre-parsed struct. `6s` stays `"6s"` through the capture; the
        // conversion to milliseconds happens at the boundary that needs it (the
        // animation runtime emits 6000 — asserted end-to-end, not here). One
        // representation serves CSS, JSON seeds and admin widgets alike.
        if let CapturedValue::String(text) = loop_match.captures.get("duration").unwrap() {
            assert_eq!(text, "6s", "Duration should keep its source spelling");
        } else {
            panic!(
                "Duration capture should be the source text String, got: {:?}",
                loop_match.captures.get("duration")
            );
        }

        // Verify selector
        assert_eq!(loop_match.selector.as_deref(), Some(".spinner"));
    }

    #[test]
    fn test_loop_named_duration() {
        // @loop spin(duration: 6s) { ... } should still work (no regression)
        let input = r#"
.spinner {
    @loop spin(duration: 6s) {
        rotate: 0deg -> 360deg;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse @loop with named duration");

        let loop_match = ast
            .matches
            .iter()
            .find(|m| m.macro_name == "loop")
            .expect("Should have a 'loop' FormMatch");

        // Verify "name" capture
        if let CapturedValue::Ident(name) = loop_match.captures.get("name").unwrap() {
            assert_eq!(name, "spin", "Name should be 'spin'");
        } else {
            panic!("Name capture should be an Ident");
        }

        // Verify "duration" is captured (named arg)
        assert!(
            loop_match.captures.contains_key("duration"),
            "Should have 'duration' capture for named time arg, got: {:?}",
            loop_match.captures.keys().collect::<Vec<_>>()
        );

        // PLAN-122 W1.2: a scalar's captured repr is its SOURCE TEXT, not a
        // pre-parsed struct. `6s` stays `"6s"` through the capture; the
        // conversion to milliseconds happens at the boundary that needs it (the
        // animation runtime emits 6000 — asserted end-to-end, not here). One
        // representation serves CSS, JSON seeds and admin widgets alike.
        if let CapturedValue::String(text) = loop_match.captures.get("duration").unwrap() {
            assert_eq!(text, "6s", "Duration should keep its source spelling");
        } else {
            panic!(
                "Duration capture should be the source text String, got: {:?}",
                loop_match.captures.get("duration")
            );
        }
    }

    #[test]
    fn test_loop_positional_with_nested_targets() {
        // @loop name(2s) { .child { scale: 0 -> 1; } } nested targets
        let input = r#"
.parent {
    @loop grow(2s) {
        .child {
            scale: 0 -> 1;
        }
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse @loop with nested targets");

        let loop_match = ast
            .matches
            .iter()
            .find(|m| m.macro_name == "loop")
            .expect("Should have a 'loop' FormMatch");

        if let CapturedValue::Ident(name) = loop_match.captures.get("name").unwrap() {
            assert_eq!(name, "grow", "Name should be 'grow'");
        } else {
            panic!("Name capture should be an Ident");
        }

        assert!(
            loop_match.captures.contains_key("duration"),
            "Should have 'duration' capture, got: {:?}",
            loop_match.captures.keys().collect::<Vec<_>>()
        );

        // PLAN-122 W1.2: a scalar's captured repr is its SOURCE TEXT, not a
        // pre-parsed struct. `2s` stays `"2s"` through the capture; the
        // conversion to milliseconds happens at the boundary that needs it (the
        // animation runtime emits 2000 — asserted end-to-end, not here). One
        // representation serves CSS, JSON seeds and admin widgets alike.
        if let CapturedValue::String(text) = loop_match.captures.get("duration").unwrap() {
            assert_eq!(text, "2s", "Duration should keep its source spelling");
        } else {
            panic!(
                "Duration capture should be the source text String, got: {:?}",
                loop_match.captures.get("duration")
            );
        }

        // Verify body is captured
        assert!(
            loop_match.captures.contains_key("body"),
            "Should have 'body' capture for keyframes"
        );
    }

    #[test]
    fn test_loop_default_duration() {
        // @loop name { ... } should use default 1000ms
        let input = r#"
.pulsing {
    @loop pulse {
        opacity: 1 -> 0.7;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse @loop with default duration");

        let loop_match = ast
            .matches
            .iter()
            .find(|m| m.macro_name == "loop")
            .expect("Should have a 'loop' FormMatch");

        if let CapturedValue::Ident(name) = loop_match.captures.get("name").unwrap() {
            assert_eq!(name, "pulse", "Name should be 'pulse'");
        } else {
            panic!("Name capture should be an Ident");
        }

        // Selector should be captured
        assert_eq!(loop_match.selector.as_deref(), Some(".pulsing"));
    }

    #[test]
    fn test_jallete_style_content_no_panic() {
        // Parse a snippet similar to jallete/index.st - should not panic
        let input = r#"
// Type and data definitions
@type PackItem {
    label: string;
    price: number;
}

@data packs: PackItem[] {
    src: "/data/packs.json";
}

body {
    $cart CartItem[]: localStorage("jallete-cart", []);
    $currentPackId string: "";
}

.story-image {
    @scroll story-image-reveal(start: 0.1, end: 0.3) {
        opacity: 0 -> 1;
        scale: 1.05 -> 1;
    }

    @on hover story-image-lift(350ms) {
        translate-y: 0 -> -6px;
    }
}

.pack-card {
    @scroll pack-card-reveal(start: 0.1, end: 0.3) {
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
    }

    @on &.click {
        $currentPackId <- $.dataset.packId;
    }
}
"#;

        // Parse should succeed
        let ast = parse_to_ast(input).expect("Failed to parse jallete-style content");

        // Should have collected FormMatches
        assert!(
            ast.matches.len() >= 4,
            "Should have at least 4 FormMatches (scroll + on for each element), got {}",
            ast.matches.len()
        );

        // Try pipeline - should not panic
        let context = CompileContext::new(MetaRegistry::new()).without_runtime();
        let result = compile(&ast.matches, &context);

        // Result doesn't matter - we're testing no panic
        match result {
            Ok(_) => println!("Pipeline succeeded"),
            Err(e) => println!("Pipeline error (no panic): {:?}", e),
        }
    }

    // ==========================================================================
    // %creates removal verification tests
    // ==========================================================================

    #[test]
    fn test_scroll_recognized_from_stdlib() {
        use crate::parser::parse as parse_to_ast;

        let input = r#"
.x {
    @scroll hero(start: 0, end: 1) {
        opacity: 0 -> 1;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        // @scroll should produce a FormMatch — not be flagged as unknown
        let scroll_match = ast.matches.iter().find(|m| m.macro_name == "scroll");
        assert!(
            scroll_match.is_some(),
            "Should have a 'scroll' FormMatch. Got: {:?}",
            ast.matches
                .iter()
                .map(|m| &m.macro_name)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_value_change_recognized_from_stdlib() {
        use crate::parser::parse as parse_to_ast;

        // The LEADING BAREWORD form (`@value-change count(...)`) is gone. It was
        // never a declared grammar — `%macro value-change-timeline`'s `%form` is
        // named-only — it merely slipped through the "extra tokens before the arg
        // list" tolerance, which existed for forms with inline elements and
        // accidentally covered zero-inline forms too. That same tolerance is what
        // let `@object torusknot(...)` compile to a silent default (GH-14), so it
        // now reads a macro's DECLARED `%drops` instead of guessing.
        //
        // Nothing authored uses the bareword spelling: every `@value-change` in
        // examples/, tests/ and stdlib docs is the named form asserted here. The
        // `%drops ( _ )` that motivated the old tolerance lives on the migration
        // REWRITE, not on the macro, so the rewrite still consumes the positional
        // when migrating pre-cutover source.
        let input = r#"
.counter {
    @value-change(duration: 500ms) {
        font-size: 14px -> 24px;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let vc_match = ast.matches.iter().find(|m| m.macro_name == "value-change");
        assert!(
            vc_match.is_some(),
            "Should have a 'value-change' FormMatch. Got: {:?}",
            ast.matches
                .iter()
                .map(|m| &m.macro_name)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_value_change_driver_guards_container_width_by_display_level() {
        // BUG-178 (supersedes the PLAN-072-era version of this test): the
        // rewrite's FIRST containerWidth fix classified purely by the
        // element's OWN `display` value — `elIsBlockLevel ? clientWidth :
        // Infinity` — assuming an inline-level auto-width element (e.g. a
        // `display: inline-block` "flap" word-cycle) never wraps because it
        // never force-wraps ITSELF. That classification missed that the
        // element can still SIT inside an ancestor that constrains where its
        // own line runs (a `display: block` sibling above/below forcing it
        // onto its own line within a fixed-width parent) — the real-world
        // ora-ventures.com hero flap is exactly this shape. Reproduced live
        // via CDP: at a 340px-constrained container the flap's real
        // (browser-computed) box wraps to 2–3 lines, but the driver, always
        // assuming `Infinity`, designed a single-line transition every time —
        // the instant `%&el.style.overflow`/cleanup exposed the browser's
        // real (wrapped) layout, the box visibly SNAPPED from 1 line to N
        // lines instead of easing ("shows as one line then jumps").
        //
        // Fix: measure the REAL constraint for every element via a
        // display/white-space toggle (forces the element alone onto a block
        // box at its actual DOM position, reading the true available width
        // from its nearest constraining ancestor — the browser's own layout
        // engine, not a display-type heuristic), gated only by the element's
        // own RESTING `white-space` (an author-set `nowrap` is an explicit
        // opt-out of wrapping, e.g. a single-line counter that must never
        // wrap regardless of container width).
        //
        // This test asserts on the PRIMITIVE'S EMITTED JS directly (not via a
        // CDP DOM measurement — the CDP test harness was found to have
        // separate, unrelated bugs around @mount-scoped CSS application and
        // WAAPI-driven height-animation timing that make an end-to-end pixel
        // assertion unreliable there; filed separately as BUG-173/BUG-174).
        // Asserting on the compiled driver source is the most direct, stable
        // proof this specific fix is present and has not regressed.
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("var measureRealConstraint = function() {")
                && stdlib_src.contains("%&el.style.display = 'block';")
                && stdlib_src.contains("%&el.style.whiteSpace = 'normal';")
                && stdlib_src.contains("var w = %&el.clientWidth;"),
            "value-change-driver must measure the REAL wrap constraint via a display/white-space \
             toggle (forcing the element alone onto a block box at its actual position), not infer \
             it from the element's own display-type alone — an inline-level auto-width element can \
             still sit inside a width-constraining ancestor."
        );
        assert!(
            stdlib_src.contains("var elRestingWhiteSpace = cs.whiteSpace;")
                && stdlib_src.contains("var containerWidth = elRestingWhiteSpace === 'nowrap' ? Infinity : measureRealConstraint();"),
            "containerWidth must be Infinity ONLY when the element's own resting (author-set) \
             white-space is 'nowrap' — an explicit opt-out of wrapping — and otherwise use the \
             real measured constraint, not a display-type heuristic."
        );
        assert!(
            stdlib_src.contains("if (!(containerWidth > 0)) { containerWidth = Infinity; }"),
            "containerWidth must fall back to Infinity when the real-constraint measurement yields \
             a degenerate (zero/negative) width, e.g. a detached or hidden element."
        );
    }

    #[test]
    fn test_value_change_driver_animates_width_alongside_height_for_inline_auto_width() {
        // Follow-up to the above fix: an inline-level auto-width "flap"
        // element's own outer box WIDTH also needs to change when its text
        // does (a wider/narrower word naturally has a wider/narrower
        // shrink-to-fit box) — but the driver only animated `height` via
        // WAAPI, leaving `width` to snap INSTANTLY the moment the DOM swap
        // happens (the new text's wrapper is appended in-flow, growing the
        // box to its natural width in one frame, well before the eased
        // height tween finishes). Reproduced live: `.ora-hero__flap`'s
        // `getBoundingClientRect().width` jumped 519.78px -> 624.03px in a
        // single 100ms sample tick at transition start (no intermediate
        // frames), and again 624.03px -> 621.83px at cleanup — a visible
        // "everything nearby jumps" artifact distinct from the false-2-line-
        // wrap bug the block-level guard above fixes (that one affects
        // HEIGHT; this one affects WIDTH, and can occur even when the
        // block-level guard is doing its job correctly).
        //
        // Fix: measure the new text's natural (single-line) width via
        // pretext (`ST.text.measureNaturalWidth`, added alongside the
        // existing `measureWord`), pin the element's width to its CURRENT
        // value at transition start (with `white-space: nowrap` so the
        // pinned-narrower width doesn't force a REAL wrap of the per-word
        // DOM spans), then animate BOTH `width` and `height` in a single
        // WAAPI `%&el.animate([...])` call so they stay in lockstep rather
        // than drifting independently.
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("measureNaturalWidth"),
            "value-change-driver's ST.text namespace must expose a natural-width \
             measurement (measureNaturalWidth) for animating an auto-width \
             element's box width, not just measureWord for per-word exit-layer sizing."
        );
        assert!(
            stdlib_src.contains("targetWidth") && stdlib_src.contains("currentWidth"),
            "value-change-driver must compute a targetWidth (from the new text's \
             natural width) and compare it against currentWidth, mirroring the \
             existing targetHeight/currentHeight height-animation logic."
        );
        assert!(
            stdlib_src.contains("el.style.whiteSpace = 'nowrap'"),
            "value-change-driver must pin white-space to nowrap while width is \
             locked to its pre-transition value, so the pinned (narrower) width \
             doesn't force a real wrap of the per-word DOM spans before the \
             width animation eases the box open."
        );
        assert!(
            stdlib_src.contains("boxFrom")
                && stdlib_src.contains("boxTo")
                && stdlib_src.contains("needsBoxAnim"),
            "value-change-driver must animate height AND width via a single WAAPI \
             animate() call (shared keyframe object), not two independent animations that could visually drift out of sync."
        );
    }

    #[test]
    fn test_value_change_driver_pins_vertical_align_to_stop_baseline_flip() {
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("isAtomicInlineBox"),
            "value-change-driver must classify the target element as an atomic inline-level box before deciding whether the overflow-toggle baseline coupling applies to it"
        );
        assert!(
            stdlib_src.contains("el.style.verticalAlign = 'bottom'"),
            "value-change-driver must pin vertical-align to a fixed value once at mount time for atomic inline-level boxes, so the overflow-visible-vs-not baseline coupling never applies"
        );
        assert!(
            stdlib_src.contains("initialCs.verticalAlign === 'baseline'"),
            "the vertical-align pin must be conditioned on the element's current computed vertical-align being the default baseline, respecting an author's own explicit choice"
        );
    }

    #[test]
    fn test_value_change_driver_uses_native_pretext_letter_spacing() {
        // Superseded by the line-aware rewrite (PLAN-072): the arithmetic
        // letter-spacing correction this test originally guarded (bolted onto
        // a private, stale, inlined pretext copy that lacked native
        // letter-spacing support) is GONE — the rewrite deleted the driver's
        // ~700-line embedded pretext copy entirely and switched to the
        // shared, current `stdlib/text` vendored bundle (referenced as the
        // global `pretext`, demand-emitted via `stdlib/macros/timeline.st`'s
        // `@import "stdlib/text"`), which has NATIVE `letterSpacing` support
        // in its `prepare`/`prepareWithSegments` options (verified against
        // `stdlib/text/vendor/pretext.bundle.js` source directly: `letterSpacing`
        // and `spacingGraphemeCounts` are threaded through the whole
        // measurement pipeline, unlike the driver's old embedded copy whose
        // exported surface — `clearCache, layout, layoutNextLine,
        // layoutWithLines, prepare, prepareWithSegments, profilePrepare,
        // setLocale, walkLineRanges` — had no letterSpacing param at all).
        //
        // This test guards the NEW mechanism: every pretext `prepareWithSegments`
        // call site threads `letterSpacingPx` through as a native `{ letterSpacing }`
        // option, and that value is read from the target element's OWN computed
        // style each transition cycle (still needed — letter-spacing can change
        // via responsive breakpoints just like font/lineHeight can).
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            !stdlib_src.contains("__Pretext") && !stdlib_src.contains("ST.text"),
            "value-change-driver must no longer embed a private pretext copy or expose an ST.text \
             namespace — it should reference the shared `pretext` global from stdlib/text instead"
        );
        assert!(
            stdlib_src.contains("var lsRaw = parseFloat(cs.letterSpacing);")
                && stdlib_src.contains("var letterSpacingPx = isNaN(lsRaw) ? 0 : lsRaw;"),
            "value-change-driver must re-read the target element's computed letter-spacing each \
             transition cycle, alongside font/lineHeight, for the same responsive-breakpoint reason"
        );
        assert!(
            stdlib_src.contains("letterSpacingPx ? { letterSpacing: letterSpacingPx } : undefined"),
            "value-change-driver must thread letterSpacingPx through to pretext's NATIVE \
             letterSpacing prepare option, not an after-the-fact arithmetic correction"
        );
        assert!(
            stdlib_src.contains("pretext.prepareWithSegments(text, font, opts)"),
            "measureNatural/layoutLines must call the shared pretext.prepareWithSegments \
             (the global from stdlib/text), not a private inlined copy"
        );
    }

    #[test]
    fn test_value_change_driver_computes_real_line_layout_before_dom_mutation() {
        // PLAN-072: the line-aware rewrite's core architectural fix. The
        // previous box-hack version NEVER asked "how many lines will this
        // text take" against a real container width — it always laid out
        // text at `Infinity` width (forcing single-line), then papered over
        // the gap with box-level height/width snap fixes (BUG-172), baseline
        // toggling guards (BUG-175), and letter-spacing corrections
        // (BUG-176). This test guards that the rewrite computes BOTH the old
        // and new text's real line layout (against the actual container
        // width, not Infinity) BEFORE mutating the DOM — the "will this wrap
        // to two lines" question is answered, not inferred after the fact.
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("var oldLayout = layoutLines(oldText, font, containerWidth, lineHeight, letterSpacingPx);")
                && stdlib_src.contains("var newLayout = layoutLines(newText, font, containerWidth, lineHeight, letterSpacingPx);"),
            "value-change-driver must compute layoutLines for BOTH oldText and newText against the \
             SAME real containerWidth, before any DOM mutation happens"
        );
        // BUG-178: containerWidth itself must come from the REAL measured
        // constraint (a display/white-space toggle against the element's
        // actual DOM position), not a display-type heuristic — see
        // test_value_change_driver_guards_container_width_by_display_level
        // for the full root cause. This assertion just guards that
        // `layoutLines` for both old/new text is fed THAT real `containerWidth`
        // variable (whatever computed it), not a hardcoded Infinity/offsetWidth.
        assert!(
            stdlib_src.contains("var containerWidth = elRestingWhiteSpace === 'nowrap' ? Infinity : measureRealConstraint();"),
            "containerWidth must be derived from a real measured constraint (falling back to \
             Infinity only for an author-set nowrap opt-out), not offsetWidth (read AFTER text \
             changed) or a display-type heuristic that misses inline-level elements confined by a \
             constraining ancestor"
        );
    }

    #[test]
    fn test_value_change_driver_has_line_aware_reflow_path() {
        // PLAN-072: when either the old or new text spans more than one
        // line, the driver must take the WORD-level reflow path (diff at the
        // word/line/x/y grid, glide persisting-but-relocated words via WAAPI
        // transform) rather than the flat prefix/suffix character-diff path
        // (which has no concept of which line a word is on and cannot
        // distinguish "word moved to the next line" from "word's characters
        // changed").
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("if (oldLayout.lineCount <= 1 && newLayout.lineCount <= 1)"),
            "value-change-driver must branch on real line counts (single-line fast path vs \
             multi-line reflow path), not always take the single-line path"
        );
        assert!(
            stdlib_src
                .contains("var positionWords = function(lines, font, lineHeight, letterSpacingPx)"),
            "the reflow path must compute per-word (line, x, y) coordinates from the real line \
             layout, the foundation for diffing WHERE a persisting word sits, not just whether \
             its text changed"
        );
        assert!(
            stdlib_src.contains("var diffWordSequence = function(oldWords, newWords)"),
            "the reflow path must diff at the WORD-sequence level (LCS-style keep/remove/add), so \
             a persisting word can be identified as MOVED (different line/x) rather than being \
             blindly torn down and rebuilt like every other changed word"
        );
        assert!(
            stdlib_src.contains("'translate(' + (-dx) + 'px, ' + (-dy) + 'px)'"),
            "a persisting-but-relocated word ('keep' op with different old/new x or y) must glide \
             via a real 2D WAAPI transform animation from its old position to its new one — this \
             is the literal 'text should know it's going to wrap and smoothly reorganize' behavior"
        );
    }

    #[test]
    fn test_value_change_driver_bounds_reflow_stagger_by_word_count_not_character_count() {
        // Regression: an early version of the line-aware rewrite bounded the
        // multi-line reflow path's per-character stagger by a GLOBAL running
        // character index across ALL changed words (mirroring the single-line
        // path's flat per-character array) — for a large content change (many
        // words differing at once, e.g. a full sentence rewrite), that scaled
        // `totalAnimDur` with total changed CHARACTERS, not changed WORDS.
        // Reproduced live: a ~66-character, 15-word sentence replacement took
        // 2.5+ seconds to settle instead of the intended lean, bounded
        // duration. Fixed by staggering each changed WORD's characters
        // relative to a per-WORD delay offset (`changedWordIndex * stagger`),
        // so total duration scales with word count — a much smaller, more
        // meaningful bound for a "lean" reflow transition.
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("var changedWordIndex = 0;")
                && stdlib_src.contains("changedWordIndex++;"),
            "the reflow path must track a per-CHANGED-WORD index (incremented once per add/remove \
             op), not a running character count, to bound cross-word stagger"
        );
        assert!(
            stdlib_src.contains("var wordDelay = changedWordIndex * stagger;")
                || stdlib_src.contains("var wordDelay2 = changedWordIndex * stagger;"),
            "each changed word's base stagger delay must be `changedWordIndex * stagger` (bounded \
             by word count), passed as applyCharStagger's baseDelay parameter — not each \
             character's flat index within a shared, unbounded array spanning every changed word"
        );
    }

    #[test]
    fn test_value_change_driver_measures_old_geometry_from_computed_layout_not_live_dom() {
        // Regression, found during the line-aware rewrite's own live testing
        // (not a pre-existing bug — the box-hack version never needed
        // accurate PRE-mutation geometry since a single line's height never
        // changes with content). `animateChange` runs from a
        // MutationObserver callback, which by definition only fires AFTER
        // the DOM mutation (textContent -> newText) has ALREADY happened.
        // Reading `%&el.offsetHeight`/`%&el.offsetWidth` at that point
        // silently measures the NEW text's box while the code intends to
        // read the OLD (pre-change) box — reproduced live: a 2-line-to-
        // 1-line transition's box snapped INSTANTLY to the 1-line height
        // instead of easing down from the 2-line height, because
        // `currentHeight` was accidentally reading the ALREADY-1-line
        // offsetHeight. Fixed by deriving currentHeight/currentWidth from
        // `oldLayout` (computed from the `oldText` STRING via layoutLines/
        // measureNatural — a pure function, immune to DOM mutation timing),
        // falling back to a live DOM read only when the computed layout is
        // degenerate (0 height, e.g. truly empty oldText).
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains(
                "var currentHeight = oldLayout.height > 0 ? oldLayout.height : liveHeightFallback;"
            ),
            "currentHeight must be derived from oldLayout (computed from the oldText STRING, \
             immune to DOM mutation timing), not a live offsetHeight read alone — by the time the \
             MutationObserver callback runs, offsetHeight already reflects the NEW text"
        );
        assert!(
            stdlib_src.contains("var liveHeightFallback = %&el.offsetHeight;")
                && stdlib_src.contains("var liveWidthFallback = %&el.offsetWidth;"),
            "a live DOM read must still exist as a FALLBACK for degenerate computed layouts (e.g. \
             truly empty oldText, where layoutLines returns 0 height)"
        );
    }

    #[test]
    fn test_value_change_driver_applies_letter_spacing_to_char_layer() {
        // BUG-178: `buildCharLayer` splits a word into one `inline-block`
        // span PER GRAPHEME so each character can animate independently —
        // but per-element inline-block boxes do NOT inherit letter-spacing
        // the way a single text run does. Reproduced live (direct DOM
        // construction test against the ora-ventures.com hero, letter-
        // spacing: -1.911px): the word "stalls" measured 232.19px as a
        // single text node (matching the pretext-corrected width used for
        // ALL layout math — measureNatural/layoutLines/positionWords), but
        // 245.20px as buildCharLayer's uncorrected char-span construction —
        // a ~13px-per-word discrepancy between the width the driver ANIMATES
        // toward and the width the driver's own char layer ACTUALLY renders
        // at, that only self-corrected the instant cleanup swapped back to
        // real textContent (a visible width snap at the end of every
        // transition, compounding with more changed words).
        //
        // Fix: thread `letterSpacingPx` into buildCharLayer and set
        // `letter-spacing` on the LAYER wrapper (not each character span) —
        // verified empirically to reproduce the single-text-node width
        // (233.75px vs 232.19px reference, matching the SAME baseline
        // inline-block overhead the ls=0 case already had, not a residual
        // letter-spacing error).
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("var buildCharLayer = function(word, kind, letterSpacingPx) {"),
            "buildCharLayer must accept a letterSpacingPx parameter"
        );
        assert!(
            stdlib_src.contains(
                "if (letterSpacingPx) { layer.style.letterSpacing = letterSpacingPx + 'px'; }"
            ),
            "buildCharLayer must apply letter-spacing to its LAYER wrapper so the split char spans \
             render at the same width as an equivalent single text run"
        );

        // All 4 call sites (single-line path's enter/exit words, reflow
        // path's add/remove words) must thread the SAME letterSpacingPx
        // through — a partial fix (e.g. only the single-line path) would
        // leave the reflow path's added/removed words silently mismeasured.
        let call_sites = [
            "buildCharLayer(newWord, 'enter', letterSpacingPx);",
            "buildCharLayer(oldWord, 'exit', letterSpacingPx);",
            "buildCharLayer(op.oldW.word, 'exit', letterSpacingPx);",
            "buildCharLayer(op.newW.word, 'enter', letterSpacingPx);",
        ];
        for call_site in call_sites {
            assert!(
                stdlib_src.contains(call_site),
                "buildCharLayer call site must thread letterSpacingPx: {}",
                call_site
            );
        }
    }

    #[test]
    fn test_value_change_driver_measures_real_wrap_constraint_not_display_heuristic() {
        // BUG-178: the previous containerWidth fix (BUG-172-era) classified
        // wrap eligibility purely from the target element's OWN `display`
        // value (`elIsBlockLevel ? clientWidth : Infinity`) — correct for a
        // `<div>` counter, WRONG for an inline-level auto-width element that
        // nonetheless sits inside a width-constraining ancestor (e.g. the
        // ora-ventures.com hero's `.ora-hero__flap`: `display: inline-block`,
        // sole occupant of its own line between two `display: block`
        // siblings inside a fixed-width `h1`). Reproduced live via CDP at a
        // 340px-constrained container: forcing `display: block` +
        // `white-space: normal` directly on the flap (the ground-truth wrap
        // test) measured a real 340px available width and a 3-line wrap —
        // but the driver's `elIsBlockLevel` check classified this exact
        // element as `Infinity` (never wraps), so it always DESIGNED a
        // single-line transition. The instant cleanup exposed the browser's
        // real (wrapped) layout, producing a visible 1-line -> N-line SNAP
        // (100.31px -> 200.63px / 300.94px in a single ~18ms sample tick,
        // captured via requestAnimationFrame sampling across 12 transitions).
        //
        // Fix: measure the REAL constraint via a display/white-space toggle
        // (forces the element alone onto a block box at its ACTUAL DOM
        // position, then reads clientWidth — the true available width from
        // whatever ancestor actually constrains it, verified to exactly
        // match a ground-truth block-toggle measurement: 340px vs 340px, and
        // to be a no-op for an already-block-level element: 360px vs 360px
        // in a padded-box sanity check), immediately reverting the inline
        // style. `white-space: nowrap` (the element's own RESTING, author-set
        // value) remains the one legitimate Infinity case — an explicit
        // opt-out, not a display-type inference.
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("var measureRealConstraint = function() {")
                && stdlib_src.contains("var prevDisplay = %&el.style.display;")
                && stdlib_src.contains("var prevWhiteSpace = %&el.style.whiteSpace;")
                && stdlib_src.contains("%&el.style.display = 'block';")
                && stdlib_src.contains("%&el.style.whiteSpace = 'normal';")
                && stdlib_src.contains("var w = %&el.clientWidth;")
                && stdlib_src.contains("%&el.style.display = prevDisplay;")
                && stdlib_src.contains("%&el.style.whiteSpace = prevWhiteSpace;")
                && stdlib_src.contains("return w;"),
            "value-change-driver must define measureRealConstraint: toggle display:block + \
             white-space:normal, read clientWidth, then revert both inline styles"
        );
        assert!(
            stdlib_src.contains("var elRestingWhiteSpace = cs.whiteSpace;")
                && stdlib_src.contains(
                    "var containerWidth = elRestingWhiteSpace === 'nowrap' ? Infinity : measureRealConstraint();"
                ),
            "containerWidth must call measureRealConstraint() for every element EXCEPT one whose \
             own resting white-space is already 'nowrap' (an explicit author opt-out of wrapping)"
        );

        // targetWidth/currentWidth for an auto-width element must now use the
        // WIDEST LINE of the real (possibly multi-line) layout, not a single
        // measureNatural call that assumes exactly one line.
        assert!(
            stdlib_src.contains("var widestLine = function(layout, text) {"),
            "an auto-width element's currentWidth/targetWidth must be computed from the widest \
             line of its real (line-count-aware) layout, not measureNatural alone (which assumes \
             a single unwrapped line)"
        );
    }

    #[test]
    fn test_value_change_driver_exit_layer_has_no_explicit_kerned_width() {
        // BUG-178 follow-up: the single-line fast path's exit layer (each
        // changed word's OLD text, rendered via buildCharLayer's per-
        // grapheme inline-block spans) previously had an explicit `width`
        // pinned to `measureNatural(oldWord, ...)` — pretext's KERNED
        // measurement. But splitting a word into one inline-block span per
        // grapheme structurally prevents cross-glyph kerning (no CSS
        // property recovers it once glyphs are in separate boxes), so the
        // char-split layer always renders AT LEAST as wide as that kerned
        // value — verified live: "AVAV"/"WAWA"-class kerning pairs lose
        // 8-16px vs their real-text width; a plain repeated-char word like
        // "aaaaaaaaaa" (no kerning pairs) loses ~0px, isolating the effect
        // to kerning specifically, not letter-spacing (which the wrapper's
        // own CSS `letter-spacing` already applies correctly). Pinning the
        // exit layer's box to the NARROWER kerned width while its rendered
        // content was the WIDER un-kerned width clipped the tail of the
        // layer's own text against the flap's clip-path for the animation's
        // entire duration — not just a boundary snap (verified live: a full
        // word like "L" in "P&L." was cut off, opacity 1, fully visible,
        // mid-transition). `exit.layer` is `display:inline-block` +
        // `position:absolute`, so it ALREADY shrink-wraps to its own
        // rendered content width with no explicit `width` needed — letting
        // it auto-size is both simpler and correct.
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            !stdlib_src.contains(
                "exit.layer.style.width = measureNatural(oldWord, font, letterSpacingPx) + 'px';"
            ),
            "the single-line fast path's exit layer must NOT pin an explicit `width` to \
             pretext's kerned measureNatural() value — the char-split rendering is wider \
             (un-kerned) than that value, and pinning it clips the layer's own content"
        );
        assert!(
            stdlib_src.contains("var exit = buildCharLayer(oldWord, 'exit', letterSpacingPx);")
                && stdlib_src.contains("exit.layer.style.position = 'absolute';")
                && stdlib_src.contains("exit.layer.style.opacity = '1';"),
            "the exit layer must still be absolutely positioned with opacity set, just without \
             the explicit (wrong) width pin"
        );
    }

    #[test]
    fn test_value_change_driver_widens_clip_pad_to_real_leaf_span_overflow() {
        // BUG-178 follow-up (word-count/word-length delta case): a word-
        // container (`wc`, single-line fast path) is `display:inline-block`,
        // so its OWN box width comes only from its in-flow child — the enter
        // layer. The exit layer is `position:absolute` (so enter/exit can
        // cross-fade in place without pushing layout), which means it does
        // NOT contribute to `wc`'s width AT ALL. When a word shrinks or is
        // removed outright (fewer new words than old — e.g. "misses the
        // P&L." -> "never ships.", 3 words -> 2 words), the enter layer is
        // narrow/empty while the exit layer still paints the full old word
        // (wider still once kerning loss is added) with NOTHING reserving
        // space for it. Verified live on the real ora-ventures.com hero: the
        // third word-container's exit layer ("P&L") painted 114px past the
        // flap's own clip-path boundary — genuinely clipped, fully opaque
        // (not a sub-pixel artifact, not an animation-in-flight false
        // positive).
        //
        // A `minWidth` reservation on each `wc` was tried and rejected: the
        // OUTER box (`%&el`) legitimately eases its own width from
        // `currentWidth` toward `targetWidth` via WAAPI, and forcing the
        // wrapper's rendered content to stay at its full original width
        // fights that shrink — the gap (and visible clipped overflow) GREW
        // monotonically as the box narrowed (verified: 3.6px -> 168px within
        // one transition). The fix instead widens the CLIP PAD to the real
        // measured overflow: `position:absolute` elements don't contribute
        // to any ancestor's `getBoundingClientRect()` (not `wc`'s, not
        // `wrapper`'s), so the true painted extent is read directly from the
        // LEAF (grapheme) spans' own rects — the widest left/right excursion
        // across all of them relative to the wrapper's left edge — and used
        // to grow `clipPath`'s horizontal inset beyond the baseline
        // `overflowPad`. Verified live: 0 breaches across 1201 samples / 20s
        // spanning multiple word-count-varying transitions (down from 217
        // breach events, max 114px, before this fix).
        let stdlib_src = std::fs::read_to_string("stdlib/primitives/animation/drivers.st")
            .expect("read drivers.st");

        assert!(
            stdlib_src.contains("var leafSpans = wrapper.querySelectorAll('span');")
                && stdlib_src.contains("var wrapperLeft = wrapper.getBoundingClientRect().left;"),
            "the horizontal clip-pad widening must measure real painted extent from leaf \
             (grapheme) spans, not from wrapper/word-container bounding rects (which do not \
             include position:absolute descendants like the exit layer)"
        );
        assert!(
            stdlib_src.contains("if (leafSpans[lsI].children.length > 0) continue;"),
            "the leaf-span walk must skip non-leaf spans (word-containers, layer wrappers) and \
             only measure actual grapheme spans"
        );
        assert!(
            stdlib_src.contains("var narrowestBoxWidth = elIsBlockLevel ? currentWidth : Math.min(currentWidth, targetWidth);"),
            "the overflow calculation must compare against the NARROWER of currentWidth/targetWidth \
             — the box's own WAAPI width tween can end at either value, and the worst-case clip \
             gap occurs at whichever one is smaller"
        );
        assert!(
            stdlib_src.contains("%&el.style.clipPath = 'inset(-' + overflowPad + 'px -' + padRight + 'px -' + overflowPad + 'px -' + padLeft + 'px)';"),
            "the clip-path must be widened (not just the baseline overflowPad) when leaf-span \
             content genuinely exceeds the box's narrowest tween endpoint"
        );

        // The earlier (rejected) minWidth-reservation approach must NOT be present —
        // it fights the box's own WAAPI width tween and grows the visible overflow
        // over the course of the transition instead of fixing it.
        assert!(
            !stdlib_src.contains("wci.enterLayer.parentElement.style.minWidth"),
            "must not reserve word-container width via minWidth — this was tried and reverted \
             because it fights the outer box's legitimate WAAPI width shrink, growing (not \
             fixing) the visible clipped overflow over the transition's duration"
        );
    }

    #[test]
    fn test_on_visible_matches_dispatcher_form() {
        use crate::parser::parse as parse_to_ast;

        // `visible` is a PROGRESS driver (not an EVENT driver): a mutation body
        // on it is E0949 by design, so the fixture carries an ANIMATION body
        // (`opacity: 0 -> 1`) — that is what this test's subject exercises.
        let input = r#"
.hero {
    @on &.visible {
        opacity: 0 -> 1;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let on_match = ast.matches.iter().find(|m| m.macro_name == "on");
        // PLAN-124 W3.4: the bare `@on visible` dispatcher was RETIRED; the
        // sigil head `@on-driver-body` answers `@on &.visible { … }`. The
        // driver member is the event (`visible`).
        assert_eq!(
            on_match.as_ref().and_then(|m| m.matched_macro.as_deref()),
            Some("on-driver-body"),
            "@on &.visible should match the driver body head. Got: {:?}",
            ast.matches
                .iter()
                .map(|m| &m.macro_name)
                .collect::<Vec<_>>()
        );

        let m = on_match.unwrap();
        // The event is the driver member: `&.visible` → member = "visible".
        let driver = match m.captures.get("driver") {
            Some(CapturedValue::Named(d)) => d,
            _ => panic!("Should have a Named 'driver' capture"),
        };
        match driver.get("member") {
            Some(CapturedValue::Ident(member)) => assert_eq!(member, "visible"),
            _ => panic!("driver.member should be Ident(\"visible\")"),
        }
    }

    #[test]
    fn test_on_click_mutation_matches_form() {
        use crate::parser::parse as parse_to_ast;

        let input = r#"
.x {
    @on &.click {
        $y <- 1;
    }
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        let on_match = ast.matches.iter().find(|m| m.macro_name == "on");
        assert!(
            on_match.is_some(),
            "Should have an 'on' FormMatch for @on click mutation. Got: {:?}",
            ast.matches
                .iter()
                .map(|m| &m.macro_name)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_unknown_directive_still_flagged() {
        use crate::parser::parse as parse_to_ast;

        let input = r#"
.x {
    @nonexistent
}
"#;
        let ast = parse_to_ast(input).expect("Failed to parse");

        // @nonexistent should NOT produce a FormMatch
        let bad_match = ast.matches.iter().find(|m| m.macro_name == "nonexistent");
        assert!(
            bad_match.is_none(),
            "@nonexistent should not produce a FormMatch"
        );
    }

}
