//! Event types for the event-based parser.
//!
//! The parser emits a flat `Vec<Event>`. Multiple sinks (TreeSink, MatchSink,
//! DiagnosticSink) consume these events to produce different outputs from a
//! single parse pass.
//!
//! Adapted from rust-analyzer's event-based parser design.

use crate::syntax::cst::SyntaxKind;

/// A parser event. Events form a flat stream that `process()` resolves into
/// nested Sink calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Start a new node. `forward_parent` links this node to an ancestor
    /// that was started later (used by `CompletedMarker::precede()`).
    Start {
        kind: SyntaxKind,
        forward_parent: Option<u32>,
    },
    /// Emit a token. `n_raw_tokens` is typically 1, but may be >1 for
    /// compound tokens.
    Token { kind: SyntaxKind, n_raw_tokens: u8 },
    /// Finish the current node (matching the most recent unfinished Start).
    Finish,
    /// A parse error at the current position.
    Error(String),
}

/// A `DropBomb` that panics if dropped without being defused.
/// Used to ensure Markers are always completed or abandoned.
#[derive(Debug)]
pub(crate) struct DropBomb {
    defused: bool,
    #[cfg(debug_assertions)]
    msg: &'static str,
}

impl DropBomb {
    pub(crate) fn new(msg: &'static str) -> Self {
        Self {
            defused: false,
            #[cfg(debug_assertions)]
            msg,
        }
    }

    pub(crate) fn defuse(&mut self) {
        self.defused = true;
    }
}

impl Drop for DropBomb {
    fn drop(&mut self) {
        if !self.defused && !std::thread::panicking() {
            #[cfg(debug_assertions)]
            panic!("DropBomb: {}", self.msg);
            #[cfg(not(debug_assertions))]
            panic!("DropBomb: marker was neither completed nor abandoned");
        }
    }
}

/// Marks the start of a node in the event stream. Must be either completed
/// (via `complete()`) or abandoned (via `abandon()`), otherwise panics on drop.
#[derive(Debug)]
pub struct Marker {
    pub(crate) pos: usize,
    pub(crate) bomb: DropBomb,
}

impl Marker {
    pub(crate) fn new(pos: usize) -> Self {
        Self {
            pos,
            bomb: DropBomb::new("Marker must be either completed or abandoned"),
        }
    }

    /// Complete this marker, wrapping all events since it was opened into a
    /// node of the given `kind`. Returns a `CompletedMarker` that can be used
    /// for `precede()`.
    pub fn complete(mut self, events: &mut Vec<Event>, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        match &mut events[self.pos] {
            Event::Start { kind: slot, .. } => *slot = kind,
            _ => unreachable!("Marker::complete on non-Start event"),
        }
        events.push(Event::Finish);
        CompletedMarker {
            pos: self.pos,
            kind,
        }
    }

    /// Abandon this marker, replacing its Start event with a tombstone.
    /// The events emitted since the marker was opened remain, but no node
    /// wraps them.
    pub fn abandon(mut self, events: &mut Vec<Event>) {
        self.bomb.defuse();
        if self.pos == events.len() - 1 {
            // The Start event is the last event, just pop it
            match events.pop() {
                Some(Event::Start {
                    kind: SyntaxKind::TOMBSTONE,
                    ..
                }) => {}
                _ => unreachable!("Marker::abandon on non-Start event"),
            }
        }
        // Otherwise leave the tombstone Start in place — process() will skip it
    }
}

// Ergonomic methods for grammar functions that work with &mut Parser.
impl Marker {
    /// Complete this marker using a Parser reference (ergonomic API for grammar functions).
    pub fn complete_p(self, p: &mut super::parser::Parser, kind: SyntaxKind) -> CompletedMarker {
        self.complete(&mut p.events, kind)
    }

    /// Abandon this marker using a Parser reference.
    pub fn abandon_p(self, p: &mut super::parser::Parser) {
        self.abandon(&mut p.events)
    }
}

/// A completed node in the event stream. Supports `precede()` for retroactive
/// wrapping (e.g., wrapping `a` in `a + b` to become a BinaryExpr).
#[derive(Debug, Clone, Copy)]
pub struct CompletedMarker {
    pub(crate) pos: usize,
    pub(crate) kind: SyntaxKind,
}

impl CompletedMarker {
    /// The syntax kind of this completed node.
    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Create a new Marker that wraps this completed node and everything that
    /// follows. This is used for left-recursive constructs like binary expressions:
    ///
    /// ```text
    /// // Parse "a"  -> CompletedMarker for IDENT
    /// // See "+"    -> precede() to start BINARY_EXPR before "a"
    /// // Parse "b"  -> complete BINARY_EXPR
    /// ```
    ///
    /// Sets `forward_parent` on the original Start event so `process()` can
    /// resolve the chain.
    pub fn precede(self, events: &mut Vec<Event>) -> Marker {
        let new_pos = events.len();
        events.push(Event::Start {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        // Point the original Start's forward_parent to the new wrapper
        match &mut events[self.pos] {
            Event::Start { forward_parent, .. } => {
                *forward_parent = Some((new_pos - self.pos) as u32);
            }
            _ => unreachable!("CompletedMarker::precede on non-Start event"),
        }
        Marker::new(new_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_complete() {
        let mut events = vec![];
        let m = Marker::new(events.len());
        events.push(Event::Start {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        events.push(Event::Token {
            kind: SyntaxKind::IDENT,
            n_raw_tokens: 1,
        });
        let cm = m.complete(&mut events, SyntaxKind::EXPR);
        assert_eq!(cm.kind(), SyntaxKind::EXPR);
        assert_eq!(events.len(), 3); // Start + Token + Finish
        assert!(matches!(
            events[0],
            Event::Start {
                kind: SyntaxKind::EXPR,
                ..
            }
        ));
        assert!(matches!(events[2], Event::Finish));
    }

    #[test]
    fn marker_abandon() {
        let mut events = vec![];
        let m = Marker::new(events.len());
        events.push(Event::Start {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        m.abandon(&mut events);
        assert!(events.is_empty()); // Tombstone popped
    }

    #[test]
    fn completed_marker_precede() {
        let mut events = vec![];
        // Start inner node
        let m = Marker::new(events.len());
        events.push(Event::Start {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        events.push(Event::Token {
            kind: SyntaxKind::IDENT,
            n_raw_tokens: 1,
        });
        let cm = m.complete(&mut events, SyntaxKind::EXPR);

        // Precede it with a wrapper
        let outer = cm.precede(&mut events);
        events.push(Event::Token {
            kind: SyntaxKind::PLUS,
            n_raw_tokens: 1,
        });
        events.push(Event::Token {
            kind: SyntaxKind::IDENT,
            n_raw_tokens: 1,
        });
        let _cm2 = outer.complete(&mut events, SyntaxKind::BINARY_EXPR);

        // The original Start should have forward_parent set
        match &events[0] {
            Event::Start { forward_parent, .. } => {
                assert!(forward_parent.is_some());
            }
            _ => panic!("expected Start event"),
        }
    }

    #[test]
    #[should_panic(expected = "DropBomb")]
    fn drop_bomb_panics_on_uncommitted_marker() {
        let mut events = vec![];
        let _m = Marker::new(events.len());
        events.push(Event::Start {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        // _m dropped without complete() or abandon() -> panic
    }
}
