//! Integration tests for the InspectElement roundtrip.
//!
//! These tests verify the full pipeline: parse .st files → handle InspectElement
//! → return ElementContext with correct properties, source locations, and value
//! classifications.

use spacetime::sync::handlers::handle_inspect_element;
use spacetime::sync::protocol::{ServerMessage, ValueType};
use std::path::Path;
use tempfile::TempDir;

/// Helper to write a .st file into a temp directory.
fn write_st_file(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
}

/// Extract an ElementContext from a ServerMessage, panicking if it's the wrong variant.
fn unwrap_element_context(
    msg: ServerMessage,
) -> (
    String,
    Vec<String>,
    Vec<spacetime::sync::protocol::PropertySection>,
    Option<spacetime::sync::protocol::StateMachineContext>,
    Vec<spacetime::sync::protocol::DataBindingContext>,
    bool,
) {
    match msg {
        ServerMessage::ElementContext {
            selector,
            source_files,
            sections,
            state_machine,
            data_bindings,
            not_found,
        } => (
            selector,
            source_files,
            sections,
            state_machine,
            data_bindings,
            not_found,
        ),
        other => panic!("Expected ElementContext, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Test 1: Roundtrip with scroll animations
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_roundtrip_with_animations() {
    let dir = TempDir::new().unwrap();
    write_st_file(
        dir.path(),
        "styles.st",
        r#".hero {
    @scroll reveal(&fade-up) {
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
        duration: 800ms;
        easing: &ease-out;
    }
}
"#,
    );

    let result = handle_inspect_element(dir.path(), ".hero", None);
    let (selector, source_files, sections, _state_machine, _data_bindings, not_found) =
        unwrap_element_context(result);

    assert_eq!(selector, ".hero");
    assert!(!not_found, "Element should be found");
    assert!(
        source_files.contains(&"styles.st".to_string()),
        "source_files should contain styles.st, got: {:?}",
        source_files
    );

    // Should have at least one section with animation-related properties
    assert!(
        !sections.is_empty(),
        "Expected at least one section for .hero with @scroll"
    );

    // Look for a "Scroll Animations" section (from section_label_for_macro)
    let scroll_section = sections.iter().find(|s| s.label == "Scroll Animations");
    assert!(
        scroll_section.is_some(),
        "Expected 'Scroll Animations' section, got labels: {:?}",
        sections.iter().map(|s| &s.label).collect::<Vec<_>>()
    );

    let scroll = scroll_section.unwrap();
    assert!(
        !scroll.properties.is_empty(),
        "Scroll Animations section should have properties"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Multi-file merge
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_multi_file_merge() {
    let dir = TempDir::new().unwrap();
    write_st_file(dir.path(), "main.st", ".hero {\n    color: #ff0000;\n}\n");
    write_st_file(dir.path(), "theme.st", ".hero {\n    padding: 20px;\n}\n");

    let result = handle_inspect_element(dir.path(), ".hero", None);
    let (_selector, source_files, sections, _sm, _db, not_found) = unwrap_element_context(result);

    assert!(!not_found);
    assert!(
        source_files.len() >= 2,
        "Expected source_files from both main.st and theme.st, got: {:?}",
        source_files
    );

    // Verify CSS sections from both files
    assert!(
        sections.len() >= 2,
        "Expected at least 2 CSS sections (one per file), got {}",
        sections.len()
    );

    // Collect all CSS property names across sections
    let all_prop_names: Vec<&str> = sections
        .iter()
        .filter(|s| s.label == "CSS")
        .flat_map(|s| s.properties.iter().map(|p| p.name.as_str()))
        .collect();

    assert!(
        all_prop_names.contains(&"color"),
        "Expected 'color' property from main.st, got: {:?}",
        all_prop_names
    );
    assert!(
        all_prop_names.contains(&"padding"),
        "Expected 'padding' property from theme.st, got: {:?}",
        all_prop_names
    );
}

// ---------------------------------------------------------------------------
// Test 3: Unknown selector graceful handling
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_unknown_selector_graceful() {
    let dir = TempDir::new().unwrap();
    write_st_file(dir.path(), "styles.st", ".hero {\n    color: red;\n}\n");

    let result = handle_inspect_element(dir.path(), ".nonexistent", None);
    let (_selector, source_files, sections, state_machine, data_bindings, not_found) =
        unwrap_element_context(result);

    assert!(not_found, "Unknown selector should set not_found = true");
    assert!(
        source_files.is_empty(),
        "Unknown selector should have empty source_files"
    );
    assert!(
        sections.is_empty(),
        "Unknown selector should have empty sections"
    );
    assert!(
        state_machine.is_none(),
        "Unknown selector should have no state machine"
    );
    assert!(
        data_bindings.is_empty(),
        "Unknown selector should have no data bindings"
    );
}

// ---------------------------------------------------------------------------
// Test 4: CSS source locations
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_css_source_locations() {
    let dir = TempDir::new().unwrap();
    // Carefully crafted so we know the line numbers:
    // Line 1: .box {
    // Line 2:     color: #ff0000;
    // Line 3:     margin: 10px;
    // Line 4: }
    write_st_file(
        dir.path(),
        "layout.st",
        ".box {\n    color: #ff0000;\n    margin: 10px;\n}\n",
    );

    let result = handle_inspect_element(dir.path(), ".box", None);
    let (_selector, _source_files, sections, _sm, _db, not_found) = unwrap_element_context(result);

    assert!(!not_found);

    let css_section = sections
        .iter()
        .find(|s| s.label == "CSS")
        .expect("Expected CSS section");

    // Verify source_file is populated on each property
    for prop in &css_section.properties {
        assert!(
            prop.source_file.is_some(),
            "Property '{}' should have source_file set",
            prop.name
        );
        assert!(
            prop.source_file.as_ref().unwrap().contains("layout.st"),
            "Property '{}' source_file should reference layout.st",
            prop.name
        );
    }

    // Verify source_line is populated and reasonable (> 0)
    for prop in &css_section.properties {
        assert!(
            prop.source_line.is_some(),
            "Property '{}' should have source_line set",
            prop.name
        );
        let line = prop.source_line.unwrap();
        assert!(
            line > 0,
            "Property '{}' source_line should be > 0, got {}",
            prop.name,
            line
        );
    }

    // Color is on line 2, margin on line 3
    let color_prop = css_section
        .properties
        .iter()
        .find(|p| p.name == "color")
        .expect("color prop");
    let margin_prop = css_section
        .properties
        .iter()
        .find(|p| p.name == "margin")
        .expect("margin prop");

    assert_eq!(color_prop.source_line, Some(2), "color should be on line 2");
    assert_eq!(
        margin_prop.source_line,
        Some(3),
        "margin should be on line 3"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Value classification integration
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_value_classification_integration() {
    let dir = TempDir::new().unwrap();
    write_st_file(
        dir.path(),
        "styles.st",
        ".widget {\n    color: #ff0000;\n    transition-duration: 1200ms;\n    padding: 40px;\n    opacity: 0.5;\n}\n",
    );

    let result = handle_inspect_element(dir.path(), ".widget", None);
    let (_selector, _source_files, sections, _sm, _db, not_found) = unwrap_element_context(result);

    assert!(!not_found);

    let css = sections
        .iter()
        .find(|s| s.label == "CSS")
        .expect("CSS section");

    // Color → ValueType::Color
    let color = css
        .properties
        .iter()
        .find(|p| p.name == "color")
        .expect("color prop");
    assert!(
        matches!(color.value.parsed, ValueType::Color(_)),
        "Expected Color, got {:?}",
        color.value.parsed
    );

    // Duration → ValueType::Duration
    let dur = css
        .properties
        .iter()
        .find(|p| p.name == "transition-duration")
        .expect("transition-duration prop");
    assert!(
        matches!(dur.value.parsed, ValueType::Duration { ms } if (ms - 1200.0).abs() < f64::EPSILON),
        "Expected Duration {{ ms: 1200 }}, got {:?}",
        dur.value.parsed
    );

    // Dimension → ValueType::Dimension
    let pad = css
        .properties
        .iter()
        .find(|p| p.name == "padding")
        .expect("padding prop");
    assert!(
        matches!(pad.value.parsed, ValueType::Dimension { ref unit, .. } if unit == "px"),
        "Expected Dimension with px, got {:?}",
        pad.value.parsed
    );

    // Number → ValueType::Number
    let opacity = css
        .properties
        .iter()
        .find(|p| p.name == "opacity")
        .expect("opacity prop");
    assert!(
        matches!(opacity.value.parsed, ValueType::Number(n) if (n - 0.5).abs() < f64::EPSILON),
        "Expected Number(0.5), got {:?}",
        opacity.value.parsed
    );
}

// ---------------------------------------------------------------------------
// Test 6: file_hint restricts scope
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_file_hint_restricts_scope() {
    let dir = TempDir::new().unwrap();
    write_st_file(dir.path(), "main.st", ".hero {\n    color: #ff0000;\n}\n");
    write_st_file(dir.path(), "extras.st", ".hero {\n    padding: 20px;\n}\n");

    // With file_hint, only main.st should be scanned
    let result = handle_inspect_element(dir.path(), ".hero", Some("main.st"));
    let (_selector, source_files, sections, _sm, _db, not_found) = unwrap_element_context(result);

    assert!(!not_found);
    assert_eq!(
        source_files.len(),
        1,
        "file_hint should restrict to 1 file, got: {:?}",
        source_files
    );
    assert!(
        source_files[0].contains("main.st"),
        "source_files should contain main.st, got: {:?}",
        source_files
    );

    // Only CSS properties from main.st should appear
    let all_prop_names: Vec<&str> = sections
        .iter()
        .filter(|s| s.label == "CSS")
        .flat_map(|s| s.properties.iter().map(|p| p.name.as_str()))
        .collect();

    assert!(
        all_prop_names.contains(&"color"),
        "Should have 'color' from main.st"
    );
    assert!(
        !all_prop_names.contains(&"padding"),
        "Should NOT have 'padding' from extras.st when file_hint restricts to main.st"
    );
}

// ---------------------------------------------------------------------------
// Test 7: Empty directory returns not_found
// ---------------------------------------------------------------------------

#[test]
fn test_inspect_element_empty_directory() {
    let dir = TempDir::new().unwrap();
    // No .st files at all

    let result = handle_inspect_element(dir.path(), ".hero", None);
    let (_selector, source_files, sections, _sm, _db, not_found) = unwrap_element_context(result);

    assert!(not_found);
    assert!(source_files.is_empty());
    assert!(sections.is_empty());
}
