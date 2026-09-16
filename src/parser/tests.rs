//! Comprehensive test suite for the Spacetime DSL parser

use super::*;
use crate::syntax::CapturedValue;

/// A `~` preset declaration/reference is RETIRED (SIP-001c / BUG-263): parsing
/// it must fail LOUDLY with the retirement diagnostic, never silently parse.
/// Every old `@preset ... ~name` test asserts this instead of the retired
/// directive's success path.
fn assert_preset_retired(input: &str) {
    let errs = parse(input).expect_err("a `~` preset must now be a parse error");
    assert!(
        errs
            .iter()
            .any(|e| e.message.contains("preset references are retired")),
        "expected the retirement error, got: {:?}",
        errs
    );
}


// =============================================================================
// @on Timeline Tests (Phase 7)
// =============================================================================

#[test]
fn test_parse_import() {
    let input = r#"@import "./shared.st";"#;
    let result = parse(input).unwrap();
    assert_eq!(result.imports.len(), 1);
    assert_eq!(result.imports[0].path, "./shared.st");
    // Verify span is captured
    assert!(result.imports[0].span.start < result.imports[0].span.end);
}
#[test]
fn test_parse_multiple_imports() {
    let input = r#"
@import "./easings.st";
@import "./presets.st";
@import "../shared/animations.st";
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.imports.len(), 3);
    assert_eq!(result.imports[0].path, "./easings.st");
    assert_eq!(result.imports[1].path, "./presets.st");
    assert_eq!(result.imports[2].path, "../shared/animations.st");
}
#[test]
fn test_parse_easing_preset_spring() {
    // `@preset ... ~name` is retired: the `~` declaration must now error loudly.
    assert_preset_retired("@preset easing ~my-bounce: spring(350, 15, 1);");
}
#[test]
fn test_parse_easing_preset_cubic_bezier() {
    assert_preset_retired("@preset easing ~soft-exit: cubic-bezier(0.4, 0, 0.2, 1);");
}
#[test]
fn test_parse_scroll_preset() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"@preset scroll ~slow-reveal: start: 0.2, end: 0.8;"#);}
#[test]
fn test_parse_generic_macro_call() {
    // Previously this was expected to fail, but with the generic_macro_call grammar,
    // any @identifier pattern is now accepted at parse time.
    // Validation of whether the macro exists happens in the metasystem.
    let input = r#"
.hero {
    @invalid timeline {
        .title {
            opacity: 0 -> 1;
        }
    }
}
"#;
    let result = parse(input);
    // Now parses as a generic_macro_call with name="invalid" and inline_args="timeline"
    assert!(result.is_ok());
}
#[test]
fn test_parse_animation_preset_definition() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"@preset animation ~my-fade: from: 0, to: 1;"#);}
#[test]
fn test_parse_multiple_animation_presets() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~fade-in: from: 0, to: 1;
@preset animation ~slide-up: start: 20, end: 0;
@preset easing ~smooth: cubic-bezier(0.4, 0, 0.2, 1);
"#);}
#[test]
fn test_animation_preset_with_mixed_types() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~complex-entry: opacity: 0, scale: 0.9, offset: 20;
"#);}
#[test]
fn test_parse_animation_preset_block_style() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~fade-in {
    opacity: 0 -> 1;
    easing: ~ease-out-expo;
}
"#);}
#[test]
fn test_parse_animation_preset_complex() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~slide-up {
    opacity: 0 -> 1;
    translate-y: 40px -> 0;
    easing: ~ease-out-expo;
}
"#);}
#[test]
fn test_parse_preset_with_timing() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~delayed-fade {
    opacity: 0 -> 1;
    range: 0.2 + 0.5;
}
"#);}
#[test]
fn test_parse_preset_with_stagger() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~staggered-fade {
    opacity: 0 -> 1;
    stagger: 0.1 first;
}
"#);}

#[test]
fn test_stagger_grid_center() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~grid-fade {
    opacity: 0 -> 1;
    stagger: 0.003 grid(13 13) center;
}
"#);}

#[test]
fn test_stagger_grid_first() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~grid-appear {
    opacity: 0 -> 1;
    stagger: 0.005 grid(8 4) first;
}
"#);}

#[test]
fn test_stagger_simple_first() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~simple-stagger {
    opacity: 0 -> 1;
    stagger: 0.08 first;
}
"#);}

#[test]
fn test_stagger_simple_center() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~center-stagger {
    opacity: 0 -> 1;
    stagger: 0.1 center;
}
"#);}

#[test]
fn test_stagger_grid_nonsquare() {
    // `@preset ... ~name` is retired (SIP-001c / BUG-263): the `~`
    // declaration must now error loudly, not silently parse.
    assert_preset_retired(r#"
@preset animation ~rect-grid {
    opacity: 0 -> 1;
    stagger: 0.01 grid(6 3) center;
}
"#);}

#[test]
fn test_parse_type_definition() {
    let input = r#"
@type Product {
    id: string;
    name: string;
    price: number;
    inStock: boolean;
}
"#;
    let result = parse(input).unwrap();
    let type_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "type")
        .expect("should have @type FormMatch");

    // Type name should be captured
    assert_eq!(type_fm.get_ident("name"), Some("Product"));
}
#[test]
fn test_parse_type_with_optional_fields() {
    let input = r#"
@type User {
    id: string;
    name: string;
    email?: string;
    avatar?: url;
}
"#;
    let result = parse(input).unwrap();
    let type_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "type")
        .expect("should have @type FormMatch");
    assert_eq!(type_fm.get_ident("name"), Some("User"));
}
#[test]
fn test_parse_type_with_array() {
    let input = r#"
@type Article {
    tags: string[];
    authors: string[];
}
"#;
    let result = parse(input).unwrap();
    let type_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "type")
        .expect("should have @type FormMatch");
    assert_eq!(type_fm.get_ident("name"), Some("Article"));
}
#[test]
fn test_parse_type_with_union() {
    let input = r#"
@type Status {
    state: "draft" | "published" | "archived";
}
"#;
    let result = parse(input).unwrap();
    let type_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "type")
        .expect("should have @type FormMatch");
    assert_eq!(type_fm.get_ident("name"), Some("Status"));
}
#[test]
fn test_parse_data_from_url() {
    // Unified surface: remote URL via @data fetch.
    let input = r#"
@data fetch $products Product[] : "/api/products" { refresh: 5m }
"#;
    let result = parse(input).unwrap();
    let data_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "data")
        .expect("should have @data FormMatch");
    // BUG-137 body-shape resolution: a `@data fetch … { refresh: 5m }` WITH an
    // options body resolves to the body-consuming `data-fetch-opts` overload —
    // NOT the bodyless `data-fetch-kind` (which would silently ignore the body and
    // hardcode `refresh: 0`). The sibling resolver prefers the form that consumes
    // the `{ … }` block.
    assert_eq!(data_fm.matched_macro.as_deref(), Some("data-fetch-opts"));
    let name = data_fm
        .get_binding("name")
        .map(|s| s.strip_prefix('$').unwrap_or(s));
    assert_eq!(name, Some("products"));
    // The URL source is captured as `src`.
    assert!(
        data_fm.captures.contains_key("src"),
        "fetch should capture 'src', got: {:?}",
        data_fm.captures.keys().collect::<Vec<_>>()
    );
}
#[test]
fn test_parse_data_from_localstorage() {
    // Unified surface: localStorage source via @data fetch.
    let input = r#"
@data fetch $settings Settings : localStorage("app-settings") { initial: [] }
"#;
    let result = parse(input).unwrap();
    let data_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "data")
        .expect("should have @data FormMatch");
    // BUG-137 body-shape resolution: the `{ initial: [] }` options body selects the
    // body-consuming `data-fetch-opts` overload (was wrongly `data-fetch-kind`,
    // which ignored the body).
    assert_eq!(data_fm.matched_macro.as_deref(), Some("data-fetch-opts"));
    let name = data_fm
        .get_binding("name")
        .map(|s| s.strip_prefix('$').unwrap_or(s));
    assert_eq!(name, Some("settings"));
}
#[test]
fn test_parse_data_inline() {
    // Unified surface (PLAN-023 W5/FEAT-047): inline carries a value literal.
    let input = r#"
@data inline $theme Theme : 3 ;
"#;
    let result = parse(input).unwrap();
    let data_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "data")
        .expect("should have @data FormMatch");
    assert_eq!(data_fm.matched_macro.as_deref(), Some("data-inline"));
    let name = data_fm
        .get_binding("name")
        .map(|s| s.strip_prefix('$').unwrap_or(s));
    assert_eq!(name, Some("theme"));
}
#[test]
fn test_parse_computed_filter() {
    // Unified surface: @computed { from; where } -> @data query from $src { where }.
    let input = r#"
@data query $activeProducts Product[] from $products {
    where: item.inStock == true
}
"#;
    let result = parse(input).unwrap();
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.matched_macro.as_deref() == Some("data-query")),
        "should have @data query FormMatch"
    );
}
#[test]
fn test_parse_computed_with_sort() {
    // Unified surface: @computed { from; sort; limit } -> @data query.
    let input = r#"
@data query $sortedProducts Product[] from $products {
    sort: item.price; dir: desc; limit: 10
}
"#;
    let result = parse(input).unwrap();
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.matched_macro.as_deref() == Some("data-query")),
        "should have @data query FormMatch"
    );
}
#[test]
fn test_parse_fn_definition() {
    let input = r#"
@fn formatPrice($price number): string {
    return "$" + price.toFixed(2);
}
"#;
    let result = parse(input).unwrap();
    let fn_matches: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.macro_name == "fn")
        .collect();
    assert_eq!(fn_matches.len(), 1);

    let fn_def = fn_matches[0];
    // Function name is stored in "name" capture (heuristic naming for first positional arg)
    assert_eq!(fn_def.get_ident("name"), Some("formatPrice"));
    // PLAN-025 / BUG-045: the return type after `)` (`: string`) is now captured as a
    // post-arglist inline element (it used to be silently dropped).
    let rt = fn_def.get("returnType").and_then(|v| match v {
        crate::syntax::CapturedValue::TypeRef(t) | crate::syntax::CapturedValue::Ident(t) => {
            Some(t.as_str())
        }
        _ => None,
    });
    assert_eq!(
        rt,
        Some("string"),
        "@fn returnType must be captured (PLAN-025)"
    );
}

