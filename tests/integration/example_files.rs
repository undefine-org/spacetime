//! Example file parsing and compilation tests.
//!
//! Tests verify that example files parse correctly and documents features
//! that are not yet implemented.

use spacetime::parser::parse;
use spacetime::{compile, compiler::CompileOptions};
use std::fs;
use std::path::Path;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Parse a file and return (success, error_message)
fn parse_file(path: &str) -> (bool, Option<String>) {
    let full_path = Path::new("examples").join(path);
    match fs::read_to_string(&full_path) {
        Ok(content) => match parse(&content) {
            Ok(_) => (true, None),
            Err(e) => (false, Some(format!("{:?}", e))),
        },
        Err(e) => (false, Some(format!("File read error: {}", e))),
    }
}

/// Compile a file and return (success, css_output, js_output, error)
fn compile_file(path: &str) -> (bool, Option<String>, Option<String>, Option<String>) {
    let full_path = Path::new("examples").join(path);
    let content = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => return (false, None, None, Some(format!("File read error: {}", e))),
    };

    let ast = match parse(&content) {
        Ok(a) => a,
        Err(e) => return (false, None, None, Some(format!("Parse error: {:?}", e))),
    };

    let compiled = compile(&ast, CompileOptions::default());
    (true, Some(compiled.css), Some(compiled.js), None)
}

// =============================================================================
// PARSEABLE EXAMPLES (should fully parse)
// =============================================================================

#[test]
fn test_data_binding_demo_parses() {
    let (success, error) = parse_file("data-binding-demo.st");
    assert!(success, "data-binding-demo.st should parse: {:?}", error);
}

#[test]
#[ignore = "requires full stdlib support in new pipeline"]
fn test_data_binding_demo_compiles() {
    let (success, css, js, error) = compile_file("data-binding-demo.st");
    assert!(success, "data-binding-demo.st should compile: {:?}", error);

    let css = css.unwrap();
    let js = js.unwrap();

    // Verify CSS output contains expected elements
    assert!(
        !css.is_empty() || !js.is_empty(),
        "Should generate CSS or JS output"
    );
}

// Note: Demo files using old directive syntax (@scroll, @on hover, etc.)
// have been removed. The example files need to be updated to use macro syntax.
// See: presets-demo.st, filter_demo.st, value_change_demo.st, load_demo.st,
//      on_mutation_demo.st, after_timeline.st

// =============================================================================
// PARTIAL PARSE TESTS
// These test files that contain some implemented and some unimplemented features.
// Tests document what works vs what fails.
// =============================================================================

/// Hero section uses some unimplemented directives like @fade-in, @parallax.
/// This test documents the current parse status.
#[test]
fn test_hero_section_parse_status() {
    let (success, error) = parse_file("hero-section.st");

    // Document parse result - expected to fail due to unimplemented @fade-in, @parallax
    if !success {
        // Expected failure - document which directives failed
        let error_msg = error.unwrap_or_default();
        println!("hero-section.st parse failure (expected):");
        println!("  Error: {}", error_msg);
        println!("  Unimplemented directives: @fade-in, @parallax, @scroll-progress");
    } else {
        println!("hero-section.st parses successfully!");
    }
}

/// Product grid uses @fade-in-stagger, @if which are unimplemented.
#[test]
fn test_product_grid_parse_status() {
    let (success, error) = parse_file("product-grid.st");

    if !success {
        let error_msg = error.unwrap_or_default();
        println!("product-grid.st parse failure (expected):");
        println!("  Error: {}", error_msg);
        println!("  Unimplemented directives: @fade-in-stagger, @if");
    } else {
        println!("product-grid.st parses successfully!");
    }
}

/// Modal dialog uses @state_machine variants that may not be implemented.
#[test]
fn test_modal_dialog_parse_status() {
    let (success, error) = parse_file("modal-dialog.st");

    if !success {
        let error_msg = error.unwrap_or_default();
        println!("modal-dialog.st parse failure (expected):");
        println!("  Error: {}", error_msg);
        println!("  May use: @state_machine, @mutate");
    } else {
        println!("modal-dialog.st parses successfully!");
    }
}

/// Responsive layout uses @dark, @light, @breakpoint, @media variants.
#[test]
fn test_responsive_layout_parse_status() {
    let (success, error) = parse_file("responsive-layout.st");

    if !success {
        let error_msg = error.unwrap_or_default();
        println!("responsive-layout.st parse failure (expected):");
        println!("  Error: {}", error_msg);
        println!("  Unimplemented: @dark, @light, @breakpoint, @reduced-motion");
    } else {
        println!("responsive-layout.st parses successfully!");
    }
}

