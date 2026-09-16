//! Rowan-based Concrete Syntax Tree for Spacetime
//!
//! This module provides a lossless, error-tolerant syntax tree representation.
//! SyntaxKind defines structural grammar; semantic interpretation comes from
//! %form pattern matching in the stdlib.

mod ast;
pub(crate) mod lexer;
mod parser;

pub use ast::*;
pub use lexer::Lexer;
pub use parser::parse;
pub use parser::scan_html_end;

use rowan::Language;

/// Language definition for Spacetime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpacetimeLang {}

impl Language for SpacetimeLang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind.into())
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SpacetimeLang>;
pub type SyntaxToken = rowan::SyntaxToken<SpacetimeLang>;
pub type SyntaxElement = rowan::SyntaxElement<SpacetimeLang>;
pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<SpacetimeLang>;

/// Syntax kinds for Spacetime's structural grammar
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // === Tokens ===
    /// Whitespace (spaces, tabs, newlines)
    WHITESPACE = 0,
    /// Line comment (// ...) or block comment (/* ... */)
    COMMENT,
    /// Identifier (letters, digits, underscore, hyphen)
    IDENT,
    /// String literal ("..." or '...')
    STRING,
    /// Number literal (123, 3.14, -42)
    NUMBER,
    /// Number with unit (500ms, 2s, 100px, 50%)
    NUMBER_WITH_UNIT,
    /// Color literal (#fff, #aabbcc, #rrggbbaa)
    COLOR,

    // Single-char prefix tokens
    /// $ (variable prefix)
    DOLLAR,
    /// & (element reference prefix)
    AMPERSAND,
    /// ~ (preset reference prefix)
    TILDE,
    /// @ (directive prefix)
    AT_SIGN,
    /// % (meta definition prefix)
    PERCENT,
    /// ` (hole prefix — THE hole form everywhere, per AGENTS.md)
    ///
    /// One char, NON-CONSUMING, exactly like its sibling sigils above. The
    /// backtick does not decide how far a hole reaches; the parser does, from
    /// grammar. A span-consuming backtick token was tried (FUP-046 attempt 1)
    /// and swallowed all 4,934 holes in the corpus.
    BACKTICK,
    /// # (id selector or color prefix - context dependent)
    HASH,

    // Punctuation
    /// :
    COLON,
    /// ;
    SEMICOLON,
    /// ,
    COMMA,
    /// .
    DOT,
    /// ?
    QUESTION,
    /// ??
    QUESTION_QUESTION,
    /// !
    EXCLAIM,
    /// =
    EQUALS,
    /// +
    PLUS,
    /// -
    MINUS,
    /// *
    STAR,
    /// /
    SLASH,
    /// |
    PIPE,
    /// ^
    CARET,
    /// <
    LT,
    /// >
    GT,
    /// ->
    ARROW,
    /// <-
    LEFT_ARROW,
    /// =>
    FAT_ARROW,
    /// &&
    AND_AND,
    /// ||
    OR_OR,
    /// ==
    EQ_EQ,
    /// ===
    EQ_EQ_EQ,
    /// !=
    NOT_EQ,
    /// !==
    NOT_EQ_EQ,
    /// <=
    LT_EQ,
    /// >=
    GT_EQ,

    // Delimiters
    /// (
    L_PAREN,
    /// )
    R_PAREN,
    /// {
    L_BRACE,
    /// }
    R_BRACE,
    /// [
    L_BRACKET,
    /// ]
    R_BRACKET,

    // Keywords (contextual)
    /// as
    KW_AS,
    /// if
    KW_IF,
    /// else
    KW_ELSE,
    /// true
    KW_TRUE,
    /// false
    KW_FALSE,
    /// in
    KW_IN,
    /// with
    KW_WITH,
    /// on
    KW_ON,
    /// from
    KW_FROM,
    /// to
    KW_TO,

    // === Composite Nodes ===
    /// File root
    ROOT,
    /// Error recovery node
    ERROR,

    // References with prefix
    /// $name - variable reference
    VARIABLE_REF,
    /// &name - element reference
    ELEMENT_REF,
    /// &name selector; or &name selector { body } - element reference statement
    ELEMENT_REF_STMT,
    /// --name; or --name(args); - form splice (SIP-001c, BUG-241). A
    /// statement-position dashed ident NOT followed by `:` (which would be a
    /// CSS custom property). Validated against the form registry after rematch;
    /// expansion (splicing the declared body) is W3 scope.
    FORM_REF,
    /// @name(args) { body } - directive
    DIRECTIVE,
    /// %macro { ... } - meta definition
    META_DEF,
    /// `%uses a, b` prelude-dependency clause in a %primitive header (PLAN-133)
    META_USES,

    // Structure
    /// (arg1, arg2, name: value)
    ARG_LIST,
    /// Single argument
    ARG,
    /// name: value
    NAMED_ARG,
    /// { ... }
    BODY,
    /// .selector { ... }
    SCOPE_BLOCK,
    /// .class, #id, [attr], etc.
    SELECTOR,
    /// Selector part (single selector in a comma-separated list)
    SELECTOR_PART,

    // Pattern/capture related
    /// :capture_type
    CAPTURE,
    /// The type identifier in a capture
    CAPTURE_TYPE,

    // Expressions
    /// General expression
    EXPR,
    /// a + b, a - b, etc.
    BINARY_EXPR,
    /// !a, -a
    UNARY_EXPR,
    /// fn(args)
    CALL_EXPR,
    /// a[b]
    INDEX_EXPR,
    /// a.b
    MEMBER_EXPR,
    /// (expr)
    PAREN_EXPR,
    /// [a, b, c]
    ARRAY_EXPR,
    /// { key: value, ... }
    OBJECT_EXPR,
    /// key: value in an object
    OBJECT_FIELD,
    /// Ternary: cond ? a : b
    TERNARY_EXPR,

    // CSS-related
    /// property: value;
    CSS_PROPERTY,
    /// property value(s)
    CSS_VALUE,
    /// Transition value: from -> to
    TRANSITION_VALUE,
    /// Keyframe block: { 0%: v; 50%: v; 100%: v; }
    KEYFRAME_BLOCK,
    /// Single keyframe: 50%: value
    KEYFRAME,

    // HTML (first-class markup; see PLAN-023)
    /// <tag ...>...</tag> - an HTML element literal (raw span + extracted holes)
    HTML_ELEMENT,
    /// `expr` - a Spacetime expression embedded in HTML (text or attr-value position)
    HTML_HOLE,
    /// Raw HTML source text between holes (token; carries verbatim markup for html5ever)
    HTML_RAW,

    // Special
    /// Marker during parsing
    TOMBSTONE,
    /// End of file
    EOF,

    // Sentinel value - must be last
    #[doc(hidden)]
    __LAST,
}

