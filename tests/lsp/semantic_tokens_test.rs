//! Tests for LSP semantic tokens provider.

use spacetime::lsp::{DocumentState, provide_semantic_tokens};
use tower_lsp::lsp_types::SemanticTokensResult;

fn document_from(content: &str) -> DocumentState {
    DocumentState::new(content.to_string(), 1)
}

/// Decode delta-encoded semantic tokens into (line, char, length, token_type, modifiers).
fn decode_tokens(result: &SemanticTokensResult) -> Vec<(u32, u32, u32, u32, u32)> {
    let tokens = match result {
        SemanticTokensResult::Tokens(t) => &t.data,
        _ => return vec![],
    };

    let mut decoded = Vec::new();
    let mut line = 0u32;
    let mut col = 0u32;

    for token in tokens {
        if token.delta_line > 0 {
            line += token.delta_line;
            col = token.delta_start;
        } else {
            col += token.delta_start;
        }
        decoded.push((
            line,
            col,
            token.length,
            token.token_type,
            token.token_modifiers_bitset,
        ));
    }

    decoded
}

// Token type indices (from TOKEN_TYPES array in semantic_tokens.rs)
const TT_VARIABLE: u32 = 2;
const TT_PROPERTY: u32 = 3;
const TT_STRING: u32 = 4;
const TT_TYPE: u32 = 7;
const TT_OPERATOR: u32 = 9;

// =============================================================================
// Tilde is NOT a preset sigil anymore (SIP-001c / BUG-263)
// =============================================================================

#[test]
fn tilde_no_longer_emits_preset_variable_token() {
    // The `~` preset-ref sigil is retired; a `~name` in a value slot errors at
    // parse and must NOT be surfaced as a VARIABLE token here.
    let doc = document_from("easing: ~ease-out-expo;");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let variable_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_VARIABLE).collect();
    assert_eq!(
        variable_tokens.len(),
        0,
        "`~` must not emit a preset VARIABLE token, got {:?}",
        tokens
    );
}

#[test]
fn tilde_in_property_context() {
    let doc = document_from("easing: --ease-out-expo;");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let property_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_PROPERTY).collect();
    assert_eq!(
        property_tokens.len(),
        1,
        "expected 1 PROPERTY token, got {:?}",
        tokens
    );
}

// =============================================================================
// Arrow operator tokens
// =============================================================================

#[test]
fn arrow_operator_token() {
    let doc = document_from("opacity: 0 -> 1;");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let operator_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_OPERATOR).collect();
    assert_eq!(
        operator_tokens.len(),
        1,
        "expected 1 OPERATOR for ->, got {:?}",
        tokens
    );
    assert_eq!(operator_tokens[0].2, 2, "-> should have length 2");
}

#[test]
fn left_arrow_mutation_token() {
    let doc = document_from("$navTheme <- \"dark\";");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let variable_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_VARIABLE).collect();
    let operator_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_OPERATOR).collect();
    let string_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_STRING).collect();
    assert!(
        !variable_tokens.is_empty(),
        "expected VARIABLE for $navTheme, got {:?}",
        tokens
    );
    assert!(
        !operator_tokens.is_empty(),
        "expected OPERATOR for <-, got {:?}",
        tokens
    );
    assert!(
        !string_tokens.is_empty(),
        "expected STRING for \"dark\", got {:?}",
        tokens
    );
}

// =============================================================================
// Attribute and pseudo selector tokens
// =============================================================================

#[test]
fn attribute_selector_token() {
    let doc = document_from("[data-panel=\"crystal\"]");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let type_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_TYPE).collect();
    assert!(
        !type_tokens.is_empty(),
        "expected TYPE for [data-panel], got {:?}",
        tokens
    );
}

#[test]
fn pseudo_selector_token() {
    let doc = document_from(":first-child");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let type_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_TYPE).collect();
    assert_eq!(
        type_tokens.len(),
        1,
        "expected 1 TYPE for :first-child, got {:?}",
        tokens
    );
}

#[test]
fn pseudo_entering_exiting() {
    let doc = document_from(":entering { } :exiting { }");
    let result = provide_semantic_tokens(&doc).unwrap();
    let tokens = decode_tokens(&result);
    let type_tokens: Vec<_> = tokens.iter().filter(|t| t.3 == TT_TYPE).collect();
    assert_eq!(
        type_tokens.len(),
        2,
        "expected 2 TYPE tokens for :entering and :exiting, got {:?}",
        tokens
    );
}

// =============================================================================
// Integration
// =============================================================================

#[test]
fn provide_semantic_tokens_integration() {
    let content = r#"
.hero {
    @scroll(start: 0.15) {
        opacity: 0 -> 1;
        easing: --ease-out-expo;
    }
}
"#;
    let doc = document_from(content);
    let result = provide_semantic_tokens(&doc);
    assert!(result.is_some());
    let tokens = decode_tokens(&result.unwrap());
    assert!(
        !tokens.is_empty(),
        "integration snippet should produce tokens"
    );
}