#[test]
fn test_parse_post_arglist_returntype_capture() {
    // PLAN-025: a capture between `)` and `{` (the `: $returnType:typeref` run) is
    // captured for both @fn and @compute. Regression for the silently-dropped returnType.
    let fn_rt = parse("@fn f($x number): string { \"y\" }\n")
        .unwrap()
        .matches
        .iter()
        .find(|m| m.macro_name == "fn")
        .and_then(|m| m.get("returnType").cloned());
    assert!(fn_rt.is_some(), "@fn returnType captured");

    let cm_rt = parse("@compute c($a, $b): number { return 0; }\n")
        .unwrap()
        .matches
        .iter()
        .find(|m| m.macro_name == "compute")
        .and_then(|m| m.get("returnType").cloned());
    assert!(
        cm_rt.is_some(),
        "@compute returnType captured (was E0804 before PLAN-025)"
    );

    // Regression: a form with NOTHING between `)` and `{` must be unaffected.
    let on = parse(".b { @on &.click { $x <- 1; } }\n").unwrap();
    assert!(
        on.scopes
            .iter()
            .flat_map(|s| &s.matches)
            .any(|m| m.macro_name == "on"),
        "@on &.click {{}} (no post-arglist run) still matches"
    );
}
#[test]
fn test_parse_fn_no_return_type() {
    let input = r#"
@fn logMessage($msg string) {
    console.log(msg);
}
"#;
    let result = parse(input).unwrap();
    let fn_matches: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.macro_name == "fn")
        .collect();
    assert_eq!(fn_matches.len(), 1);
    // Verify function was parsed - function name is in "name" capture
    assert_eq!(fn_matches[0].get_ident("name"), Some("logMessage"));
}
#[test]
fn test_parse_each_block_simple() {
    let input = r#"
.product-list {
    @each($products as $p) {
        &product-card($p);
    }
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let each_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "each")
        .expect("scope should have @each FormMatch");
    assert!(
        each_fm.captures.contains_key("source"),
        "each should capture 'source', got: {:?}",
        each_fm.captures.keys().collect::<Vec<_>>()
    );
    assert!(
        each_fm.captures.contains_key("item"),
        "each should capture 'item', got: {:?}",
        each_fm.captures.keys().collect::<Vec<_>>()
    );
}
#[test]
fn test_parse_each_with_template_invocation() {
    // @each with template invocation uses stdlib syntax
    let input = r#"
.list {
    @each($items as $item) {
        &card($item);
    }
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let each_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "each")
        .expect("scope should have @each FormMatch");
    assert!(each_fm.captures.contains_key("source"));
    assert!(each_fm.captures.contains_key("item"));
}
#[test]
fn test_parse_each_with_key() {
    // @each with key parameter
    let input = r#"
.cards {
    @each($cards as $card, key: $card.id) {
        &card-item($card);
    }
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let each_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "each")
        .expect("scope should have @each FormMatch");
    assert!(each_fm.captures.contains_key("source"));
    assert!(each_fm.captures.contains_key("item"));
    assert!(
        each_fm.captures.contains_key("key"),
        "each should capture 'key', got: {:?}",
        each_fm.captures.keys().collect::<Vec<_>>()
    );
}
#[test]
fn test_parse_each_with_transitions() {
    // @each with entering/exiting transition blocks
    let input = r#"
.items {
    @each($items as $item) {
        &list-item($item);

        :entering {
            opacity: 0 -> 1
        }
        :exiting {
            opacity: 1 -> 0
        }
    }
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let each_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "each")
        .expect("scope should have @each FormMatch");
    assert!(each_fm.captures.contains_key("source"));
    assert!(each_fm.captures.contains_key("item"));
}
#[test]
fn test_parse_bind_text() {
    // @bind uses stdlib named-arg syntax
    let input = r#"
.test {
    @bind(text: $user.name)
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let bind_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "bind")
        .expect("scope should have @bind FormMatch");
    // PLAN-079: the tombstone %macro bind is gone; the match is claimed by
    // the `reactive-surface` migration's bind-text rewrite rule (a rule def
    // outranks the embedded retired macro's catch-all), which captures `$x`.
    assert_eq!(
        bind_fm.matched_macro.as_deref(),
        Some("reactive-surface#bind-text")
    );
    assert!(
        bind_fm.captures.contains_key("x"),
        "bind-text rule should capture 'x', got: {:?}",
        bind_fm.captures.keys().collect::<Vec<_>>()
    );
}
#[test]
fn test_parse_bind_class_with_when() {
    // @bind with class + when condition
    let input = r#"
.test {
    @bind(class: "active", when: $isActive)
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let bind_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "bind")
        .expect("scope should have @bind FormMatch");
    // PLAN-079: claimed by the `reactive-surface` migration's bind-class
    // rewrite rule, whose %match captures `$c` (class) and `$w` (when).
    assert_eq!(
        bind_fm.matched_macro.as_deref(),
        Some("reactive-surface#bind-class")
    );
    assert!(
        bind_fm.captures.contains_key("c"),
        "bind-class migration should capture 'c', got: {:?}",
        bind_fm.captures.keys().collect::<Vec<_>>()
    );
    assert!(
        bind_fm.captures.contains_key("w"),
        "bind-class migration should capture 'w', got: {:?}",
        bind_fm.captures.keys().collect::<Vec<_>>()
    );
}
#[test]
fn test_parse_complete_data_binding_example() {
    // Complete example with @type, @data, @computed, @fn, @each
    let input = r#"
@type Product {
    id: string;
    name: string;
    price: number;
    inStock: boolean;
}

@data fetch $products Product[] : "/api/products" { refresh: 5m }

@data query $activeProducts Product[] from $products {
    where: item.inStock == true; sort: item.price; dir: asc
}

@fn formatPrice($price number): string {
    return "$" + price.toFixed(2);
}

.product-grid {
    @each($activeProducts as $p) {
        &product-card($p);
    }
}
"#;
    let result = parse(input).unwrap();

    // Verify @type, @data, @computed are in file-level matches
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.macro_name == "type"),
        "should have @type FormMatch"
    );
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.macro_name == "data"),
        "should have @data FormMatch"
    );
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.matched_macro.as_deref() == Some("data-query")),
        "should have @data query FormMatch"
    );

    // @fn is in file-level matches
    assert!(
        result
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.macro_name == "fn"),
        "should have @fn FormMatch"
    );

    // Verify scope and @each
    assert_eq!(result.scopes.len(), 1);
    assert!(
        result.scopes[0]
            .matches
            .iter()
            .any(|m| m.macro_name == "each"),
        "scope should have @each FormMatch"
    );
}
#[test]
fn test_parse_for_range() {
    // @for range loop uses stdlib syntax
    let input = r#"
.stars {
    @for($i in 1 .. 5) {
        > .star { }
    }
}
"#;
    let result = parse(input).unwrap();
    let scope = &result.scopes[0];
    let for_fm = scope
        .matches
        .iter()
        .find(|m| m.macro_name == "for" || m.macro_name == "for-range")
        .expect("scope should have @for FormMatch");
    assert!(
        for_fm.captures.contains_key("_var"),
        "for should capture '_var' (the deliberately-dropped loop var), got: {:?}",
        for_fm.captures.keys().collect::<Vec<_>>()
    );
}
#[test]
fn test_parse_type_missing_semicolon() {
    // Missing semicolons in body — grammar is lenient, may or may not match %form
    let input = r#"
@type Product {
    name: string
    price: number;
}
"#;
    let result = parse(input);
    // Either it parses and gets a FormMatch, or parsing fails — both acceptable
    if let Ok(file) = result {
        // If it matched, great; if not, metasystem validates later
        let _has_type = file
            .matches
            .iter()
            .any(|m| m.selector.is_none() && m.macro_name == "type");
    }
}
#[test]
fn test_parse_data_no_semicolons() {
    // Unified @data fetch: the trailing options block is optional and its inner
    // props are semicolon-free (Spacetime allows newline-separated props).
    let input = r#"
@data fetch $suites TestSuite[] : "/api/tests" {
  refresh: 0
}
"#;
    let result = parse(input).unwrap();
    let data_fm = result
        .matches
        .iter()
        .find(|m| m.selector.is_none() && m.macro_name == "data")
        .expect("should have @data FormMatch");
    // BUG-137 body-shape resolution: the (semicolon-free) options body selects the
    // body-consuming `data-fetch-opts` overload (was wrongly `data-fetch-kind`).
    assert_eq!(data_fm.matched_macro.as_deref(), Some("data-fetch-opts"));
    let name = data_fm
        .get_binding("name")
        .map(|s| s.strip_prefix('$').unwrap_or(s));
    assert_eq!(name, Some("suites"));
    assert!(
        data_fm.captures.contains_key("src"),
        "fetch should capture 'src', got: {:?}",
        data_fm.captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_parse_each_invalid_syntax_still_parses() {
    // Invalid content inside macro blocks parses but won't match any %form.
    // The parser itself should succeed - validation happens post-parse.
    let input = r#"
.test {
    @each(items) {
        invalid syntax here;
    }
}
"#;
    let result = parse(input);
    // Parser succeeds even if no %form matches
    assert!(result.is_ok());
}
#[test]
fn test_parse_minimal_pattern() {
    let input = r#"
@pattern foo() {
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.patterns.len(), 1);
    assert_eq!(result.patterns[0].name, "foo");
    assert!(result.patterns[0].params.is_empty());
    assert!(result.patterns[0].body.is_empty());
}
#[test]
fn test_parse_pattern_with_params() {
    let input = r#"
@pattern edit($persist, $trigger: dblclick) {
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.patterns.len(), 1);
    let pattern = &result.patterns[0];
    assert_eq!(pattern.name, "edit");
    assert_eq!(pattern.params.len(), 2);

    // First param - no default
    assert_eq!(pattern.params[0].name, "persist");
    assert!(pattern.params[0].default.is_none());

    // Second param - with default identifier
    assert_eq!(pattern.params[1].name, "trigger");
    assert!(pattern.params[1].default.is_some());
    if let Some(CapturedValue::Ident(ref id)) = pattern.params[1].default {
        assert_eq!(id, "dblclick");
    } else {
        panic!("Expected identifier default");
    }
}
#[test]
fn test_parse_pattern_with_list_param() {
    let input = r#"
@pattern wizard($steps, $highlights: []) {
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];
    assert_eq!(pattern.params.len(), 2);

    // Second param - empty list default
    if let Some(CapturedValue::Array(ref items)) = pattern.params[1].default {
        assert!(items.is_empty());
    } else {
        panic!("Expected empty list default");
    }
}
#[test]
fn test_parse_pattern_with_list_values() {
    let input = r#"
@pattern tabs($items: [home, about, contact]) {
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];

    if let Some(CapturedValue::Array(ref items)) = pattern.params[0].default {
        assert_eq!(items.len(), 3);
        if let CapturedValue::Ident(ref id) = items[0] {
            assert_eq!(id, "home");
        }
    } else {
        panic!("Expected list default");
    }
}
#[test]
fn test_parse_pattern_if_not_equals() {
    let input = r#"
@pattern guarded($persist) {
    @if $persist != none {
        @async_transition(trigger: "save", from: "editing", on_success: "saved", on_error: "error")
    }
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];

    if let PatternBodyItem::If(ref if_ast) = pattern.body[0] {
        if let PatternCondition::NotEquals(ref var, ref val) = if_ast.condition {
            assert_eq!(var, "persist");
            if let CapturedValue::Ident(id) = val {
                assert_eq!(id, "none");
            }
        } else {
            panic!("Expected not equals condition");
        }
    } else {
        panic!("Expected If body item");
    }
}
#[test]
fn test_parse_pattern_include() {
    let input = r#"
@pattern composed() {
    @include focusable()
    @include loading(delay: 300)
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];
    assert_eq!(pattern.body.len(), 2);

    if let PatternBodyItem::Include(ref inc) = pattern.body[0] {
        assert_eq!(inc.pattern_name, "focusable");
        assert!(inc.args.is_empty());
    } else {
        panic!("Expected Include body item");
    }

    if let PatternBodyItem::Include(ref inc) = pattern.body[1] {
        assert_eq!(inc.pattern_name, "loading");
        assert_eq!(inc.args.len(), 1);
        assert_eq!(inc.args[0].name, "delay");
    } else {
        panic!("Expected Include body item");
    }
}
#[test]
fn test_parse_pattern_call() {
    let input = r#"
@pattern wrapper() {
    @edit(persist: "data.html", trigger: dblclick)
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];
    assert_eq!(pattern.body.len(), 1);

    if let PatternBodyItem::Call(ref call) = pattern.body[0] {
        assert_eq!(call.pattern_name, "edit");
        assert_eq!(call.args.len(), 2);
        assert_eq!(call.args[0].name, "persist");
        assert_eq!(call.args[1].name, "trigger");
    } else {
        panic!("Expected Call body item");
    }
}
#[test]
fn test_parse_pattern_call_with_variable_arg() {
    let input = r#"
@pattern delegating($target) {
    @save(file: $target)
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];

    if let PatternBodyItem::Call(ref call) = pattern.body[0] {
        assert_eq!(call.args[0].name, "file");
        if let CapturedValue::Binding(ref var) = call.args[0].value {
            assert_eq!(var, "target");
        } else {
            panic!("Expected variable value");
        }
    } else {
        panic!("Expected Call body item");
    }
}
#[test]
fn test_parse_pattern_call_with_list_arg() {
    let input = r#"
@pattern multi() {
    @highlights(classes: [bold, italic, underline])
}
"#;
    let result = parse(input).unwrap();
    let pattern = &result.patterns[0];

    if let PatternBodyItem::Call(ref call) = pattern.body[0] {
        if let CapturedValue::Array(ref items) = call.args[0].value {
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected list value");
        }
    } else {
        panic!("Expected Call body item");
    }
}
#[test]
fn test_parse_primitive_if_block() {
    let input = r#"
%primitive animate(&el, easing: string = "linear") {
    %if $easing == "linear" {
        %emit css {
            transition: all 0.3s linear;
        }
    } %else {
        %emit js {
            el.style.transition = "all 0.3s " + easing;
        }
    }
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.meta_defs.len(), 1);

    if let crate::parser::meta_ast::MetaDef::Primitive(prim) = &result.meta_defs[0] {
        assert_eq!(prim.name, "animate");
        assert_eq!(prim.params.len(), 2);
        assert_eq!(prim.body.if_blocks.len(), 1);

        let if_block = &prim.body.if_blocks[0];

        // Check condition
        if let crate::parser::meta_ast::MetaIfCondition::Equals(var, val) = &if_block.condition {
            assert_eq!(var, "easing");
            assert_eq!(val, "linear");
        } else {
            panic!("Expected Equals condition");
        }

        // Check then body has CSS emit
        assert_eq!(if_block.then_body.emit_blocks.len(), 1);
        assert_eq!(
            if_block.then_body.emit_blocks[0].lang,
            crate::parser::meta_ast::EmitLang::Css
        );

        // Check else body has JS emit
        assert!(if_block.else_body.is_some());
        let else_body = if_block.else_body.as_ref().unwrap();
        assert_eq!(else_body.emit_blocks.len(), 1);
        assert_eq!(
            else_body.emit_blocks[0].lang,
            crate::parser::meta_ast::EmitLang::Js
        );
    } else {
        panic!("Expected primitive definition");
    }
}
#[test]
fn test_parse_primitive_if_block_without_else() {
    let input = r#"
%primitive conditional(&el, enabled: bool = true) {
    %if $enabled {
        %emit js {
            el.classList.add("enabled");
        }
    }
}
"#;
    let result = parse(input).unwrap();

    if let crate::parser::meta_ast::MetaDef::Primitive(prim) = &result.meta_defs[0] {
        assert_eq!(prim.body.if_blocks.len(), 1);

        let if_block = &prim.body.if_blocks[0];

        // Check condition is truthy
        if let crate::parser::meta_ast::MetaIfCondition::Truthy(var) = &if_block.condition {
            assert_eq!(var, "enabled");
        } else {
            panic!("Expected Truthy condition");
        }

        // Check then body
        assert_eq!(if_block.then_body.emit_blocks.len(), 1);

        // Check no else body
        assert!(if_block.else_body.is_none());
    } else {
        panic!("Expected primitive definition");
    }
}
#[test]
fn test_parse_bind_function_call() {
    let input = r#"
%macro test-macro {
    %creates @test

    %binds {
        some-primitive(
            &self,
            color: colorToArray($clearColor),
            size: 100
        ) -> { $result }
    }
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.meta_defs.len(), 1);

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] {
        assert_eq!(mac.name, "test-macro");
        assert_eq!(mac.binds.len(), 1);

        let bind = &mac.binds[0];
        assert_eq!(bind.primitive, "some-primitive");
        assert_eq!(bind.args.len(), 3); // &self + color + size

        // Check element arg
        assert!(
            matches!(&bind.args[0], crate::parser::meta_ast::BindArg::Element { name, .. } if name == "self")
        );

        // Check the function call argument
        if let crate::parser::meta_ast::BindArg::Named { name, value } = &bind.args[1] {
            assert_eq!(name, "color");
            if let crate::parser::meta_ast::BindValue::FunctionCall {
                name: fn_name,
                args,
            } = value
            {
                assert_eq!(fn_name, "colorToArray");
                assert_eq!(args.len(), 1);
                if let crate::parser::meta_ast::BindValue::Variable(var) = &args[0] {
                    assert_eq!(var, "clearColor");
                } else {
                    panic!("Expected Variable in function args");
                }
            } else {
                panic!("Expected FunctionCall bind value");
            }
        } else {
            panic!("Expected Named bind arg");
        }
    } else {
        panic!("Expected macro definition");
    }
}
#[test]
fn test_parse_bind_function_call_multiple_args() {
    let input = r#"
%macro multi-arg {
    %creates @multi

    %binds {
        blend(
            &self,
            color: lerp($color1, $color2, 0.5)
        ) -> { $blended }
    }
}
"#;
    let result = parse(input).unwrap();

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] {
        let bind = &mac.binds[0];
        assert_eq!(bind.args.len(), 2); // &self + color

        // Check element arg
        assert!(
            matches!(&bind.args[0], crate::parser::meta_ast::BindArg::Element { name, .. } if name == "self")
        );

        // Check the function call argument
        if let crate::parser::meta_ast::BindArg::Named { name, value } = &bind.args[1] {
            assert_eq!(name, "color");
            if let crate::parser::meta_ast::BindValue::FunctionCall {
                name: fn_name,
                args,
            } = value
            {
                assert_eq!(fn_name, "lerp");
                assert_eq!(args.len(), 3);
                // $color1
                assert!(
                    matches!(&args[0], crate::parser::meta_ast::BindValue::Variable(v) if v == "color1")
                );
                // $color2
                assert!(
                    matches!(&args[1], crate::parser::meta_ast::BindValue::Variable(v) if v == "color2")
                );
                // 0.5
                assert!(
                    matches!(&args[2], crate::parser::meta_ast::BindValue::Number(n) if (*n - 0.5).abs() < 0.001)
                );
            } else {
                panic!("Expected FunctionCall");
            }
        } else {
            panic!("Expected Named arg");
        }
    } else {
        panic!("Expected macro");
    }
}
#[test]
fn test_parse_bind_output_alias() {
    let input = r#"
%macro data-fetch {
    %creates @data

    %binds {
        data-source(
            name: $name,
            src: $src
        ) -> {
            $data as $$name,
            $loading as ${$name}-loading,
            $error as ${$name}-error
        }
    }
}
"#;
    let result = parse(input).unwrap();

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] {
        let bind = &mac.binds[0];
        assert_eq!(bind.outputs.len(), 3);

        // $data as $$name
        assert_eq!(bind.outputs[0].name, "data");
        assert_eq!(bind.outputs[0].alias.as_deref(), Some("$$name"));

        // $loading as ${$name}-loading
        assert_eq!(bind.outputs[1].name, "loading");
        assert_eq!(bind.outputs[1].alias.as_deref(), Some("${$name}-loading"));

        // $error as ${$name}-error
        assert_eq!(bind.outputs[2].name, "error");
        assert_eq!(bind.outputs[2].alias.as_deref(), Some("${$name}-error"));
    } else {
        panic!("Expected macro");
    }
}
/// FUP-138 root cause: a `%binds` output list written ONE PER LINE lost every
/// entry after the first. The bind-line accumulator joins physical lines with a
/// space before the output parser runs, and that parser split on `,` alone — so
/// the whole block became ONE output whose ALIAS was the rest of the text. That
/// alias fails resolve.rs's plain-ident gate, no remap is recorded, and `%yield`
/// then writes the RAW export name: the aliased signal is never populated and
/// the consuming page silently reads nothing. The inspector pill's Saved/error
/// line was dead this way from the day it shipped.
#[test]
fn test_parse_bind_outputs_one_per_line_without_commas() {
    let input = r#"
%macro multiline {
    %creates @multiline

    %binds {
        some-prim(&self) -> {
            $error as $insWriteError
            $status as $insWriteStatus
        }
    }
}
"#;
    let result = parse(input).unwrap();

    let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] else {
        panic!("expected a macro");
    };
    let outs = &mac.binds[0].outputs;
    assert_eq!(
        outs.len(),
        2,
        "a newline-separated output list must yield BOTH entries, not one run-on"
    );
    assert_eq!(outs[0].name, "error");
    assert_eq!(outs[0].alias.as_deref(), Some("$insWriteError"));
    assert_eq!(outs[1].name, "status");
    assert_eq!(
        outs[1].alias.as_deref(),
        Some("$insWriteStatus"),
        "the trailing alias must be a plain ident — swallowing it is what killed the remap"
    );
}

