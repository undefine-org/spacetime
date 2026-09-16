//! DiagnosticSink — collects parse errors as structured diagnostics.

use crate::syntax::cst::SyntaxKind;

use super::sink::Sink;

/// A collected diagnostic from parsing.
#[derive(Debug, Clone)]
pub struct ParseDiagnostic {
    pub message: String,
    pub offset: usize,
    pub severity: DiagSeverity,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagSeverity {
    Error,
    Warning,
}

/// Sink implementation that collects parse errors as diagnostics.
pub struct DiagnosticSink {
    diagnostics: Vec<ParseDiagnostic>,
}

impl Default for DiagnosticSink {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// Return the collected diagnostics.
    pub fn finish(self) -> Vec<ParseDiagnostic> {
        self.diagnostics
    }
}

impl Sink for DiagnosticSink {
    fn start_node(&mut self, _kind: SyntaxKind) {
        // DiagnosticSink doesn't track node structure
    }

    fn token(&mut self, _kind: SyntaxKind, _text: &str) {
        // DiagnosticSink doesn't track tokens
    }

    fn finish_node(&mut self) {
        // DiagnosticSink doesn't track node structure
    }

    fn error(&mut self, msg: String, offset: usize) {
        self.diagnostics.push(ParseDiagnostic {
            message: msg,
            offset,
            severity: DiagSeverity::Error,
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
    fn diag_sink_collects_errors() {
        let source = "bad token";
        let tokens = vec![
            Token {
                kind: SyntaxKind::IDENT,
                text: "bad".to_string(),
                offset: 0,
            },
            Token {
                kind: SyntaxKind::WHITESPACE,
                text: " ".to_string(),
                offset: 3,
            },
            Token {
                kind: SyntaxKind::IDENT,
                text: "token".to_string(),
                offset: 4,
            },
        ];
        let input = Input::from_tokens(&tokens);

        let mut events = vec![
            Event::Start {
                kind: SyntaxKind::ROOT,
                forward_parent: None,
            },
            Event::Error("unexpected".to_string()),
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Token {
                kind: SyntaxKind::WHITESPACE,
                n_raw_tokens: 1,
            },
            Event::Error("still bad".to_string()),
            Event::Token {
                kind: SyntaxKind::IDENT,
                n_raw_tokens: 1,
            },
            Event::Finish,
        ];

        let mut sink = DiagnosticSink::new();
        process(&mut events, source, &input, &mut sink);
        let diags = sink.finish();

        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].message, "unexpected");
        assert_eq!(diags[0].offset, 0);
        assert_eq!(diags[1].message, "still bad");
    }

    #[test]
    fn diag_sink_empty_for_valid_input() {
        let source = "@on";
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
        ];
        let input = Input::from_tokens(&tokens);

        let mut events = vec![
            Event::Start {
                kind: SyntaxKind::ROOT,
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
            Event::Finish,
        ];

        let mut sink = DiagnosticSink::new();
        process(&mut events, source, &input, &mut sink);
        let diags = sink.finish();

        assert!(diags.is_empty());
    }
}
