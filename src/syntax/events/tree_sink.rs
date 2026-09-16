//! TreeSink — produces a Rowan SyntaxNode from parser events.
//!
//! This sink wraps Rowan's `GreenNodeBuilder` and produces the same
//! `(SyntaxNode, Vec<ParseError>)` output as the current CST parser.

use rowan::GreenNodeBuilder;

use crate::syntax::cst::{SyntaxKind, SyntaxNode};

use super::sink::Sink;

/// A parse error with location information (same as the current parser's).
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

/// Sink implementation that builds a Rowan green tree.
pub struct TreeSink {
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl Default for TreeSink {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSink {
    pub fn new() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    /// Finish building and return the syntax tree + errors.
    pub fn finish(self) -> (SyntaxNode, Vec<ParseError>) {
        let green = self.builder.finish();
        let root = SyntaxNode::new_root(green);
        (root, self.errors)
    }
}

impl Sink for TreeSink {
    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(rowan::SyntaxKind(kind.into()));
    }

    fn token(&mut self, kind: SyntaxKind, text: &str) {
        self.builder.token(rowan::SyntaxKind(kind.into()), text);
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn error(&mut self, msg: String, offset: usize) {
        self.errors.push(ParseError {
            message: msg,
            offset,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::cst::lexer::Token;
    use crate::syntax::events::event::Event;
    use crate::syntax::events::input::Input;
    use crate::syntax::events::process::process;

    #[test]
    fn tree_sink_produces_valid_rowan_tree() {
        let source = "@on &.hover";
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
                kind: SyntaxKind::IDENT,
                text: "hover".to_string(),
                offset: 4,
            },
        ];
        let input = Input::from_tokens(&tokens);

        let mut events = vec![
            Event::Start {
                kind: SyntaxKind::ROOT,
                forward_parent: None,
            },
            Event::Start {
                kind: SyntaxKind::DIRECTIVE,
                forward_parent: None,
            },
            Event::Token {
                kind: SyntaxKind::AT_SIGN,
                n_raw_tokens: 1,
            },
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Token {
                kind: SyntaxKind::WHITESPACE,
                n_raw_tokens: 1,
            },
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Finish, // DIRECTIVE
            Event::Finish, // ROOT
        ];

        let mut sink = TreeSink::new();
        process(&mut events, source, &input, &mut sink);
        let (root, errors) = sink.finish();

        assert!(errors.is_empty());
        assert_eq!(root.kind(), SyntaxKind::ROOT);
        // Should have one child: the DIRECTIVE node
        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind(), SyntaxKind::DIRECTIVE);
    }

    #[test]
    fn tree_sink_collects_errors() {
        let source = "bad";
        let tokens = vec![Token {
            kind: SyntaxKind::IDENT,
            text: "bad".to_string(),
            offset: 0,
        }];
        let input = Input::from_tokens(&tokens);

        let mut events = vec![
            Event::Start {
                kind: SyntaxKind::ROOT,
                forward_parent: None,
            },
            Event::Error("unexpected token".to_string()),
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Finish,
        ];

        let mut sink = TreeSink::new();
        process(&mut events, source, &input, &mut sink);
        let (_, errors) = sink.finish();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "unexpected token");
    }
}