/// The separator fix must not change the comma forms, including a comma list
/// that spans lines and a mix of aliased and bare outputs.
#[test]
fn test_parse_bind_outputs_separator_forms_agree() {
    let cases = [
        "{ $a as $x, $b as $y }",
        "{\n  $a as $x,\n  $b as $y\n}",
        "{\n  $a as $x\n  $b as $y\n}",
    ];
    for case in cases {
        let input = format!(
            "\n%macro m {{\n  %creates @m\n\n  %binds {{\n    p(&self) -> {}\n  }}\n}}\n",
            case
        );
        let result = parse(&input).unwrap();
        let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] else {
            panic!("expected a macro");
        };
        let outs = &mac.binds[0].outputs;
        assert_eq!(outs.len(), 2, "comma and newline forms must agree: {case}");
        assert_eq!(outs[0].alias.as_deref(), Some("$x"), "case {case}");
        assert_eq!(outs[1].alias.as_deref(), Some("$y"), "case {case}");
    }
}

#[test]
fn test_parse_bind_output_no_alias() {
    let input = r#"
%macro simple {
    %creates @simple

    %binds {
        some-prim(&self) -> { $data, $loading }
    }
}
"#;
    let result = parse(input).unwrap();

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] {
        let bind = &mac.binds[0];
        assert_eq!(bind.outputs.len(), 2);

        // $data (no alias)
        assert_eq!(bind.outputs[0].name, "data");
        assert!(bind.outputs[0].alias.is_none());

        // $loading (no alias)
        assert_eq!(bind.outputs[1].name, "loading");
        assert!(bind.outputs[1].alias.is_none());
    } else {
        panic!("Expected macro");
    }
}
#[test]
fn test_parse_fn_full_type_simple() {
    let input = r#"
%primitive test(&el) {
  %emit js {
    const fn = (n) => n * 2;
    %yield fn -> $double;
  }

  %exports {
    $double: fn(number) ~> number
  }
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.meta_defs.len(), 1);

    if let meta_ast::MetaDef::Primitive(prim) = &result.meta_defs[0] {
        assert_eq!(prim.body.exports.len(), 1);
        let export = &prim.body.exports[0];
        assert_eq!(export.name, "double");

        if let meta_ast::ExportTypeExpr::Function(func) = &export.type_expr {
            assert_eq!(func.params.len(), 1);
            assert_eq!(func.params[0].param_type, "number");
            assert!(!func.params[0].optional);
            assert_eq!(func.return_type, "number");
        } else {
            panic!("Expected Function type expression");
        }
    } else {
        panic!("Expected Primitive");
    }
}
#[test]
fn test_parse_fn_full_type_with_optional_params() {
    let input = r#"
%primitive test(&el) {
  %emit js {
    %yield fn -> $setProgress;
  }

  %exports {
    $setProgress: fn(number, string?) ~> void
  }
}
"#;
    let result = parse(input).unwrap();

    if let meta_ast::MetaDef::Primitive(prim) = &result.meta_defs[0] {
        let export = &prim.body.exports[0];

        if let meta_ast::ExportTypeExpr::Function(func) = &export.type_expr {
            assert_eq!(func.params.len(), 2);

            // First param: number (required)
            assert!(!func.params[0].optional);
            assert_eq!(func.params[0].param_type, "number");

            // Second param: string? (optional)
            assert!(func.params[1].optional);
            assert_eq!(func.params[1].param_type, "string");

            assert_eq!(func.return_type, "void");
        } else {
            panic!("Expected Function type expression");
        }
    } else {
        panic!("Expected Primitive");
    }
}
#[test]
fn test_parse_fn_legacy_type_still_works() {
    let input = r#"
%primitive test(&el) {
  %emit js {
    %yield fn -> $legacy;
  }

  %exports {
    $legacy: fn
    $typed: fn(string)
  }
}
"#;
    let result = parse(input).unwrap();

    if let meta_ast::MetaDef::Primitive(prim) = &result.meta_defs[0] {
        // First export: plain fn
        if let meta_ast::ExportTypeExpr::LegacyFn(param) = &prim.body.exports[0].type_expr {
            assert!(param.is_none());
        } else {
            panic!("Expected LegacyFn for first export");
        }

        // Second export: fn(string)
        if let meta_ast::ExportTypeExpr::LegacyFn(param) = &prim.body.exports[1].type_expr {
            assert_eq!(param.as_deref(), Some("string"));
        } else {
            panic!("Expected LegacyFn for second export");
        }
    } else {
        panic!("Expected Primitive");
    }
}
#[test]
fn test_export_type_expr_format() {
    // Test formatting of various ExportTypeExpr variants
    use meta_ast::{ExportTypeExpr, FunctionTypeExpr, FunctionTypeParam};

    // Simple type
    let simple = ExportTypeExpr::Simple("number".to_string());
    assert_eq!(simple.format(), "number");

    // Array type
    let array = ExportTypeExpr::Array("Node".to_string());
    assert_eq!(array.format(), "Node[]");

    // Function type with params
    let func = ExportTypeExpr::Function(FunctionTypeExpr {
        params: vec![
            FunctionTypeParam {
                name: Some("progress".to_string()),
                param_type: "number".to_string(),
                optional: false,
            },
            FunctionTypeParam {
                name: Some("easing".to_string()),
                param_type: "string".to_string(),
                optional: true,
            },
        ],
        return_type: "void".to_string(),
    });
    assert_eq!(
        func.format(),
        "fn(progress: number, easing: string?) ~> void"
    );

    // Legacy fn
    let legacy = ExportTypeExpr::LegacyFn(None);
    assert_eq!(legacy.format(), "fn");

    let legacy_typed = ExportTypeExpr::LegacyFn(Some("Event".to_string()));
    assert_eq!(legacy_typed.format(), "fn(Event)");
}
#[test]
fn test_parse_macro_with_scope_clause() {
    let source = r#"
%macro test {
  %scope file | selector | property-value
  %creates @test
}
"#;

    let result = parse(source);
    assert!(result.is_ok(), "Failed to parse macro with scope clause");

    let ast = result.unwrap();
    assert_eq!(ast.meta_defs.len(), 1, "Should have one meta definition");

    if let meta_ast::MetaDef::Macro(macro_def) = &ast.meta_defs[0] {
        assert_eq!(macro_def.name, "test");
        // Only the enforced scopes (file, selector) are retained; `property-value` is a
        // non-enforced token, ignored at parse (PLAN-039: MacroScope holds only the arms
        // macro_scope_matches actually checks — richer containment is `%scope within(...)`).
        assert_eq!(macro_def.scopes.len(), 2);
        assert!(macro_def.scopes.contains(&meta_ast::MacroScope::File));
        assert!(macro_def.scopes.contains(&meta_ast::MacroScope::Selector));
    } else {
        panic!("Expected MacroDef");
    }
}

