//! Semantic tokens provider for syntax highlighting.
//!
//! Scans document content to emit LSP semantic tokens for syntax highlighting.

use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensResult,
    SemanticTokensServerCapabilities,
};

use crate::parser::SourceSpan;

use super::document::DocumentState;

// =============================================================================
// Token Type Registry
// =============================================================================

/// Standard semantic token types we support.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,  // 0: @import, @scroll, etc.
    SemanticTokenType::MACRO,    // 1: %primitive, %emit, etc.
    SemanticTokenType::VARIABLE, // 2: $variable, &element
    SemanticTokenType::PROPERTY, // 3: CSS property names
    SemanticTokenType::STRING,   // 4: "strings"
    SemanticTokenType::NUMBER,   // 5: numbers, durations, colors
    SemanticTokenType::COMMENT,  // 6: // comments
    SemanticTokenType::TYPE,     // 7: type names, selectors
    SemanticTokenType::FUNCTION, // 8: function calls
    SemanticTokenType::OPERATOR, // 9: ->, <-, etc.
];

/// Semantic token modifiers we support.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION, // 0: definitions
    SemanticTokenModifier::DEFINITION,  // 1: definitions
];

/// Get the server capability for semantic tokens.
pub fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        legend: SemanticTokensLegend {
            token_types: TOKEN_TYPES.to_vec(),
            token_modifiers: TOKEN_MODIFIERS.to_vec(),
        },
        full: Some(SemanticTokensFullOptions::Bool(true)),
        range: None,
        ..Default::default()
    })
}

// =============================================================================
// Token Builder
// =============================================================================

/// Builder for constructing semantic tokens with delta encoding.
struct SemanticTokensBuilder {
    tokens: Vec<SemanticToken>,
    prev_line: u32,
    prev_start: u32,
}

impl SemanticTokensBuilder {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            prev_line: 0,
            prev_start: 0,
        }
    }

    /// Push a token with absolute line/character positions.
    fn push(&mut self, line: u32, start: u32, length: u32, token_type: u32, modifiers: u32) {
        let delta_line = line - self.prev_line;
        let delta_start = if delta_line == 0 {
            start - self.prev_start
        } else {
            start
        };

        self.tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: modifiers,
        });

        self.prev_line = line;
        self.prev_start = start;
    }

    fn build(self) -> SemanticTokens {
        SemanticTokens {
            result_id: None,
            data: self.tokens,
        }
    }
}

// =============================================================================
// Token Type Indices
// =============================================================================

const TT_KEYWORD: u32 = 0;
const TT_MACRO: u32 = 1;
const TT_VARIABLE: u32 = 2;
const TT_PROPERTY: u32 = 3;
const TT_STRING: u32 = 4;
const TT_NUMBER: u32 = 5;
const TT_COMMENT: u32 = 6;
const TT_TYPE: u32 = 7;
#[allow(dead_code)]
const TT_FUNCTION: u32 = 8;
const TT_OPERATOR: u32 = 9;

// =============================================================================
// Main Provider
// =============================================================================

/// Provide semantic tokens for a document.
pub fn provide_semantic_tokens(doc: &DocumentState) -> Option<SemanticTokensResult> {
    let content = &doc.content;
    let mapper = &doc.position_mapper;

    let mut builder = SemanticTokensBuilder::new();

    // Collect all tokens via text scanning
    let mut tokens = Vec::new();
    collect_tokens_from_text(content, &mut tokens);

    // Sort tokens by position (start offset)
    tokens.sort_by_key(|(span, _, _)| span.start);

    // Convert to LSP tokens
    for (span, token_type, modifiers) in tokens {
        let start_pos = mapper.position_from_offset(span.start);
        let end_pos = mapper.position_from_offset(span.end);

        // Only emit single-line tokens
        if start_pos.line == end_pos.line {
            let length = end_pos.character - start_pos.character;
            if length > 0 {
                builder.push(
                    start_pos.line,
                    start_pos.character,
                    length,
                    token_type,
                    modifiers,
                );
            }
        }
    }

    Some(SemanticTokensResult::Tokens(builder.build()))
}

// =============================================================================
// Token Collection (Text-Based)
// =============================================================================

type TokenList = Vec<(SourceSpan, u32, u32)>; // (span, token_type, modifiers)