impl From<u16> for SyntaxKind {
    fn from(raw: u16) -> Self {
        // Safety: We check bounds
        if raw < SyntaxKind::__LAST as u16 {
            // SAFETY: All values from 0 to __LAST - 1 are valid SyntaxKind variants
            unsafe { std::mem::transmute(raw) }
        } else {
            SyntaxKind::ERROR
        }
    }
}

impl From<SyntaxKind> for u16 {
    fn from(kind: SyntaxKind) -> Self {
        kind as u16
    }
}

impl SyntaxKind {
    /// Returns true if this kind represents trivia (whitespace or comments)
    pub fn is_trivia(self) -> bool {
        matches!(self, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT)
    }

    /// Returns true if this kind is a keyword
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            SyntaxKind::KW_AS
                | SyntaxKind::KW_IF
                | SyntaxKind::KW_ELSE
                | SyntaxKind::KW_TRUE
                | SyntaxKind::KW_FALSE
                | SyntaxKind::KW_IN
                | SyntaxKind::KW_WITH
                | SyntaxKind::KW_ON
                | SyntaxKind::KW_FROM
                | SyntaxKind::KW_TO
        )
    }

    /// Returns true if this kind can start a selector part
    pub fn is_selector_start(self) -> bool {
        matches!(
            self,
            SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::L_BRACKET
                | SyntaxKind::COLON
                | SyntaxKind::IDENT
                | SyntaxKind::STAR
                | SyntaxKind::AMPERSAND
        )
    }

    /// Returns true if this kind is a prefix character ($, &, ~, @, %, #)
    pub fn is_prefix(self) -> bool {
        matches!(
            self,
            SyntaxKind::DOLLAR
                | SyntaxKind::AMPERSAND
                | SyntaxKind::TILDE
                | SyntaxKind::AT_SIGN
                | SyntaxKind::PERCENT
                | SyntaxKind::HASH
        )
    }

    /// Returns true if this kind is a delimiter
    pub fn is_delimiter(self) -> bool {
        matches!(
            self,
            SyntaxKind::L_PAREN
                | SyntaxKind::R_PAREN
                | SyntaxKind::L_BRACE
                | SyntaxKind::R_BRACE
                | SyntaxKind::L_BRACKET
                | SyntaxKind::R_BRACKET
        )
    }

    /// Returns true if this kind is a comparison operator
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            SyntaxKind::EQ_EQ
                | SyntaxKind::EQ_EQ_EQ
                | SyntaxKind::NOT_EQ
                | SyntaxKind::NOT_EQ_EQ
                | SyntaxKind::LT
                | SyntaxKind::GT
                | SyntaxKind::LT_EQ
                | SyntaxKind::GT_EQ
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_kind_round_trip() {
        for kind in [
            SyntaxKind::WHITESPACE,
            SyntaxKind::IDENT,
            SyntaxKind::ROOT,
            SyntaxKind::EOF,
        ] {
            let raw: u16 = kind.into();
            let back: SyntaxKind = raw.into();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn test_trivia_detection() {
        assert!(SyntaxKind::WHITESPACE.is_trivia());
        assert!(SyntaxKind::COMMENT.is_trivia());
        assert!(!SyntaxKind::IDENT.is_trivia());
    }

    #[test]
    fn test_keyword_detection() {
        assert!(SyntaxKind::KW_AS.is_keyword());
        assert!(SyntaxKind::KW_IF.is_keyword());
        assert!(SyntaxKind::KW_TRUE.is_keyword());
        assert!(!SyntaxKind::IDENT.is_keyword());
    }

    #[test]
    fn test_prefix_detection() {
        assert!(SyntaxKind::DOLLAR.is_prefix());
        assert!(SyntaxKind::AMPERSAND.is_prefix());
        assert!(SyntaxKind::TILDE.is_prefix());
        assert!(SyntaxKind::AT_SIGN.is_prefix());
        assert!(SyntaxKind::PERCENT.is_prefix());
        assert!(SyntaxKind::HASH.is_prefix());
        assert!(!SyntaxKind::IDENT.is_prefix());
    }

    #[test]
    fn test_out_of_bounds_kind() {
        let invalid: SyntaxKind = 9999u16.into();
        assert_eq!(invalid, SyntaxKind::ERROR);
    }
}