/// GH-12 / PLAN-137 W6: `%scope element(<tag>)` parses into `scope_element` as
/// kinds-as-data, alongside `file | selector`. The CST reconstructs inline text
/// with single spaces between tokens (`element ( canvas )`), so the parser must
/// compact whitespace before matching the parenthesised kind — both spellings
/// are accepted.
#[test]
fn test_parse_macro_with_element_scope() {
    let source = r#"
%macro stage {
  %scope selector | element(canvas)
  %creates @stage
}
"#;

    let result = parse(source);
    assert!(result.is_ok(), "Failed to parse macro with element scope");

    let ast = result.unwrap();
    assert_eq!(ast.meta_defs.len(), 1, "Should have one meta definition");

    if let meta_ast::MetaDef::Macro(macro_def) = &ast.meta_defs[0] {
        assert_eq!(macro_def.name, "stage");
        assert!(macro_def.scopes.contains(&meta_ast::MacroScope::Selector));
        assert_eq!(macro_def.scope_element, vec!["canvas".to_string()]);
    } else {
        panic!("Expected MacroDef");
    }
}
#[test]
fn test_parse_value_declaration_number() {
    let input = r#"
.counter {
    $count number: 0;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let value_decls: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "local-state" || m.macro_name == "value-decl")
        .collect();
    assert_eq!(value_decls.len(), 1);

    let decl = value_decls[0];
    assert_eq!(decl.get_ident("name"), Some("count"));
    // Type and value are captured in the FormMatch
}
#[test]
fn test_parse_value_declaration_string() {
    let input = r#"
.greeting {
    $message string: "Hello";
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let value_decls: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "local-state" || m.macro_name == "value-decl")
        .collect();
    assert_eq!(value_decls.len(), 1);

    let decl = value_decls[0];
    assert_eq!(decl.get_ident("name"), Some("message"));
    // Type and value are captured in the FormMatch
}
#[test]
fn test_parse_value_declaration_array() {
    let input = r#"
.list {
    $items string[]: [];
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let value_decls: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "local-state" || m.macro_name == "value-decl")
        .collect();
    assert_eq!(value_decls.len(), 1);

    let decl = value_decls[0];
    assert_eq!(decl.get_ident("name"), Some("items"));
    // Type and value are captured in the FormMatch
}
#[test]
fn test_parse_value_declaration_uninitialized() {
    let input = r#"
.component {
    $error string?;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let value_decls: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| {
            m.macro_name == "local-state"
                || m.macro_name == "local-state-uninitialized"
                || m.macro_name == "value-decl"
        })
        .collect();
    assert_eq!(value_decls.len(), 1);

    let decl = value_decls[0];
    assert_eq!(decl.get_ident("name"), Some("error"));
    // Type is captured in the FormMatch
}
#[test]
fn test_parse_value_declaration_custom_type() {
    let input = r#"
.product {
    $item Product[]: [];
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let value_decls: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "local-state" || m.macro_name == "value-decl")
        .collect();
    assert_eq!(value_decls.len(), 1);

    let decl = value_decls[0];
    assert_eq!(decl.get_ident("name"), Some("item"));
    // Type is captured in the FormMatch
}
#[test]
fn test_parse_multiple_value_declarations() {
    let input = r#"
.counter {
    $count number: 0;
    $message string: "Click me";
    $enabled boolean: true;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let value_decls: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "local-state" || m.macro_name == "value-decl")
        .collect();
    assert_eq!(value_decls.len(), 3);

    assert_eq!(value_decls[0].get_ident("name"), Some("count"));
    assert_eq!(value_decls[1].get_ident("name"), Some("message"));
    assert_eq!(value_decls[2].get_ident("name"), Some("enabled"));
}
#[test]
fn test_parse_element_ref_simple() {
    let input = r#"
.card {
    &description [slot=description];
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    assert_eq!(elem_refs.len(), 1);

    let elem_ref = elem_refs[0];
    assert_eq!(elem_ref.get_ident("name"), Some("description"));
    // Selector is stored as String, not Selector variant
    assert_eq!(
        elem_ref.get_selector("selector"),
        Some("[slot=description]")
    );
}
#[test]
fn test_parse_element_ref_with_class() {
    let input = r#"
.card {
    &header .header;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    assert_eq!(elem_refs.len(), 1);

    let elem_ref = elem_refs[0];
    assert_eq!(elem_ref.get_ident("name"), Some("header"));
    assert_eq!(elem_ref.get_selector("selector"), Some(".header"));
}
#[test]
fn test_parse_element_ref_with_id() {
    let input = r#"
.card {
    &main #main;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    assert_eq!(elem_refs.len(), 1);

    let elem_ref = elem_refs[0];
    assert_eq!(elem_ref.get_ident("name"), Some("main"));
    assert_eq!(elem_ref.get_selector("selector"), Some("#main"));
}
#[test]
fn test_parse_element_ref_with_child_combinator() {
    let input = r#"
.card {
    &title > .title;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    assert_eq!(elem_refs.len(), 1);

    let elem_ref = elem_refs[0];
    assert_eq!(elem_ref.get_ident("name"), Some("title"));
    assert_eq!(elem_ref.get_selector("selector"), Some("> .title"));
}
#[test]
fn test_parse_element_ref_with_inline_scope() {
    let input = r#"
.card {
    &title [slot=title] {
        font-size: 5px;
    }
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    assert_eq!(elem_refs.len(), 1);

    let elem_ref = elem_refs[0];
    assert_eq!(elem_ref.get_ident("name"), Some("title"));
    assert_eq!(elem_ref.get_selector("selector"), Some("[slot=title]"));
    // Note: inline scope handling would be in a different capture
}
#[test]
fn test_parse_multiple_element_refs() {
    let input = r#"
.card {
    &description [slot=description];
    &title [slot=title] {
        font-size: 5px;
    }
    &header .header;
    &main #main;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    // All 4 element refs should be captured (including the one with inline scope)
    assert_eq!(elem_refs.len(), 4);

    assert_eq!(elem_refs[0].get_ident("name"), Some("description"));
    assert_eq!(elem_refs[1].get_ident("name"), Some("title"));
    assert_eq!(elem_refs[2].get_ident("name"), Some("header"));
    assert_eq!(elem_refs[3].get_ident("name"), Some("main"));
}
#[test]
fn test_parse_element_ref_with_nested_scope() {
    let input = r#"
.card {
    &title [slot=title] {
        font-size: 5px;

        > .subtitle {
            font-size: 3px;
        }
    }
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let elem_refs: Vec<_> = result.scopes[0]
        .matches
        .iter()
        .filter(|m| m.macro_name == "element-ref")
        .collect();
    assert_eq!(elem_refs.len(), 1);

    let elem_ref = elem_refs[0];
    assert_eq!(elem_ref.get_ident("name"), Some("title"));
    // Note: nested scope handling would be in a different capture
}

#[test]
fn test_parse_data_source_primitive() {
    let content = std::fs::read_to_string("stdlib/primitives/data/source.st").unwrap();
    let result = parse(&content);
    match result {
        Ok(parsed) => {
            println!("meta_defs: {}", parsed.meta_defs.len());
            for def in &parsed.meta_defs {
                match def {
                    crate::parser::meta_ast::MetaDef::Primitive(p) => {
                        println!("primitive: {}", p.name)
                    }
                    crate::parser::meta_ast::MetaDef::Macro(m) => println!("macro: {}", m.name),
                    crate::parser::meta_ast::MetaDef::Preset(p) => {
                        println!("preset: {}:{}", p.category, p.name)
                    }
                    crate::parser::meta_ast::MetaDef::RuntimeRegistry(r) => {
                        println!("runtime-registry: {}", r.name)
                    }
                    crate::parser::meta_ast::MetaDef::CaptureType(ct) => {
                        println!("capture-type: {}", ct.name)
                    }
                    crate::parser::meta_ast::MetaDef::Vendor(v) => println!("vendor: {}", v.name),
                    crate::parser::meta_ast::MetaDef::Migration(m) => {
                        println!("migration: {} ({})", m.id, m.date)
                    }
                    crate::parser::meta_ast::MetaDef::CommentType(c) => {
                        println!("comment-type: {}", c.id)
                    }
                    crate::parser::meta_ast::MetaDef::ScalarType(s) => {
                        println!("scalar-type: {}", s.id)
                    }
                }
            }
            assert!(
                !parsed.meta_defs.is_empty(),
                "Expected meta_defs to not be empty"
            );
        }
        Err(e) => {
            panic!("Parse error: {}", e);
        }
    }
}

// =============================================================================
// Runtime Registry Parsing Tests
// =============================================================================

#[test]
fn test_parse_runtime_registry_def() {
    let input = r#"
%runtime-registry functions {
  %target js {
    %namespace { ST.functions }
    %init { ST.functions = ST.functions || {}; }
    %call($name, $args) { ST.functions[$name]($args) }
    %register($name, $fn) { ST.functions[$name] = $fn }
    %access($name) { ST.functions[$name] }
  }
}"#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "Failed to parse runtime registry: {:?}",
        result.err()
    );
    let ast = result.unwrap();

    assert_eq!(ast.meta_defs.len(), 1);
    if let crate::parser::meta_ast::MetaDef::RuntimeRegistry(registry) = &ast.meta_defs[0] {
        assert_eq!(registry.name, "functions");
        assert_eq!(registry.targets.len(), 1);

        let target = &registry.targets[0];
        assert_eq!(target.target_name, "js");
        assert_eq!(target.namespace, "ST.functions");
        assert!(target.init.contains("ST.functions = ST.functions || {}"));
        assert_eq!(target.operations.len(), 3);

        // Check call operation
        if let crate::parser::meta_ast::RegistryOperation::Call { params, template } =
            &target.operations[0]
        {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], "$name");
            assert_eq!(params[1], "$args");
            assert!(template.contains("ST.functions[$name]($args)"));
        } else {
            panic!("Expected Call operation");
        }

        // Check register operation
        if let crate::parser::meta_ast::RegistryOperation::Register { params, template } =
            &target.operations[1]
        {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], "$name");
            assert_eq!(params[1], "$fn");
            assert!(template.contains("ST.functions[$name] = $fn"));
        } else {
            panic!("Expected Register operation");
        }

        // Check access operation
        if let crate::parser::meta_ast::RegistryOperation::Access { params, template } =
            &target.operations[2]
        {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], "$name");
            assert!(template.contains("ST.functions[$name]"));
        } else {
            panic!("Expected Access operation");
        }
    } else {
        panic!("Expected RuntimeRegistry definition");
    }
}

