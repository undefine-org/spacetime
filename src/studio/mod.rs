//! Studio — the visual editor's source model (PLAN-154).
//!
//! The studio edits a site by editing its `.st` SOURCE, never the rendered
//! page: the page is a projection, the source is the truth. This module is the
//! layer that turns "the user changed this text / moved this row" into a
//! byte-exact rewrite of the right file, and is deliberately free of HTTP and
//! of the browser so every rule here is unit-tested.
//!
//! Layers, bottom up:
//!
//! - [`literal`] — a LOSSLESS reader for the value of an `@data` declaration
//!   (JSON-shaped): every row, key and value keeps its byte span, so a field can
//!   be replaced or a row moved without reformatting the author's file.
//! - [`markup`] — a lossless tokenizer for HTML element literals inside `.st`:
//!   elements, classes, and each static text run with its span.
//! - [`outline`] — the SHAPE of a page's code: its files, lists, markup,
//!   templates and style rules, built from those readers (and the real parser's
//!   rule spans). The explorer and the inspector sidebar are drawn from this.
//! - [`ops`] — the edits: set text, set a field, move / duplicate / delete a row,
//!   set a style declaration. Pure `files in -> files out` functions with guards:
//!   an edit that cannot be placed UNIQUELY is refused, never guessed.
//!
//! Addressing is by CONTENT, not by DOM position: the browser reports the text it
//! shows (and the list and row it sits in, when it knows), and [`ops`] finds the
//! single place in the source that produces it. That survives templates,
//! filtered and derived lists, and literate prose, which positional ids do not.

pub mod literal;
pub mod markup;
pub mod ops;
pub mod outline;
pub mod score;

/// A byte range in one source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

/// Collapse every whitespace run to one space and trim: the comparison form for
/// text, since the browser reflows source whitespace.
pub fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
