//! Event processing — resolves forward_parent chains and drives a Sink.
//!
//! Adapted from rust-analyzer's `sink.rs`. The `process()` function walks
//! the flat `Vec<Event>`, resolves `forward_parent` chains, inserts trivia
//! tokens, and calls `Sink` methods in correct nested tree order.

use crate::syntax::cst::SyntaxKind;

use super::event::Event;
use super::input::Input;
use super::sink::Sink;

/// Process a flat event stream into nested Sink calls.
///
/// 1. Walk events, following `forward_parent` chains to find the outermost ancestor
/// 2. Emit `start_node` / `token` / `finish_node` in correct nested order
/// 3. Insert trivia tokens (WHITESPACE, COMMENT) between non-trivia tokens
pub fn process(events: &mut Vec<Event>, source: &str, input: &Input, sink: &mut dyn Sink) {
    // Track which raw token we're at for trivia insertion
    let mut token_pos: usize = 0;

    // We need to resolve forward_parent chains. For each Start event with
    // forward_parent, we follow the chain to find all ancestors, then emit
    // them outermost-first.
    let mut forward_parents = Vec::new();

    for i in 0..events.len() {
        match std::mem::replace(&mut events[i], Event::Finish) {
            Event::Start {
                kind,
                forward_parent,
            } => {
                // Skip tombstones (abandoned markers)
                if kind == SyntaxKind::TOMBSTONE && forward_parent.is_none() {
                    continue;
                }

                // Collect forward_parent chain
                forward_parents.clear();
                let mut idx = i;
                let mut fp = forward_parent;
                while let Some(offset) = fp {
                    idx += offset as usize;
                    fp = match std::mem::replace(&mut events[idx], Event::Finish) {
                        Event::Start {
                            kind,
                            forward_parent,
                        } => {
                            forward_parents.push(kind);
                            forward_parent
                        }
                        _ => unreachable!("forward_parent pointed to non-Start event"),
                    };
                }

                // Emit ancestors outermost-first
                for &ancestor_kind in forward_parents.iter().rev() {
                    if ancestor_kind != SyntaxKind::TOMBSTONE {
                        sink.start_node(ancestor_kind);
                    }
                }

                // Emit this node's start (if not a tombstone)
                if kind != SyntaxKind::TOMBSTONE {
                    sink.start_node(kind);
                }
            }
            Event::Token {
                kind: _,
                n_raw_tokens,
            } => {
                // Emit trivia tokens leading up to this token's position,
                // then emit the token itself
                for _ in 0..n_raw_tokens {
                    emit_token(&mut token_pos, source, input, sink);
                }
            }
            Event::Finish => {
                sink.finish_node();
            }
            Event::Error(msg) => {
                let offset = input.start(token_pos) as usize;
                sink.error(msg, offset);
            }
        }
    }
}

/// Emit a single raw token (advancing token_pos), including any leading trivia.
fn emit_token(token_pos: &mut usize, source: &str, input: &Input, sink: &mut dyn Sink) {
    let kind = input.kind(*token_pos);
    let text = input.text(*token_pos, source);
    sink.token(kind, text);
    *token_pos += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::cst::lexer::Token;

    /// A test sink that records calls for verification.
    struct TestSink {
        calls: Vec<String>,
    }

    impl TestSink {
        fn new() -> Self {
            Self { calls: Vec::new() }
        }
    }

    impl Sink for TestSink {
        fn start_node(&mut self, kind: SyntaxKind) {
            self.calls.push(format!("start({:?})", kind));
        }
        fn token(&mut self, kind: SyntaxKind, text: &str) {
            self.calls.push(format!("token({:?}, {:?})", kind, text));
        }
        fn finish_node(&mut self) {
            self.calls.push("finish()".to_string());
        }
        fn error(&mut self, msg: String, offset: usize) {
            self.calls.push(format!("error({:?}, {})", msg, offset));
        }
    }

    fn make_tokens(specs: &[(SyntaxKind, &str)]) -> (Vec<Token>, Input, String) {
        let mut source = String::new();
        let mut tokens = Vec::new();
        let mut offset = 0;
        for (kind, text) in specs {
            tokens.push(Token {
                kind: *kind,
                text: text.to_string(),
                offset,
            });
            source.push_str(text);
            offset += text.len();
        }
        let input = Input::from_tokens(&tokens);
        (tokens, input, source)
    }

    #[test]
    fn simple_node_with_token() {
        let (_, input, source) = make_tokens(&[(SyntaxKind::IDENT, "foo")]);

        let mut events = vec![
            Event::Start {
                kind: SyntaxKind::ROOT,
                forward_parent: None,
            },
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Finish,
        ];

        let mut sink = TestSink::new();
        process(&mut events, &source, &input, &mut sink);

        assert_eq!(
            sink.calls,
            vec!["start(ROOT)", "token(IDENT, \"foo\")", "finish()",]
        );
    }

    #[test]
    fn nested_nodes() {
        let (_, input, source) =
            make_tokens(&[(SyntaxKind::AT_SIGN, "@"), (SyntaxKind::IDENT, "on")]);

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
            Event::Finish, // DIRECTIVE
            Event::Finish, // ROOT
        ];

        let mut sink = TestSink::new();
        process(&mut events, &source, &input, &mut sink);

        assert_eq!(
            sink.calls,
            vec![
                "start(ROOT)",
                "start(DIRECTIVE)",
                "token(AT_SIGN, \"@\")",
                "token(IDENT, \"on\")",
                "finish()",
                "finish()",
            ]
        );
    }

    #[test]
    fn forward_parent_chain_depth_1() {
        // Simulates: parse IDENT, then precede() to wrap in BINARY_EXPR
        let (_, input, source) = make_tokens(&[
            (SyntaxKind::IDENT, "a"),
            (SyntaxKind::PLUS, "+"),
            (SyntaxKind::IDENT, "b"),
        ]);

        // The inner EXPR points forward_parent to the outer BINARY_EXPR
        let mut events = vec![
            Event::Start {
                kind: SyntaxKind::EXPR,
                forward_parent: Some(4),
            }, // idx 0 -> idx 4
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Finish, // inner EXPR
            Event::Token {
                kind: SyntaxKind::PLUS,
                n_raw_tokens: 1,
            },
            Event::Start {
                kind: SyntaxKind::BINARY_EXPR,
                forward_parent: None,
            }, // idx 4
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Finish, // BINARY_EXPR
        ];

        let mut sink = TestSink::new();
        process(&mut events, &source, &input, &mut sink);

        // The BINARY_EXPR should wrap the EXPR
        assert_eq!(sink.calls[0], "start(BINARY_EXPR)");
        assert_eq!(sink.calls[1], "start(EXPR)");
    }

    #[test]
    fn error_event() {
        let (_, input, source) = make_tokens(&[(SyntaxKind::IDENT, "foo")]);

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

        let mut sink = TestSink::new();
        process(&mut events, &source, &input, &mut sink);

        assert!(sink.calls.iter().any(|c| c.contains("error")));
    }
}