#[test]
fn test_parse_runtime_registry_multiple_targets() {
    let input = r#"
%runtime-registry store {
  %target js {
    %namespace { window.Store }
    %init { window.Store = {}; }
    %access($key) { window.Store[$key] }
  }
  %target wasm {
    %namespace { WasmStore }
    %init { initWasmStore(); }
    %access($key) { wasmGet($key) }
  }
}"#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "Failed to parse multi-target registry: {:?}",
        result.err()
    );
    let ast = result.unwrap();

    if let crate::parser::meta_ast::MetaDef::RuntimeRegistry(registry) = &ast.meta_defs[0] {
        assert_eq!(registry.name, "store");
        assert_eq!(registry.targets.len(), 2);
        assert_eq!(registry.targets[0].target_name, "js");
        assert_eq!(registry.targets[1].target_name, "wasm");
    } else {
        panic!("Expected RuntimeRegistry definition");
    }
}

// =============================================================================
// Resolves Clause Parsing Tests
// =============================================================================

#[test]
fn test_parse_resolves_clause() {
    let input = r#"
%macro fn {
  %creates @fn
  %form { @fn $name:ident }
  %resolves {
    $name -> functions
  }
}"#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "Failed to parse resolves clause: {:?}",
        result.err()
    );
    let ast = result.unwrap();

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &ast.meta_defs[0] {
        assert!(mac.resolves.is_some());
        let resolves = mac.resolves.as_ref().unwrap();
        assert_eq!(resolves.mappings.len(), 1);
        assert_eq!(resolves.mappings[0].symbol, "$name");
        assert_eq!(resolves.mappings[0].registry, "functions");
    } else {
        panic!("Expected Macro definition");
    }
}

#[test]
fn test_parse_resolves_clause_multiple_mappings() {
    let input = r#"
%macro component {
  %creates @component
  %form { @component $name:ident }
  %resolves {
    $name -> components
    $template -> templates
    $state -> states
  }
}"#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "Failed to parse resolves with multiple mappings: {:?}",
        result.err()
    );
    let ast = result.unwrap();

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &ast.meta_defs[0] {
        assert!(mac.resolves.is_some());
        let resolves = mac.resolves.as_ref().unwrap();
        assert_eq!(resolves.mappings.len(), 3);

        assert_eq!(resolves.mappings[0].symbol, "$name");
        assert_eq!(resolves.mappings[0].registry, "components");

        assert_eq!(resolves.mappings[1].symbol, "$template");
        assert_eq!(resolves.mappings[1].registry, "templates");

        assert_eq!(resolves.mappings[2].symbol, "$state");
        assert_eq!(resolves.mappings[2].registry, "states");
    } else {
        panic!("Expected Macro definition");
    }
}

