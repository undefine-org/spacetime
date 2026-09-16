//! Input representation for the event-based parser.
//!
//! Stores tokens as struct-of-arrays (kinds + byte offsets) without owning
//! the source text. The parser operates on SyntaxKind + position only.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::cst::lexer::Token;

/// Parser input: a struct-of-arrays representation of lexer tokens.
///
/// The parser never touches source text directly — only SyntaxKind and byte
/// offsets. Token `n` spans `source[starts[n]..starts[n+1]]`.
#[derive(Debug)]
pub struct Input {
    /// The SyntaxKind of each token (including trivia).
    kinds: Vec<SyntaxKind>,
    /// Byte offset where each token starts in the source.
    starts: Vec<u32>,
    /// Whether a newline appears in the trivia immediately preceding this token
    /// (i.e. this token starts on a new line relative to the previous
    /// non-trivia token). Used to stop greedy inline-arg consumption at a
    /// statement boundary (e.g. `@import "x"` then `\n.box {}`).
    newline_before: Vec<bool>,
}

impl Input {
    /// Construct Input from existing lexer tokens.
    pub fn from_tokens(tokens: &[Token]) -> Self {
        let mut kinds = Vec::with_capacity(tokens.len());
        let mut starts = Vec::with_capacity(tokens.len());
        let mut newline_before = Vec::with_capacity(tokens.len());

        // Track whether a newline occurred in the trivia run preceding each token.
        let mut pending_newline = false;
        for token in tokens {
            if token.kind.is_trivia() {
                if token.text.contains('\n') {
                    pending_newline = true;
                }
                kinds.push(token.kind);
                starts.push(token.offset as u32);
                newline_before.push(false);
            } else {
                kinds.push(token.kind);
                starts.push(token.offset as u32);
                newline_before.push(pending_newline);
                pending_newline = false;
            }
        }

        Self {
            kinds,
            starts,
            newline_before,
        }
    }

    /// Number of tokens (including trivia and EOF).
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    /// Whether the input is empty.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    /// Get the kind of token at position `pos`.
    pub fn kind(&self, pos: usize) -> SyntaxKind {
        self.kinds.get(pos).copied().unwrap_or(SyntaxKind::EOF)
    }

    /// Get the byte offset where token `pos` starts.
    pub fn start(&self, pos: usize) -> u32 {
        self.starts.get(pos).copied().unwrap_or(0)
    }

    /// Whether a newline appears in the trivia immediately before token `pos`.
    pub fn newline_before(&self, pos: usize) -> bool {
        self.newline_before.get(pos).copied().unwrap_or(false)
    }

    /// Get the text of token `pos` from the source string.
    pub fn text<'s>(&self, pos: usize, source: &'s str) -> &'s str {
        let start = self.start(pos) as usize;
        let end = if pos + 1 < self.starts.len() {
            self.starts[pos + 1] as usize
        } else {
            source.len()
        };
        &source[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_from_tokens_round_trip() {
        // `@on &.hover` — the driver is now an ELEMENT_REF (`&.`) at the token
        // level (AMPERSAND, DOT, IDENT), not a bare ARG ident.
        let tokens = vec![
            Token {
                kind: SyntaxKind::AT_SIGN,
                text: "@".to_string(),
                offset: 0,
            },
            Token {
                kind: SyntaxKind::IDENT,
                text: "on".to_string(),
                offset: 1,
            },
            Token {
                kind: SyntaxKind::WHITESPACE,
                text: " ".to_string(),
                offset: 3,
            },
            Token {
                kind: SyntaxKind::AMPERSAND,
                text: "&".to_string(),
                offset: 4,
            },
            Token {
                kind: SyntaxKind::DOT,
                text: ".".to_string(),
                offset: 5,
            },
            Token {
                kind: SyntaxKind::IDENT,
                text: "hover".to_string(),
                offset: 6,
            },
            Token {
                kind: SyntaxKind::EOF,
                text: String::new(),
                offset: 11,
            },
        ];

        let input = Input::from_tokens(&tokens);
        assert_eq!(input.len(), 7);
        assert_eq!(input.kind(0), SyntaxKind::AT_SIGN);
        assert_eq!(input.kind(1), SyntaxKind::IDENT);
        assert_eq!(input.kind(2), SyntaxKind::WHITESPACE);
        assert_eq!(input.kind(3), SyntaxKind::AMPERSAND);
        assert_eq!(input.kind(4), SyntaxKind::DOT);
        assert_eq!(input.kind(5), SyntaxKind::IDENT);
        assert_eq!(input.kind(6), SyntaxKind::EOF);

        let source = "@on &.hover";
        assert_eq!(input.text(0, source), "@");
        assert_eq!(input.text(1, source), "on");
        assert_eq!(input.text(2, source), " ");
        assert_eq!(input.text(3, source), "&");
        assert_eq!(input.text(4, source), ".");
        assert_eq!(input.text(5, source), "hover");
    }

    #[test]
    fn input_out_of_bounds_returns_eof() {
        let input = Input::from_tokens(&[]);
        assert_eq!(input.kind(0), SyntaxKind::EOF);
        assert_eq!(input.kind(999), SyntaxKind::EOF);
    }
}
