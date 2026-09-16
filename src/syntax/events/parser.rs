//! Core Parser struct for the event-based parser.
//!
//! The parser consumes `Input` and emits `Vec<Event>`. It never touches source
//! text directly — only SyntaxKind tokens via `Input`. Grammar functions drive
//! the parser by calling `bump()`, `expect()`, `at()`, `start()`, etc.

use std::cell::Cell;

use crate::syntax::cst::SyntaxKind;

use super::event::{Event, Marker};
use super::input::Input;
use super::strategy::ErrorStrategy;

/// The event-based parser. Grammar functions call methods on this struct
/// to produce a stream of events.
pub struct Parser<'i> {
    input: &'i Input,
    pos: usize,
    pub(crate) events: Vec<Event>,
    fuel: Cell<u32>,
    strategy: ErrorStrategy,
    failed: bool,
}

impl<'i> Parser<'i> {
    /// Create a new parser for the given input with the specified error strategy.
    pub fn new(input: &'i Input, strategy: ErrorStrategy) -> Self {
        Self {
            input,
            pos: 0,
            events: Vec::new(),
            fuel: Cell::new(256),
            strategy,
            failed: false,
        }
    }

    /// The current token kind, skipping trivia for lookahead but NOT consuming trivia.
    pub fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Kind of the nearest non-trivia token BEFORE the current position
    /// (lookback sibling of `nth`). Used when a routing decision depends on
    /// the token that immediately precedes the cursor regardless of how args
    /// were grouped (e.g. "is this `@` directly preceded by a value colon?").
    pub fn prev_non_trivia(&self) -> SyntaxKind {
        let fuel = self.fuel.get();
        assert!(fuel > 0, "parser fuel exhausted (infinite loop bug)");
        self.fuel.set(fuel - 1);

        let mut pos = self.pos;
        loop {
            if pos == 0 {
                return SyntaxKind::EOF;
            }
            pos -= 1;
            let kind = self.input.kind(pos);
            if kind == SyntaxKind::EOF {
                return SyntaxKind::EOF;
            }
            if !kind.is_trivia() {
                return kind;
            }
        }
    }

    /// Lookahead by `n` non-trivia tokens. Consumes fuel.
    /// Trivia (WHITESPACE, COMMENT) is skipped for lookahead purposes but
    /// stays in the input for lossless CST construction.
    pub fn nth(&self, n: usize) -> SyntaxKind {
        let fuel = self.fuel.get();
        assert!(fuel > 0, "parser fuel exhausted (infinite loop bug)");
        self.fuel.set(fuel - 1);

        let mut pos = self.pos;
        let mut non_trivia_seen = 0;
        loop {
            let kind = self.input.kind(pos);
            if kind == SyntaxKind::EOF {
                return SyntaxKind::EOF;
            }
            if !kind.is_trivia() {
                if non_trivia_seen == n {
                    return kind;
                }
                non_trivia_seen += 1;
            }
            pos += 1;
        }
    }

    /// Whether a newline precedes the current (next non-trivia) token.
    ///
    /// Used by grammar to stop greedy inline-arg consumption at a statement
    /// boundary, e.g. a bodyless `@import "x"` followed by `\n.box { … }`.
    pub fn newline_before_current(&self) -> bool {
        let mut pos = self.pos;
        loop {
            let kind = self.input.kind(pos);
            if kind == SyntaxKind::EOF {
                return false;
            }
            if !kind.is_trivia() {
                return self.input.newline_before(pos);
            }
            pos += 1;
        }
    }