#[test]
fn test_on_mutation_form_match() {
    // Check that mutation-style @on is recognized (form directive name is "on")
    let input = r#"
.pack-card {
    @on &.click {
        $currentId <- $.dataset.packId;
    }
}
"#;
    let result = parse(input).unwrap();

    // Both animation and mutation forms use @on — macro_name is "on"
    let on_match = result.matches.iter().find(|m| m.macro_name == "on");
    assert!(
        on_match.is_some(),
        "Should have an 'on' FormMatch. Got: {:?}",
        result
            .matches
            .iter()
            .map(|m| &m.macro_name)
            .collect::<Vec<_>>()
    );

    // Check that body is captured
    let on_match = on_match.unwrap();
    assert!(
        on_match.captures.contains_key("body"),
        "Should have 'body' capture. Got: {:?}",
        on_match.captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_on_animation_form_match() {
    // Check that animation-style @on is recognized as "on"
    let input = r#"
.button {
    @on hover lift(350ms) {
        translate-y: 0 -> -6px;
    }
}
"#;
    let result = parse(input).unwrap();

    // Check that the animation form was matched (should be "on", not "on-mutation")
    let on_match = result.matches.iter().find(|m| m.macro_name == "on");
    assert!(
        on_match.is_some(),
        "Should have an 'on' FormMatch. Got: {:?}",
        result
            .matches
            .iter()
            .map(|m| &m.macro_name)
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// Descendant Combinator Selector Tests
// =============================================================================

#[test]
fn test_parse_descendant_element_selector() {
    // .parent a { color: blue; }
    let input = r#"
.nav-links a {
    color: blue;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope block, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, ".nav-links a");
    assert_eq!(result.scopes[0].css_declarations.len(), 1);
    assert_eq!(result.scopes[0].css_declarations[0].property, "color");
}

#[test]
fn test_parse_multi_level_descendant() {
    let input = r#"
nav ul li {
    list-style: none;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope block, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "nav ul li");
}

#[test]
fn test_parse_descendant_plus_compound() {
    // .wrapper div.inner { }
    let input = r#"
.wrapper div.inner {
    display: flex;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope block, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, ".wrapper div.inner");
}

#[test]
fn test_parse_descendant_with_universal() {
    let input = r#"
.container * {
    box-sizing: border-box;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope block, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, ".container *");
}

#[test]
fn test_parse_descendant_with_pseudo() {
    let input = r#"
.nav a:hover {
    color: red;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope block, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, ".nav a:hover");
}

#[test]
fn test_parse_class_descendant_class() {
    let input = r#"
.parent .child {
    opacity: 0.5;
}
"#;
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope block, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, ".parent .child");
}

// =============================================================================
// Pseudo-class / Pseudo-element Selector Tests (PROJ-054)
// =============================================================================

#[test]
fn test_parse_element_pseudo_selector() {
    // nav a:hover {} — element-only with pseudo
    let input = "nav a:hover {\n    color: red;\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "nav a:hover");
}

#[test]
fn test_parse_element_pseudo_no_descendant() {
    // div:first-child {} — single element with pseudo
    let input = "div:first-child {\n    margin: 0;\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "div:first-child");
}

#[test]
fn test_parse_element_pseudo_element() {
    // a::before {} — pseudo-element (double colon)
    let input = "a::before {\n    content: \"\";\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "a::before");
}

#[test]
fn test_parse_element_functional_pseudo() {
    // li:nth-child(2n+1) {} — functional pseudo with parens
    let input = "li:nth-child(2n+1) {\n    background: gray;\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "li:nth-child(2n+1)");
}

#[test]
fn test_parse_element_chained_pseudos() {
    // a:hover:focus {} — chained pseudos
    let input = "a:hover:focus {\n    outline: none;\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "a:hover:focus");
}

#[test]
fn test_parse_multi_descendant_with_pseudo() {
    // ul li:last-child {} — multi-level with pseudo at end
    let input = "ul li:last-child {\n    font-weight: bold;\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(
        result.scopes.len(),
        1,
        "Should parse one scope, got: {:#?}",
        result.scopes
    );
    assert_eq!(result.scopes[0].selector, "ul li:last-child");
}

#[test]
fn test_css_property_not_confused_with_pseudo() {
    // opacity: 0.5; inside a scope must remain a property, not a selector
    let input = ".test {\n    opacity: 0.5;\n    color: red;\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes[0].css_declarations.len(), 2);
    assert_eq!(result.scopes[0].css_declarations[0].property, "opacity");
}

// =============================================================================
// Multi-Error Reporting Tests (ParseErrors newtype)
// =============================================================================

#[test]
fn parse_returns_multiple_errors() {
    // Two independent broken scope blocks → at least 2 errors
    let input = ".a { @@@ }\n.b { @@@ }";
    let errs = parse(input).unwrap_err();
    assert!(errs.len() >= 2, "got {}: {:?}", errs.len(), errs);
}

#[test]
fn parse_errors_first() {
    let errs = parse(".x {").unwrap_err();
    let first = errs.first();
    assert!(!first.message.is_empty());
}

#[test]
fn parse_errors_display_single() {
    let errs = parse(".x {").unwrap_err();
    let s = format!("{}", errs);
    assert!(s.contains("at offset"));
}

#[test]
fn parse_errors_display_multi() {
    let errs = parse(".a { @@@ }\n.b { @@@ }").unwrap_err();
    if errs.len() >= 2 {
        let s = format!("{}", errs);
        assert!(s.contains("parse errors:"));
    }
}

#[test]
fn parse_errors_to_miette_reports() {
    let src = ".a { @@@ }\n.b { @@@ }";
    let errs = parse(src).unwrap_err();
    let reports = errs.to_miette_reports(src, "test.st");
    assert_eq!(reports.len(), errs.len());
}

#[test]
fn parse_errors_render_all_plain() {
    let src = ".a { @@@ }";
    let errs = parse(src).unwrap_err();
    let rendered = errs.render_all_plain(src, "test.st");
    assert!(!rendered.is_empty());
}

// =============================================================================
// Nested Scope Selector Tests (PROJ-054)
// =============================================================================

#[test]
fn test_parse_child_combinator_nested() {
    // > .child {} inside a parent scope should preserve ">" in the selector
    let input = ".parent {\n    > .child {\n        margin: 0;\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let parent = &result.scopes[0];
    assert_eq!(parent.selector, ".parent");
    assert_eq!(parent.nested_scopes.len(), 1, "should have 1 nested scope");
    assert!(
        parent.nested_scopes[0].selector.contains('>'),
        "nested selector should contain '>': got {:?}",
        parent.nested_scopes[0].selector
    );
}

#[test]
fn test_parse_adjacent_sibling_nested() {
    // + .sibling {} inside a parent scope
    let input = ".parent {\n    + .sibling {\n        color: red;\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes[0].nested_scopes.len(), 1);
    assert!(
        result.scopes[0].nested_scopes[0].selector.contains('+'),
        "nested selector should contain '+': got {:?}",
        result.scopes[0].nested_scopes[0].selector
    );
}

#[test]
fn test_parse_ampersand_pseudo_nested() {
    // &:hover {} should be parsed as a nested scope, not an element ref
    let input = ".btn {\n    &:hover {\n        color: red;\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let parent = &result.scopes[0];
    assert_eq!(parent.selector, ".btn");
    assert_eq!(
        parent.nested_scopes.len(),
        1,
        "should have 1 nested scope for &:hover"
    );
    assert_eq!(parent.nested_scopes[0].selector, "&:hover");
}

#[test]
fn test_parse_ampersand_class_nested() {
    // &.active {} should be parsed as a nested scope
    let input = ".card {\n    &.active {\n        opacity: 1;\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes[0].nested_scopes.len(), 1);
    assert_eq!(result.scopes[0].nested_scopes[0].selector, "&.active");
}

#[test]
fn test_parse_ampersand_pseudo_element() {
    // &::before {} should be parsed as a nested scope
    let input = ".item {\n    &::before {\n        content: \"\";\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes[0].nested_scopes.len(), 1);
    let sel = &result.scopes[0].nested_scopes[0].selector;
    assert!(
        sel.contains("&::before") || sel.contains("& ::before"),
        "selector should reference &::before, got {:?}",
        sel
    );
}

#[test]
fn test_parse_ampersand_bare_scope() {
    // & {} (bare ampersand scope) should be parsed as a nested scope
    let input = ".parent {\n    & {\n        color: blue;\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes[0].nested_scopes.len(), 1);
    assert_eq!(result.scopes[0].nested_scopes[0].selector.trim(), "&");
}

#[test]
fn test_parse_element_ref_still_works() {
    // &name should still be parsed as element reference (regression guard)
    let input = ".parent {\n    &header .title;\n}\n";
    let result = parse(input).unwrap();
    // &header should NOT appear as a nested scope — it's an element ref
    assert_eq!(
        result.scopes[0].nested_scopes.len(),
        0,
        "element ref &header should not produce a nested scope"
    );
}

#[test]
fn test_comment_before_nested_selector_not_in_ast() {
    // A // comment before a nested .child selector should not leak into the selector text
    let input = ".parent {\n    // comment\n    .child {\n        color: red;\n    }\n}\n";
    let result = parse(input).unwrap();
    assert_eq!(result.scopes.len(), 1);
    let parent = &result.scopes[0];
    assert_eq!(parent.nested_scopes.len(), 1, "should have 1 nested scope");
    let sel = &parent.nested_scopes[0].selector;
    assert!(
        !sel.contains("//"),
        "selector should not contain comment marker: got {:?}",
        sel
    );
    assert!(
        !sel.contains("comment"),
        "selector should not contain comment text: got {:?}",
        sel
    );
    assert!(
        sel.contains(".child"),
        "selector should contain .child: got {:?}",
        sel
    );
}

// =============================================================================
// split_respecting_nesting quote handling
// =============================================================================

#[test]
fn split_respecting_nesting_handles_double_quoted_commas() {
    use super::split_respecting_nesting;
    // Commas inside double-quoted strings should NOT split
    let result = split_respecting_nesting(r#"a: $a:string = "x, y, z", b: $b:number = 1"#, ',');
    assert_eq!(result.len(), 2, "expected 2 parts, got {:?}", result);
    assert!(
        result[0].contains(r#""x, y, z""#),
        "first part should contain the full quoted string, got {:?}",
        result[0]
    );
}

#[test]
fn split_respecting_nesting_handles_single_quoted_commas() {
    use super::split_respecting_nesting;
    // Commas inside single-quoted strings should NOT split
    let result = split_respecting_nesting("a: $a:string = 'x, y', b: $b:number = 1", ',');
    assert_eq!(result.len(), 2, "expected 2 parts, got {:?}", result);
}

#[test]
fn split_respecting_nesting_handles_cursor_default() {
    use super::split_respecting_nesting;
    // Simulates the @cursor hoverElements default: "a, button, [data-cursor]"
    let result = split_respecting_nesting(
        r#"hoverElements: $hoverElements:string = "a, button, [data-cursor]""#,
        ',',
    );
    assert_eq!(
        result.len(),
        1,
        "cursor default with commas in quotes should be 1 part, got {:?}",
        result
    );
}

#[test]
fn test_parse_bind_args_quoted_string_with_colon() {
    let input = r#"
%macro breakpoint {
    %creates @breakpoint

    %binds {
        media("(min-width: 768px)") -> { $matches }
    }
}
"#;
    let result = parse(input).unwrap();

    if let crate::parser::meta_ast::MetaDef::Macro(mac) = &result.meta_defs[0] {
        let bind = &mac.binds[0];
        // The string arg "(min-width: 768px)" should be positional, not named
        assert_eq!(bind.args.len(), 1, "expected 1 arg, got {:?}", bind.args);
        match &bind.args[0] {
            crate::parser::meta_ast::BindArg::Positional(
                crate::parser::meta_ast::BindValue::String(s),
            ) => {
                assert_eq!(s, "(min-width: 768px)");
            }
            other => panic!("Expected Positional(String), got {:?}", other),
        }
    } else {
        panic!("Expected macro");
    }
}

#[test]
fn parse_vendor_def_full() {
    use crate::parser::meta_ast::{MetaDef, VendorBundleStrategy};
    let input = r#"%vendor pretext {
  source:  submodule "vendor/pretext"
  entry:   "src/layout.ts"
  bundle:  bun
  exports: { prepare, layout, walkLineRanges }
  out:     "vendor/pretext.bundle.js"
  license: MIT
}"#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "Failed to parse %vendor: {:?}",
        result.err()
    );
    let ast = result.unwrap();
    assert_eq!(ast.meta_defs.len(), 1);
    match &ast.meta_defs[0] {
        MetaDef::Vendor(v) => {
            assert_eq!(v.name, "pretext");
            assert_eq!(v.source, "vendor/pretext");
            assert_eq!(v.entry, "src/layout.ts");
            assert_eq!(v.bundle, VendorBundleStrategy::Bun);
            assert_eq!(v.out, "vendor/pretext.bundle.js");
            assert_eq!(v.license.as_deref(), Some("MIT"));
            assert_eq!(v.exports, vec!["prepare", "layout", "walkLineRanges"]);
        }
        other => panic!("Expected MetaDef::Vendor, got {:?}", other),
    }
}

#[test]
fn parse_vendor_def_direct_strategy() {
    use crate::parser::meta_ast::{MetaDef, VendorBundleStrategy};
    let input = r#"%vendor smol {
  source: submodule "vendor/smol"
  entry: "index.js"
  bundle: direct
  exports: { foo }
  out: "vendor/smol.bundle.js"
}"#;
    let ast = parse(input).unwrap();
    match &ast.meta_defs[0] {
        MetaDef::Vendor(v) => {
            assert_eq!(v.bundle, VendorBundleStrategy::Direct);
            assert_eq!(v.license, None);
            assert_eq!(v.exports, vec!["foo"]);
        }
        other => panic!("Expected MetaDef::Vendor, got {:?}", other),
    }
}

#[test]
fn vendor_def_registers_and_roundtrips() {
    use crate::metasystem::MetaRegistry;
    let input = r#"%vendor pretext {
  source: submodule "vendor/pretext"
  entry: "src/layout.ts"
  bundle: bun
  exports: { prepare, layout }
  out: "vendor/pretext.bundle.js"
  license: MIT
}"#;
    let ast = parse(input).unwrap();
    let mut registry = MetaRegistry::new();
    registry.load_from_defs(ast.meta_defs).unwrap();
    let v = registry
        .get_vendor("pretext")
        .expect("vendor should be registered");
    assert_eq!(v.entry, "src/layout.ts");
    assert_eq!(v.exports.len(), 2);
    assert_eq!(registry.vendors().count(), 1);
}