/// The end offsets (exclusive) of each line within `text` — used to split a
/// multi-line block comment into per-line semantic tokens, which the LSP
/// protocol requires (gh-33).
fn comment_line_ends(text: &str) -> Vec<usize> {
    let mut ends = Vec::new();
    for (idx, b) in text.bytes().enumerate() {
        if b == b'\n' {
            ends.push(idx + 1);
        }
    }
    if ends.is_empty() || ends.last() != Some(&text.len()) {
        ends.push(text.len());
    }
    ends
}

/// Scan content for tokens using text patterns.
fn collect_tokens_from_text(content: &str, tokens: &mut TokenList) {
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Line comments: //
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            tokens.push((SourceSpan::new(start, i), TT_COMMENT, 0));
            continue;
        }

        // Block comments: /* */
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            let end = if i + 1 < bytes.len() { i + 2 } else { i };
            // gh-33: the LSP protocol requires each semantic token to live on a
            // single line. A block comment spanning N lines is split into N
            // per-line COMMENT tokens (leading/trailing whitespace trimmed so a
            // `/*` on its own line is not an empty token, which the builder
            // drops as length 0).
            let mut line_start = start;
            for line_end in comment_line_ends(&content[..end]) {
                let ls = line_start;
                let le = line_end;
                // Trim the token to its visible text.
                let mut a = ls;
                let mut b = le;
                while a < b && bytes[a].is_ascii_whitespace() {
                    a += 1;
                }
                while b > a && bytes[b - 1].is_ascii_whitespace() {
                    b -= 1;
                }
                if b > a {
                    tokens.push((SourceSpan::new(a, b), TT_COMMENT, 0));
                }
                line_start = le;
            }
            i = end;
            continue;
        }

        // @ directive keywords
        if bytes[i] == b'@' {
            let start = i;
            i += 1;
            // Handle optional dot modifier like @test.skip
            while i < bytes.len() && (is_ident_char(bytes[i]) || bytes[i] == b'.') {
                i += 1;
            }
            if i > start + 1 {
                tokens.push((SourceSpan::new(start, i), TT_KEYWORD, 0));
            }
            continue;
        }

        // % metasystem keywords
        if bytes[i] == b'%' {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            if i > start + 1 {
                tokens.push((SourceSpan::new(start, i), TT_MACRO, 0));
            }
            continue;
        }

        // $ variable references
        if bytes[i] == b'$' {
            let start = i;
            i += 1;
            while i < bytes.len() && (is_ident_char(bytes[i]) || bytes[i] == b'.') {
                i += 1;
            }
            if i > start + 1 {
                tokens.push((SourceSpan::new(start, i), TT_VARIABLE, 0));
            }
            continue;
        }

        // & element/template references
        if bytes[i] == b'&' {
            let start = i;
            i += 1;
            while i < bytes.len() && (is_ident_char(bytes[i]) || bytes[i] == b'.') {
                i += 1;
            }
            if i > start + 1 {
                tokens.push((SourceSpan::new(start, i), TT_VARIABLE, 0));
            }
            continue;
        }

        // Strings: "..."
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < bytes.len() {
                i += 1;
            }
            tokens.push((SourceSpan::new(start, i), TT_STRING, 0));
            continue;
        }

        // Colors: #fff, #ffffff, #rrggbbaa
        if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_hexdigit() {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            tokens.push((SourceSpan::new(start, i), TT_NUMBER, 0));
            continue;
        }

        // -> arrow operator (must be before negative number check)
        if bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
            tokens.push((SourceSpan::new(i, i + 2), TT_OPERATOR, 0));
            i += 2;
            continue;
        }

        // <- mutation operator
        if bytes[i] == b'<' && i + 1 < bytes.len() && bytes[i + 1] == b'-' {
            tokens.push((SourceSpan::new(i, i + 2), TT_OPERATOR, 0));
            i += 2;
            continue;
        }

        // Numbers with optional units: 100px, 2.5s, 50%, -10deg
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'-' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
            || (bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
        {
            let start = i;
            if bytes[i] == b'-' {
                i += 1;
            }
            // Integer part
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Decimal part
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Unit suffix (px, em, rem, s, ms, deg, etc.)
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            // Percent
            if i < bytes.len() && bytes[i] == b'%' {
                i += 1;
            }
            if i > start {
                tokens.push((SourceSpan::new(start, i), TT_NUMBER, 0));
            }
            continue;
        }

        // Selectors starting with . or #
        if (bytes[i] == b'.' || bytes[i] == b'#')
            && i + 1 < bytes.len()
            && is_ident_start(bytes[i + 1])
        {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            tokens.push((SourceSpan::new(start, i), TT_TYPE, 0));
            continue;
        }

        // [attr] selectors
        if bytes[i] == b'[' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b']' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // include ]
                tokens.push((SourceSpan::new(start, i), TT_TYPE, 0));
            }
            continue;
        }

        // :pseudo selectors (leading : followed by ident, NOT property colons)
        if bytes[i] == b':' && i + 1 < bytes.len() && is_ident_start(bytes[i + 1]) {
            let start = i;
            i += 1; // skip :
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            tokens.push((SourceSpan::new(start, i), TT_TYPE, 0));
            continue;
        }

        // Property names (identifier followed by :)
        if is_ident_start(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            // Skip whitespace
            let mut j = i;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            // Check if followed by :
            if j < bytes.len() && bytes[j] == b':' && (j + 1 >= bytes.len() || bytes[j + 1] != b':')
            {
                tokens.push((SourceSpan::new(start, i), TT_PROPERTY, 0));
            }
            continue;
        }

        // Skip other characters
        i += 1;
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_directive_keywords() {
        let content = "@scroll test { @on &.hover { } }";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let keyword_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_KEYWORD)
            .collect();
        assert_eq!(keyword_tokens.len(), 2);
    }

    #[test]
    fn test_collect_meta_keywords() {
        let content = "%primitive test { %emit js { } }";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let macro_tokens: Vec<_> = tokens.iter().filter(|(_, tt, _)| *tt == TT_MACRO).collect();
        assert_eq!(macro_tokens.len(), 2);
    }

    #[test]
    fn test_collect_variables() {
        let content = "$myVar and &element";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let var_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_VARIABLE)
            .collect();
        assert_eq!(var_tokens.len(), 2);
    }

    #[test]
    fn test_collect_strings() {
        let content = r#""hello world" and "another""#;
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let string_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_STRING)
            .collect();
        assert_eq!(string_tokens.len(), 2);
    }

    #[test]
    fn test_collect_numbers() {
        let content = "100px 2.5s 50% -10deg";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let num_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_NUMBER)
            .collect();
        assert_eq!(num_tokens.len(), 4);
    }

    #[test]
    fn test_collect_colors() {
        let content = "#fff #ff00aa";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let color_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_NUMBER)
            .collect();
        assert_eq!(color_tokens.len(), 2);
    }

    #[test]
    fn test_collect_comments() {
        let content = "code // this is a comment\nmore code";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let comment_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_COMMENT)
            .collect();
        assert_eq!(comment_tokens.len(), 1);
    }

    #[test]
    fn test_collect_block_comments() {
        let content = "code /* block\ncomment */ more";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let comment_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_COMMENT)
            .collect();
        // gh-33: the LSP requires per-line tokens — a two-line block comment
        // yields TWO comment tokens, not one spanning both lines.
        assert_eq!(
            comment_tokens.len(),
            2,
            "multi-line block comment must be split per line, got {comment_tokens:?}"
        );
        assert_eq!(comment_tokens[0].0, SourceSpan::new(5, 13)); // "/* block"
        assert_eq!(comment_tokens[1].0, SourceSpan::new(14, 24)); // "comment */"
    }

    /// gh-33: a `#hex` inside a comment is prose, not a colour token.
    #[test]
    fn test_color_like_text_in_comment_is_not_a_color() {
        let content = "a { /* the #abc color */ color: #123; }";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        // The real CSS `#123` is a colour (TT_NUMBER); the `#abc` in the
        // comment is comment text. Collect the colour spans and assert only
        // the real one.
        let color_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_NUMBER)
            .map(|(s, _, _)| s.clone())
            .collect();
        assert_eq!(
            color_tokens,
            vec![SourceSpan::new(32, 36)],
            "only the real CSS hex should be a colour token, got {color_tokens:?}"
        );
    }

    #[test]
    fn test_collect_selectors() {
        let content = ".hero-section #main";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let type_tokens: Vec<_> = tokens.iter().filter(|(_, tt, _)| *tt == TT_TYPE).collect();
        assert_eq!(type_tokens.len(), 2);
    }

    #[test]
    fn test_collect_properties() {
        let content = "opacity: 1; transform: rotate(45deg);";
        let mut tokens = Vec::new();
        collect_tokens_from_text(content, &mut tokens);

        let prop_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, tt, _)| *tt == TT_PROPERTY)
            .collect();
        assert_eq!(prop_tokens.len(), 2);
    }
}