    /// FEAT-118: whether the IMMEDIATE next tokens (no intervening trivia) are
    /// `SLASH IDENT` — a TIGHTLY-bound namespace qualifier segment like the
    /// `/camera` in `@scene/camera`. Raw-adjacency (byte offsets touch) is what
    /// distinguishes a name qualifier from CSS shorthand division (`1 / 3`,
    /// which is spaced and lives in value position). Used by the directive
    /// grammar to extend the directive name with `/segment` runs.
    pub fn at_tight_qualifier(&self) -> bool {
        // raw token at self.pos must be SLASH with no preceding trivia, and the
        // very next raw token an IDENT/keyword with no trivia between.
        let slash = self.pos;
        if self.input.kind(slash) != SyntaxKind::SLASH {
            return false;
        }
        // The slash must touch the preceding token (no leading trivia on it).
        if slash == 0 || self.input.kind(slash - 1).is_trivia() {
            return false;
        }
        let seg = slash + 1;
        let seg_kind = self.input.kind(seg);
        (seg_kind == SyntaxKind::IDENT || seg_kind.is_keyword())
            && self.input.start(seg) == self.input.start(slash) + 1
    }

    /// Check if the current non-trivia token matches `kind`.
    pub fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Check if the current non-trivia token matches any of the given kinds.
    pub fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        let c = self.current();
        kinds.contains(&c)
    }

    /// Check if we're at the end of input.
    pub fn at_end(&self) -> bool {
        self.current() == SyntaxKind::EOF
    }

    /// Consume the current token, asserting it matches `kind`.
    /// Emits any leading trivia tokens first, then the token itself.
    /// Resets fuel to 256.
    pub fn bump(&mut self, kind: SyntaxKind) {
        assert!(
            self.eat(kind),
            "bump({:?}) called but current is {:?}",
            kind,
            self.current()
        );
    }

    /// Consume the current token regardless of its kind.
    /// Emits any leading trivia first.
    pub fn bump_any(&mut self) {
        self.eat_trivia();
        if self.pos < self.input.len() {
            let kind = self.input.kind(self.pos);
            self.do_bump(kind);
        }
    }

    /// Try to consume a token of the given `kind`. Returns true if consumed.
    /// Emits any leading trivia first.
    pub fn eat(&mut self, kind: SyntaxKind) -> bool {
        self.eat_trivia();
        if self.input.kind(self.pos) == kind {
            self.do_bump(kind);
            true
        } else {
            false
        }
    }

    /// Expect a token of `kind`. If present, consume it. If not, emit an error.
    pub fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            self.error(format!("expected {:?}", kind));
        }
    }

    /// Begin a new node. Returns a `Marker` that must be completed or abandoned.
    pub fn start(&mut self) -> Marker {
        let pos = self.events.len();
        self.events.push(Event::Start {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        Marker::new(pos)
    }

    /// Emit an error message. Behavior depends on the error strategy:
    /// - Fatal: emit error event, set failed flag
    /// - Verbose: emit error event, continue
    /// - Silent: discard silently
    pub fn error(&mut self, msg: impl Into<String>) {
        match self.strategy {
            ErrorStrategy::Fatal => {
                self.events.push(Event::Error(msg.into()));
                self.failed = true;
            }
            ErrorStrategy::Verbose => {
                self.events.push(Event::Error(msg.into()));
            }
            ErrorStrategy::Silent => {
                // Discard
            }
        }
    }

    /// Emit an error and skip tokens until one of the recovery set is found.
    pub fn error_recover(&mut self, msg: impl Into<String>, recovery: &[SyntaxKind]) {
        if self.at_any(recovery) || self.at_end() {
            self.error(msg);
            return;
        }

        let m = self.start();
        self.error(msg);
        while !self.at_any(recovery) && !self.at_end() {
            self.bump_any();
        }
        m.complete(&mut self.events, SyntaxKind::ERROR);
    }

    /// Whether the parser has encountered a fatal error.
    pub fn has_failed(&self) -> bool {
        self.failed
    }

    /// Finish parsing and return the event stream.
    pub fn finish(self) -> Vec<Event> {
        self.events
    }

    /// Current byte position in source (for diagnostics).
    pub fn current_offset(&self) -> u32 {
        self.input.start(self.pos)
    }

    /// True when the NEXT non-trivia token begins exactly where the current one
    /// ends, and is a unit suffix (`px`, `ms`, `deg`, or `%`).
    ///
    /// At Kernel tier (PLAN-122 W1.2) a dimension is two tokens: `500ms` is
    /// `NUMBER("500") IDENT("ms")`. Anything that used to consume one fused
    /// `NUMBER_WITH_UNIT` must consume the pair, or the unit is left behind and
    /// reported as a stray token ("Unexpected N extra inline token(s)").
    ///
    /// The test is pure POSITION — it carries no list of units, so `40 px`
    /// (spaced, two values) stays two values while `40px` stays one. Which unit
    /// spellings are legal is decided by the stdlib grammars in
    /// `stdlib/capture-types/css-values.st`, in one place.
    pub fn at_dimension_start(&self) -> bool {
        if self.current() != SyntaxKind::NUMBER {
            return false;
        }
        // The very next token is checked WITHOUT skipping trivia: whitespace
        // between the number and the unit means they are two values, not a
        // dimension. Because the token stream INCLUDES trivia, "the immediately
        // following token is a unit" already means there was no gap — `40 px`
        // has a WHITESPACE token at `pos + 1` and so is correctly two values.
        matches!(
            self.input.kind(self.pos + 1),
            SyntaxKind::IDENT | SyntaxKind::PERCENT
        )
    }

    /// Get the text of the nth non-trivia token from the source.
    /// Used for keyword dispatch (e.g., meta-clause keyword detection).
    /// TOTAL: returns "" when the token's range exceeds `source` (nested-body
    /// grammar paths thread an empty source — a text check there must safely
    /// fail closed, never panic; PLAN-077 W2's @match value routing hits this).
    pub fn nth_text<'s>(&self, n: usize, source: &'s str) -> &'s str {
        let mut pos = self.pos;
        let mut non_trivia_seen = 0;
        loop {
            let kind = self.input.kind(pos);
            if kind == SyntaxKind::EOF {
                return "";
            }
            if !kind.is_trivia() {
                if non_trivia_seen == n {
                    let start = self.input.start(pos) as usize;
                    return if start < source.len() {
                        self.input.text(pos, source)
                    } else {
                        ""
                    };
                }
                non_trivia_seen += 1;
            }
            pos += 1;
        }
    }

    // -- internal --

    /// Emit trivia tokens (whitespace, comments) before the current non-trivia token.
    fn eat_trivia(&mut self) {
        while self.pos < self.input.len() && self.input.kind(self.pos).is_trivia() {
            let kind = self.input.kind(self.pos);
            self.do_bump(kind);
        }
    }

    /// Emit a single token event and advance.
    fn do_bump(&mut self, kind: SyntaxKind) {
        self.events.push(Event::Token {
            kind,
            n_raw_tokens: 1,
        });
        self.pos += 1;
        self.fuel.set(256);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::cst::lexer::Token;

    fn make_input(tokens: &[(SyntaxKind, &str)]) -> (Input, Vec<Token>) {
        let mut offset = 0usize;
        let tok_vec: Vec<Token> = tokens
            .iter()
            .map(|(kind, text)| {
                let t = Token {
                    kind: *kind,
                    text: text.to_string(),
                    offset,
                };
                offset += text.len();
                t
            })
            .collect();
        let input = Input::from_tokens(&tok_vec);
        (input, tok_vec)
    }

    #[test]
    fn parser_bump_and_at() {
        let (input, _) = make_input(&[
            (SyntaxKind::AT_SIGN, "@"),
            (SyntaxKind::IDENT, "on"),
            (SyntaxKind::EOF, ""),
        ]);
        let mut p = Parser::new(&input, ErrorStrategy::Verbose);
        assert!(p.at(SyntaxKind::AT_SIGN));
        p.bump(SyntaxKind::AT_SIGN);
        assert!(p.at(SyntaxKind::IDENT));
        p.bump(SyntaxKind::IDENT);
        assert!(p.at_end());
    }

    #[test]
    fn parser_skips_trivia_for_lookahead() {
        let (input, _) = make_input(&[
            (SyntaxKind::IDENT, "a"),
            (SyntaxKind::WHITESPACE, " "),
            (SyntaxKind::PLUS, "+"),
            (SyntaxKind::WHITESPACE, " "),
            (SyntaxKind::IDENT, "b"),
            (SyntaxKind::EOF, ""),
        ]);
        let p = Parser::new(&input, ErrorStrategy::Verbose);
        // nth(0) skips whitespace -> IDENT "a"
        assert_eq!(p.nth(0), SyntaxKind::IDENT);
        // nth(1) skips WS -> PLUS
        assert_eq!(p.nth(1), SyntaxKind::PLUS);
        // nth(2) skips WS -> IDENT "b"
        assert_eq!(p.nth(2), SyntaxKind::IDENT);
    }

    #[test]
    fn parser_eat_emits_leading_trivia() {
        let (input, _) = make_input(&[
            (SyntaxKind::WHITESPACE, "  "),
            (SyntaxKind::IDENT, "foo"),
            (SyntaxKind::EOF, ""),
        ]);
        let mut p = Parser::new(&input, ErrorStrategy::Verbose);
        assert!(p.eat(SyntaxKind::IDENT));
        let events = p.finish();
        // Should have: WS Token + IDENT Token
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            Event::Token {
                kind: SyntaxKind::WHITESPACE,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            Event::Token {
                kind: SyntaxKind::IDENT,
                ..
            }
        ));
    }

    #[test]
    fn parser_strategy_fatal_stops() {
        let (input, _) = make_input(&[(SyntaxKind::IDENT, "foo"), (SyntaxKind::EOF, "")]);
        let mut p = Parser::new(&input, ErrorStrategy::Fatal);
        p.error("test error");
        assert!(p.has_failed());
    }

    #[test]
    fn parser_strategy_verbose_continues() {
        let (input, _) = make_input(&[(SyntaxKind::IDENT, "foo"), (SyntaxKind::EOF, "")]);
        let mut p = Parser::new(&input, ErrorStrategy::Verbose);
        p.error("test error");
        assert!(!p.has_failed());
        let events = p.finish();
        assert!(events.iter().any(|e| matches!(e, Event::Error(_))));
    }

    #[test]
    fn parser_strategy_silent_discards() {
        let (input, _) = make_input(&[(SyntaxKind::IDENT, "foo"), (SyntaxKind::EOF, "")]);
        let mut p = Parser::new(&input, ErrorStrategy::Silent);
        p.error("test error");
        assert!(!p.has_failed());
        let events = p.finish();
        assert!(!events.iter().any(|e| matches!(e, Event::Error(_))));
    }

    #[test]
    fn parser_start_and_complete() {
        let (input, _) = make_input(&[
            (SyntaxKind::AT_SIGN, "@"),
            (SyntaxKind::IDENT, "on"),
            (SyntaxKind::EOF, ""),
        ]);
        let mut p = Parser::new(&input, ErrorStrategy::Verbose);
        let m = p.start();
        p.bump(SyntaxKind::AT_SIGN);
        p.bump(SyntaxKind::IDENT);
        m.complete(&mut p.events, SyntaxKind::DIRECTIVE);
        let events = p.finish();
        assert!(matches!(
            events[0],
            Event::Start {
                kind: SyntaxKind::DIRECTIVE,
                ..
            }
        ));
        assert!(matches!(events.last(), Some(Event::Finish)));
    }

    #[test]
    fn parser_error_recover() {
        let (input, _) = make_input(&[
            (SyntaxKind::IDENT, "bad"),
            (SyntaxKind::IDENT, "stuff"),
            (SyntaxKind::SEMICOLON, ";"),
            (SyntaxKind::IDENT, "good"),
            (SyntaxKind::EOF, ""),
        ]);
        let mut p = Parser::new(&input, ErrorStrategy::Verbose);
        p.error_recover("unexpected token", &[SyntaxKind::SEMICOLON]);
        // Should have skipped "bad" and "stuff", stopping at ";"
        assert!(p.at(SyntaxKind::SEMICOLON));
    }
}