#[test]
fn feat116_editable_mark_body_is_a_construct_scope() {
    // FEAT-116 / Q3 generalization, end-to-end at the parser: a body-bearing
    // construct BEYOND @template (@editable-mark) gets the SAME World-A Construct
    // scope, with its body STATE and class-toggle DIRECTIVE surfaced per-instance —
    // purely because its %form carries `$body:component_body`. No @template-specific
    // special-casing, no hardcoded name set (the predicate is registry-derived).
    use crate::parser::ast::ScopeKind;
    let src = "@editable-mark &callout(&sel) {\n  <aside class=\"callout\">`&sel`</aside>\n  $hot bool: false;\n  .callout--hot: $hot;\n}\n";
    let f = parse(src).expect("parse");
    let mut construct_scopes = 0usize;
    let mut found_state = false;
    let mut found_toggle = false;
    for s in &f.scopes {
        if let ScopeKind::Construct(_) = &s.kind {
            construct_scopes += 1;
            for m in &s.matches {
                if m.macro_name == "local-state" {
                    if let Some(CapturedValue::Ident(n)) = m.captures.get("name") {
                        if n == "hot" {
                            found_state = true;
                        }
                    }
                }
            }
            for ns in &s.nested_scopes {
                for d in &ns.css_declarations {
                    if d.property.contains("callout--hot") {
                        found_toggle = true;
                    }
                }
            }
        }
    }
    assert!(
        construct_scopes >= 1,
        "@editable-mark must yield a Construct scope"
    );
    assert!(
        found_state,
        "body state $hot must surface into the Construct scope"
    );
    assert!(
        found_toggle,
        "class-toggle .callout--hot must surface into a nested scope"
    );
}

#[test]
fn reconstruct_template_html_is_clean_world_a_source() {
    // FEAT-115 final: the factory `html` is sourced SOLELY from the CST body via
    // `reconstruct_template_body_html` (span-subtraction), NOT reify's text
    // segmentation (which leaked construct segments — the S3a shipping bug). This is
    // the live home of the old BUG-059 reify-html cases: a construct segment is
    // CONSUMED (never leaked into html), and HTML around it survives verbatim.
    // FEAT-119: html is read from the World-A template SCOPE (`@template:<name>`),
    // not the retired ComponentBody capture (whose payload fields are gone).
    let html_of = |src: &str| -> String {
        let f = parse(src).expect("parse");
        f.scopes
            .iter()
            .find(|s| {
                matches!(&s.kind, crate::parser::ast::ScopeKind::Construct(_))
                    && s.selector.starts_with("@template:")
            })
            .map(|s| s.html.clone())
            .unwrap_or_default()
    };

    // state decl before HTML: decl consumed, HTML survives (BUG-059 R1)
    let h = html_of("@template &c() {\n$n number: 0;\n<div class=\"c\"><b>hi</b></div>\n}\n");
    assert!(
        !h.contains("$n number"),
        "state decl must not leak into html: {h:?}"
    );
    assert!(
        h.contains("<b>hi</b>"),
        "HTML after a state decl must survive: {h:?}"
    );

    // HTML then state decl: decl consumed, HTML survives (BUG-059 control)
    let h = html_of("@template &c() {\n<div class=\"c\"><b>hi</b></div>\n$n number: 0;\n}\n");
    assert!(
        h.contains("<b>hi</b>") && !h.contains("$n number"),
        "html: {h:?}"
    );

    // void elements must not desync depth and strand a trailing state decl (BUG-059)
    let h =
        html_of("@template &c() {\n<div>\n<img src=\"x.png\">\n<br>\n</div>\n$n number: 0;\n}\n");
    assert!(
        !h.contains("$n number"),
        "decl after void elements consumed: {h:?}"
    );
    assert!(h.contains("<img"), "void element survives: {h:?}");

    // a `.sel { … }` CSS block is consumed (not leaked into html) — the S3a fix
    let h = html_of("@template &c() {\n<div class=\"c\"></div>\n.c { color: red; }\n}\n");
    assert!(
        !h.contains("color: red"),
        "css block must not leak into html: {h:?}"
    );
    assert!(
        h.contains("<div class=\"c\">"),
        "html survives the css block: {h:?}"
    );

    // pure HTML body survives verbatim
    let h = html_of("@template &c() {\n<div><p>Hello</p></div>\n}\n");
    assert!(
        h.contains("<div><p>Hello</p></div>"),
        "pure html survives: {h:?}"
    );
}

#[test]
fn test_w0712_each_in_template_body_markup_warns() {
    // BUG-130: an `@each` placed directly in a @template body's MARKUP renders
    // nothing (it's swallowed as opaque HTML). Emit W0712 so the silent failure is
    // surfaced, pointing the author to the selector-scoped form.
    let input = "@template &lst() { <ul>@each($items as $x) { &item($x); }</ul> }\n";
    let result = parse(input).unwrap();
    let w = result
        .diagnostics
        .iter()
        .find(|d| d.code == crate::diagnostics::DiagnosticCode::W0712);
    let w = w.expect("W0712 should fire for @each in template-body markup");
    assert!(
        w.message.contains("each") && w.message.contains("lst"),
        "message names the directive + template: {}",
        w.message
    );
    assert!(
        w.hint.is_some(),
        "W0712 carries a fix hint (use a selector scope)"
    );
}

#[test]
fn test_w0712_not_fired_for_selector_scoped_each() {
    // The SUPPORTED idiom (@each inside a `.sel{}` scope within the template body)
    // must NOT warn — it parses + renders correctly.
    let input = "@template &lst() {\n  <ul class=\"l\"></ul>\n  .l { @each($items as $x) { &item($x); } }\n}\n";
    let result = parse(input).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == crate::diagnostics::DiagnosticCode::W0712),
        "selector-scoped @each must NOT trigger W0712: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| (d.code, &d.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_e0929_colon_in_template_param_list_errors() {
    // B1 fail-loud: a `:` in a @template param list previously truncated the list
    // SILENTLY at the first param. It must now surface E0929 instead of losing data.
    let input = "@template &main($a: \"one\", $b: \"two\") { <h1>`$a` `$b`</h1> }\n";
    let result = parse(input).unwrap();
    let e = result
        .diagnostics
        .iter()
        .find(|d| d.code == crate::diagnostics::DiagnosticCode::E0929);
    let e = e.expect("E0929 should fire for a `:` in a @template param list");
    assert!(
        e.message.contains("main") && e.message.contains(':'),
        "message names the template + the offending sigil: {}",
        e.message
    );
    assert!(
        e.hint.is_some(),
        "E0929 carries a fix hint (use a space for type, `=` for default)"
    );
}

#[test]
fn test_e0929_not_fired_for_space_type_and_eq_default() {
    // The VALID param forms — space-form type (`$href url`) and `=` default
    // (`$alt string = \"\"`) — must NOT trip E0929. Multiple params must all survive.
    let input =
        "@template &card($href url, $alt string = \"\", $count number = 0) { <a>`$alt`</a> }\n";
    let result = parse(input).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == crate::diagnostics::DiagnosticCode::E0929),
        "space-type + `=`-default params must NOT trigger E0929: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| (d.code, &d.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_e0929_not_fired_for_colon_inside_default_string() {
    // A `:` INSIDE a default string literal (`$t = \"a:b\"`) is data, not a separator,
    // and must NOT trip E0929 (the scanner ignores string interiors).
    let input = "@template &t($t string = \"a:b\") { <p>`$t`</p> }\n";
    let result = parse(input).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == crate::diagnostics::DiagnosticCode::E0929),
        "a `:` inside a default string must NOT trigger E0929: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| (d.code, &d.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_w0712_not_fired_for_supported_body_directives() {
    // A body-root @on (a supported behavioral directive) must NOT warn — only the
    // data-rendering @each/@view forms are dropped.
    let input = "@template &b() {\n  <button>Go</button>\n  @on &.click { $n number: 0; }\n}\n";
    let result = parse(input).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == crate::diagnostics::DiagnosticCode::W0712),
        "@on in a template body must NOT trigger W0712"
    );
}