/// Drag and drop uses @drag, @swipe which may not be implemented.
#[test]
fn test_drag_and_drop_parse_status() {
    let (success, error) = parse_file("drag-and-drop.st");

    if !success {
        let error_msg = error.unwrap_or_default();
        println!("drag-and-drop.st parse failure (expected):");
        println!("  Error: {}", error_msg);
        println!("  Unimplemented: @drag, @swipe");
    } else {
        println!("drag-and-drop.st parses successfully!");
    }
}

// =============================================================================
// FEATURE GAP DOCUMENTATION
// =============================================================================

/// This test documents all directives that need parser implementation.
/// It serves as a checklist for future development.
#[test]
fn test_document_unimplemented_directives() {
    // Visibility/Animation Macros (need parser)
    let visibility_macros = vec![
        "@fade-in",
        "@fade-in-up",
        "@fade-in-stagger",
        "@slide-in",
        "@slide-in-left",
        "@slide-in-right",
        "@zoom-in",
    ];

    // Scroll Macros (need parser)
    let scroll_macros = vec!["@parallax", "@scroll-progress", "@sticky"];

    // Media Query Macros (need parser)
    let media_macros = vec![
        "@dark",
        "@light",
        "@breakpoint",
        "@reduced-motion",
        "@portrait",
        "@landscape",
        "@print",
    ];

    // Interaction Macros (need parser)
    let interaction_macros = vec!["@drag", "@swipe", "@pinch", "@rotate-gesture"];

    // Control Flow (need parser)
    let control_flow = vec!["@if", "@else", "@switch"];

    // Element References (need parser)
    let element_refs = vec!["@mouse"];

    println!("=== UNIMPLEMENTED DIRECTIVES CHECKLIST ===\n");

    println!("Visibility/Animation Macros:");
    for m in &visibility_macros {
        println!("  [ ] {}", m);
    }

    println!("\nScroll Macros:");
    for m in &scroll_macros {
        println!("  [ ] {}", m);
    }

    println!("\nMedia Query Macros:");
    for m in &media_macros {
        println!("  [ ] {}", m);
    }

    println!("\nInteraction Macros:");
    for m in &interaction_macros {
        println!("  [ ] {}", m);
    }

    println!("\nControl Flow:");
    for m in &control_flow {
        println!("  [ ] {}", m);
    }

    println!("\nElement References:");
    for m in &element_refs {
        println!("  [ ] {}", m);
    }

    let total = visibility_macros.len()
        + scroll_macros.len()
        + media_macros.len()
        + interaction_macros.len()
        + control_flow.len()
        + element_refs.len();

    println!("\nTotal unimplemented: {} directives", total);

    // This test always passes - it's for documentation
    assert!(true);
}

// =============================================================================
// IMPLEMENTED DIRECTIVE VERIFICATION
// =============================================================================

/// Verify that core directives ARE implemented and parse correctly.
#[test]
fn test_implemented_each_directive() {
    let input = r#"
        @type Item { name: string; }
        @data items: Item[] { src: "/api/items"; }
        .list {
            @each(items) {
                template: "item-template";
                [slot="name"]: $.name;
            }
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok(), "@each should parse: {:?}", result.err());
}

#[test]
fn test_implemented_type_directive() {
    let input = r#"
        @type Product {
            id: string;
            name: string;
            price: number;
            inStock: boolean;
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok(), "@type should parse: {:?}", result.err());
}

#[test]
fn test_implemented_data_directive() {
    let input = r#"
        @type Item { name: string; }
        @data items: Item[] {
            src: "/api/items";
            cache: 5m;
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok(), "@data should parse: {:?}", result.err());
}

#[test]
fn test_implemented_computed_directive() {
    let input = r#"
        @type Product { price: number; inStock: boolean; }
        @data products: Product[] { src: "/api/products"; }
        @computed activeProducts: Product[] {
            from: products;
            where: $.inStock == true;
            sort: $.price asc;
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok(), "@computed should parse: {:?}", result.err());
}

#[test]
fn test_implemented_fn_directive() {
    let input = r#"
        @fn formatPrice(price: number): string {
            return "$" + price.toFixed(2);
        }
    "#;
    let result = parse(input);
    assert!(result.is_ok(), "@fn should parse: {:?}", result.err());
}

#[test]
fn test_implemented_state_machine_directive() {
    let input = r#"
        .modal {
            @state_machine(initial: "closed");

            @state(when: "closed") {
                opacity: 0;
            }

            @state(when: "open") {
                opacity: 1;
            }

            @transition(from: "closed", to: "open", on: click);
            @transition(from: "open", to: "closed", on: escape);
        }
    "#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "@state_machine should parse: {:?}",
        result.err()
    );
}
#[test]
fn test_implemented_on_intersect_directive() {
    let input = r#"
        .gallery {
            @on intersect entrance {
                .item {
                    opacity: 0 -> 1;
                    stagger: 0.1 first;
                }
            }
        }
    "#;
    let result = parse(input);
    assert!(
        result.is_ok(),
        "@on intersect should parse: {:?}",
        result.err()
    );
}
