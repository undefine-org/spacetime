//! Integration tests for the unified `@data query` / `@data fold` surface.
//!
//! Verifies the array-pipeline (`@data query`) and reduce-to-scalar (`@data fold`)
//! kinds are parsed as FormMatches with the right captures. These superseded the
//! legacy `@computed { from/where/sort/limit/reduce }` block-form, removed in
//! PLAN-023 W5/FEAT-047. Codegen is handled by the metasystem via
//! stdlib/macros/data-kind.st (data-query -> computed-source, data-fold ->
//! derived-signal reduce).
//!
//! `macro_name` is always "data"; the specific kind is in `matched_macro`.

use spacetime::parse;

/// Shared preamble: a Product type + a fetched products source the queries read.
const PREAMBLE: &str = r#"
@type Product {
    id: string;
    name: string;
    price: number;
    inStock: boolean;
    discount: number;
}

@data fetch $products Product[] : "/api/products"
"#;

fn parse_with_preamble(body: &str) -> spacetime::parser::ast::StFile {
    parse(&format!("{PREAMBLE}\n{body}")).expect("Failed to parse")
}

/// All `@data query` matches at file scope.
fn queries(ast: &spacetime::parser::ast::StFile) -> Vec<&spacetime::syntax::FormMatch> {
    ast.matches
        .iter()
        .filter(|m| m.selector.is_none() && m.matched_macro.as_deref() == Some("data-query"))
        .collect()
}

#[test]
fn test_query_basic_generation() {
    let ast = parse_with_preamble(
        "@data query $activeProducts Product[] from $products { where: item.inStock }",
    );
    let q = queries(&ast);
    assert_eq!(q.len(), 1, "Should have 1 @data query FormMatch");

    let name = q[0]
        .get_binding("name")
        .map(|s| s.strip_prefix('$').unwrap_or(s));
    assert_eq!(name, Some("activeProducts"), "Should capture name");
    assert!(
        q[0].captures.contains_key("source"),
        "Should have 'source' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_query_with_where_filter() {
    let ast = parse_with_preamble(
        "@data query $activeProducts Product[] from $products { where: item.inStock == true }",
    );
    let q = queries(&ast);
    assert_eq!(q.len(), 1);
    assert!(
        q[0].captures.contains_key("source"),
        "Should have 'source' capture"
    );
    assert!(
        q[0].captures.contains_key("filter"),
        "Should have 'filter' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_query_with_sort() {
    let ast = parse_with_preamble(
        "@data query $sortedProducts Product[] from $products { sort: item.price; dir: asc }",
    );
    let q = queries(&ast);
    assert!(
        q[0].captures.contains_key("sortBy"),
        "Should have 'sortBy' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_query_with_sort_desc() {
    let ast = parse_with_preamble(
        "@data query $expensiveProducts Product[] from $products { sort: item.price; dir: desc }",
    );
    let q = queries(&ast);
    assert!(
        q[0].captures.contains_key("sortBy"),
        "Should have 'sortBy' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        q[0].get_ident("sortDir")
            .or_else(|| match q[0].get("sortDir") {
                Some(spacetime::syntax::CapturedValue::Ident(s)) => Some(s.as_str()),
                _ => None,
            }),
        Some("desc"),
        "Should capture sort direction desc"
    );
}

#[test]
fn test_query_with_limit() {
    let ast =
        parse_with_preamble("@data query $topProducts Product[] from $products { limit: 10 }");
    let q = queries(&ast);
    assert!(
        q[0].captures.contains_key("limit"),
        "Should have 'limit' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_fold_with_reduce() {
    // Reduce-to-scalar is `@data fold` (acc/item reducer locals, seeded initial 0).
    let ast =
        parse_with_preamble("@data fold $totalPrice number from $products : acc + item.price ;");
    let fold: Vec<_> = ast
        .matches
        .iter()
        .filter(|m| m.selector.is_none() && m.matched_macro.as_deref() == Some("data-fold"))
        .collect();
    assert_eq!(fold.len(), 1, "Should have 1 @data fold FormMatch");
    assert!(
        fold[0].captures.contains_key("source"),
        "fold should capture its source, got: {:?}",
        fold[0].captures.keys().collect::<Vec<_>>()
    );
    assert!(
        fold[0].captures.contains_key("value"),
        "fold should capture its step expression as 'value', got: {:?}",
        fold[0].captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_query_combined_operations() {
    let ast = parse_with_preamble(
        "@data query $topInStockProducts Product[] from $products { where: item.inStock == true; sort: item.price; dir: desc; limit: 5 }",
    );
    let q = queries(&ast);
    assert!(q[0].captures.contains_key("source"));
    assert!(
        q[0].captures.contains_key("filter"),
        "Should have 'filter' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
    assert!(
        q[0].captures.contains_key("sortBy"),
        "Should have 'sortBy' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
    assert!(
        q[0].captures.contains_key("limit"),
        "Should have 'limit' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_multiple_query_definitions() {
    let ast = parse_with_preamble(
        "@data query $inStockProducts Product[] from $products { where: item.inStock == true }\n@data query $outOfStockProducts Product[] from $products { where: item.inStock == false }",
    );
    let q = queries(&ast);
    assert_eq!(q.len(), 2, "Should have 2 @data query FormMatches");
}

#[test]
fn test_query_with_each_blocks() {
    let ast = parse_with_preamble(
        "@data query $topProducts Product[] from $products { sort: item.price; dir: desc; limit: 10 }\n\n.product-grid {\n    @each($products as $p) {\n        &product-card($p);\n    }\n}",
    );

    assert!(
        ast.matches
            .iter()
            .any(|m| m.selector.is_none() && m.macro_name == "type"),
        "Should have @type FormMatch"
    );
    assert!(
        ast.matches
            .iter()
            .any(|m| m.selector.is_none() && m.macro_name == "data"),
        "Should have @data FormMatch"
    );
    assert!(
        ast.matches
            .iter()
            .any(|m| m.selector.is_none() && m.matched_macro.as_deref() == Some("data-query")),
        "Should have @data query FormMatch"
    );

    assert_eq!(ast.scopes.len(), 1, "Should have 1 scope");
    assert!(
        ast.scopes[0].matches.iter().any(|m| m.macro_name == "each"),
        "Should have @each in scope"
    );
}

#[test]
fn test_query_name_and_type_in_inline_args() {
    let ast = parse_with_preamble(
        "@data query $processedProducts Product[] from $products { where: item.id }",
    );
    let q = queries(&ast);
    assert_eq!(q.len(), 1);

    let name = q[0]
        .get_binding("name")
        .map(|s| s.strip_prefix('$').unwrap_or(s));
    assert_eq!(name, Some("processedProducts"));
    assert!(
        q[0].captures.contains_key("type"),
        "Should capture the element type"
    );
    assert!(
        q[0].captures.contains_key("source"),
        "Should have 'source' capture"
    );
}

#[test]
fn test_query_expression_in_where() {
    // `*` inside a value expr must be parenthesized (it doubles as the universal
    // selector / ZeroOrMore in directive inline-arg position).
    let ast = parse_with_preamble(
        "@data query $discountedProducts Product[] from $products { where: (item.price * (1 - item.discount)) < 50 }",
    );
    let q = queries(&ast);
    assert!(
        q[0].captures.contains_key("filter"),
        "Should have 'filter' capture, got: {:?}",
        q[0].captures.keys().collect::<Vec<_>>()
    );
}