// =============================================================================
// BUG-133: unknown top-level @directive warning (W0714)
// =============================================================================

#[test]
fn test_w0714_unknown_top_level_directive() {
    let input = "@bogusdirective { foo: bar; }\n";
    let result = parse(input).unwrap();
    let w = result
        .diagnostics
        .iter()
        .find(|d| d.code == crate::diagnostics::DiagnosticCode::W0714);
    let w = w.expect("W0714 should fire for an unknown top-level directive");
    assert!(
        w.message.contains("bogusdirective"),
        "message names the unknown directive: {}",
        w.message
    );
    assert!(
        w.message.contains("not a known directive"),
        "message explains it is unknown: {}",
        w.message
    );
    assert!(w.hint.is_some(), "W0714 carries a fix hint");
}

#[test]
fn test_w0714_not_fired_for_known_directives() {
    // A file using real FormMatch directives, structural directives, and imports
    // must remain silent (zero W0714).
    let input = r#"@import "stdlib/testing/test";

body {
    $count number: 0;
}

.nav {
    @on &.click { $count <- $count + 1; }
    @bind(class: "active", when: $count > 0)
}

@template &counter() {
    <button class="c">Go</button>
}

@media (min-width: 800px) {
    .nav { color: red; }
}
"#;
    let result = parse(input).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == crate::diagnostics::DiagnosticCode::W0714),
        "known directives must NOT trigger W0714: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| (&d.code, &d.message))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_w0714_not_fired_during_bootstrap() {
    // Bootstrap parse has no registry; unknown directives must stay silent.
    let input = "@bogusdirective { foo: bar; }\n";
    let result = parse_for_bootstrap(input).unwrap();
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == crate::diagnostics::DiagnosticCode::W0714),
        "bootstrap parse (registry None) must not emit W0714: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| (&d.code, &d.message))
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// %derives / %animates clause splitting — multi-line + string state (audit #8)
// =============================================================================

#[test]
fn parse_clause_decls_basic_multivalue() {
    let decls = parse_clause_decls("a: 1\nb: 2");
    assert_eq!(
        decls,
        vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string())
        ]
    );
}

#[test]
fn parse_clause_decls_multiline_ternary_not_split() {
    // A multi-line ternary value must stay attached to its declaration and not
    // swallow the next decl (the original %derives bug this splitter fixed).
    let inner = "x: $a\n  ? 1\n  : 0\ny: $b";
    let decls = parse_clause_decls(inner);
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].0, "x");
    assert!(decls[0].1.contains("? 1") && decls[0].1.contains(": 0"));
    assert_eq!(decls[1].0, "y");
    assert_eq!(decls[1].1, "$b");
}

#[test]
fn parse_clause_decls_multiline_string_carries_state() {
    // audit #8: a string literal spanning lines AND containing an unbalanced
    // bracket must not miscount depth, and the continuation line (which itself
    // looks like `key:`) must NOT be read as a new declaration.
    let inner = "msg: \"line one (\n  fake: still in string )\"\nnext: 2";
    let decls = parse_clause_decls(inner);
    // Exactly two decls: `msg` (the whole multi-line string) and `next`.
    assert_eq!(decls.len(), 2, "got: {:?}", decls);
    assert_eq!(decls[0].0, "msg");
    assert!(
        decls[0].1.contains("fake: still in string"),
        "the multi-line string body must stay in the msg value: {:?}",
        decls[0].1
    );
    assert_eq!(decls[1].0, "next");
    assert_eq!(decls[1].1, "2");
}

// =============================================================================
// FEAT-142 WAVE A: entity scopes (`&name { @directive... }`)
// =============================================================================

#[test]
fn test_parse_entity_scope_lowers_with_synthetic_selector() {
    let input = r#"
&evernet { @scroll-spy }
"#;
    let result = parse(input).unwrap();

    // Should harvest exactly one entity scope. Entity scopes are plain SELECTOR
    // scopes (no dedicated ScopeKind arm — elegance bar); they are identified by
    // their synthetic marker-class selector `.st-entity-<name>`.
    let entities: Vec<_> = result
        .scopes
        .iter()
        .filter(|s| s.selector.starts_with(".st-entity-"))
        .collect();
    assert_eq!(
        entities.len(),
        1,
        "expected one entity scope, got {:?}",
        result.scopes
    );

    let entity = entities[0];
    assert_eq!(entity.selector, ".st-entity-evernet");
    assert!(
        matches!(entity.kind, ScopeKind::Selector),
        "entity scope must be a plain Selector scope"
    );

    // The entity registration primitive should be synthesized.
    let entity_registrations: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.macro_name == "entity-scope-impl")
        .collect();
    assert_eq!(
        entity_registrations.len(),
        1,
        "expected one entity-scope-impl match, got {:?}",
        result
            .matches
            .iter()
            .map(|m| &m.macro_name)
            .collect::<Vec<_>>()
    );
    let registration = entity_registrations[0];
    assert_eq!(registration.get_ident("name"), Some("evernet"));

    // The interior @scroll-spy directive should bind to the entity marker.
    let scroll_spies: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.macro_name == "scroll-spy")
        .collect();
    assert_eq!(
        scroll_spies.len(),
        1,
        "expected one scroll-spy match, got {:?}",
        result
            .matches
            .iter()
            .map(|m| &m.macro_name)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        scroll_spies[0].selector.as_deref(),
        Some(".st-entity-evernet"),
        "scroll-spy must bind to the entity marker selector"
    );
}

#[test]
fn test_dotted_ref_not_harvested_as_entity() {
    // FEAT-142 (reviewer P2): a DOTTED ref `&name.component { … }` is a facet PATH,
    // not a base entity declaration. It must NOT be mis-harvested as the base
    // entity `name` (which would silently drop the `.component` segment and bind
    // the block to `.st-entity-name`). Until the Wave-B/C facet machinery handles
    // it, it simply must not become an entity scope.
    let result = parse("&evernet.scrollSpy { @scroll-spy }\n").unwrap();
    let entity_scopes: Vec<_> = result
        .scopes
        .iter()
        .filter(|s| s.selector == ".st-entity-evernet")
        .collect();
    assert!(
        entity_scopes.is_empty(),
        "a dotted facet-path ref must NOT be harvested as the base entity; got scopes {:?}",
        result
            .scopes
            .iter()
            .map(|s| s.selector.clone())
            .collect::<Vec<_>>()
    );
    let entity_regs = result
        .matches
        .iter()
        .filter(|m| m.macro_name == "entity-scope-impl")
        .count();
    assert_eq!(
        entity_regs, 0,
        "a dotted facet-path ref must NOT synthesize an entity registration"
    );
}

#[test]
fn wavee_entity_in_template_harvested() {
    // FEAT-142 WAVE E: &name{@c} nested inside a @template body must harvest as an
    // entity scope + synthesize its registration (so data-driven entities work).
    let input = "@import \"stdlib\"\n@template &spawn($r) { &node { @scroll-spy } }\n";
    let result = parse(input).unwrap();
    let ents = result
        .matches
        .iter()
        .filter(|m| m.macro_name == "entity-scope-impl")
        .count();
    assert_eq!(
        ents, 1,
        "entity scope inside a @template body must synthesize a registration match"
    );
}

#[test]
fn malformed_data_subscribe_is_a_hard_error() {
    let ast = parse("@data subscribe $x zzz : q ;").expect("source parses structurally");
    let diagnostic = ast
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == crate::diagnostics::DiagnosticCode::E0945)
        .expect("malformed @data subscribe must not silently fall through");
    assert!(
        diagnostic
            .message
            .contains("$name <type>? from $host : <assign>")
    );
}

// --- Counted repetition in capture patterns (FEAT-164 / PLAN-122 W0) --------
//
// The acceptance for the wave: a hex-colour grammar becomes WRITABLE in stdlib
// pattern syntax. Before counted repetition there was no spelling for "3, 4, 6
// or 8 of these", so the length constraint had to live in the Rust lexer's
// COLOR regex — the exact hardcoding PLAN-122 exists to dissolve.

#[test]
fn counted_hex_colour_grammar_parses_from_stdlib_syntax() {
    let pat = crate::parser::parse_capture_pattern(r##""#" [0-9a-fA-F]{3|4|6|8}"##)
        .expect("hex colour pattern must parse");
    let crate::parser::meta_ast::CapturePatternAst::Sequence(elems) = pat else {
        panic!("expected a sequence, got {pat:?}");
    };
    assert_eq!(elems.len(), 2, "literal `#` then the counted class");
    match &elems[1] {
        crate::parser::meta_ast::CapturePatternAst::CharClass { chars, counts, .. } => {
            assert_eq!(chars, "0-9a-fA-F");
            assert_eq!(
                counts.expect("must be counted").counts(),
                &[8, 6, 4, 3],
                "counts are stored longest-first so the longest legal run wins"
            );
        }
        other => panic!("expected a counted char class, got {other:?}"),
    }
}

#[test]
fn counted_single_count_form_parses() {
    let pat = crate::parser::parse_capture_pattern("[0-9]{4}").expect("single count parses");
    match pat {
        crate::parser::meta_ast::CapturePatternAst::CharClass { counts, .. } => {
            assert_eq!(counts.expect("counted").counts(), &[4]);
        }
        other => panic!("expected char class, got {other:?}"),
    }
}

#[test]
fn counted_malformed_brace_does_not_swallow_the_class() {
    // A malformed brace run must NOT become a repetition that matches nothing.
    // The class stays uncounted and the `{` falls through to the surrounding
    // pattern grammar — degrading loudly beats degrading silently.
    for src in ["[0-9]{}", "[0-9]{x}", "[0-9]{3"] {
        let pat = crate::parser::parse_capture_pattern(src)
            .unwrap_or_else(|| panic!("{src} must still parse the class"));
        match pat {
            crate::parser::meta_ast::CapturePatternAst::CharClass { counts, .. } => {
                assert!(counts.is_none(), "{src} must not produce counts");
            }
            other => panic!("{src}: expected char class, got {other:?}"),
        }
    }
}
