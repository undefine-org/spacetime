//! Enhanced source span utilities for Spacetime.
//!
//! Provides span operations for error reporting, source mapping, and LSP support.

use std::ops::Range;

/// A span in source code, representing a range of byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceSpan {
    pub start: u32,
    pub end: u32,
}

impl SourceSpan {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Create a zero-width span at the given offset
    pub const fn point(offset: u32) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Create a span covering the entire source
    pub const fn full(len: u32) -> Self {
        Self { start: 0, end: len }
    }

    /// Length of this span in bytes
    pub const fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Is this a zero-width span?
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Does this span contain the given byte offset?
    pub const fn contains(&self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Does this span fully contain another span?
    pub const fn contains_span(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// Do these spans overlap?
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Create the union of two spans (smallest span containing both)
    pub fn union(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Create a subspan with relative offsets
    pub fn subspan(&self, rel_start: u32, rel_end: u32) -> Self {
        Self {
            start: self.start.saturating_add(rel_start),
            end: self.start.saturating_add(rel_end).min(self.end),
        }
    }

    /// Extend this span to include another
    pub fn extend_to(&mut self, other: Self) {
        self.start = self.start.min(other.start);
        self.end = self.end.max(other.end);
    }

    /// Convert to a Range<usize> for slicing
    pub fn as_range(&self) -> Range<usize> {
        self.start as usize..self.end as usize
    }

    /// Get the text this span covers
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.as_range()]
    }

    /// Convert to line/column coordinates using a line index
    pub fn to_line_col(&self, line_index: &LineIndex) -> (LineCol, LineCol) {
        (
            line_index.line_col(self.start),
            line_index.line_col(self.end),
        )
    }
}

impl From<Range<usize>> for SourceSpan {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: range.start as u32,
            end: range.end as u32,
        }
    }
}

impl From<SourceSpan> for Range<usize> {
    fn from(span: SourceSpan) -> Self {
        span.as_range()
    }
}

impl From<SourceSpan> for miette::SourceSpan {
    fn from(span: SourceSpan) -> Self {
        miette::SourceSpan::from(span.start as usize..span.end as usize)
    }
}

/// Line and column position (0-based)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineCol {
    pub const fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }

    /// Convert to 1-based for display
    pub fn display_line(&self) -> u32 {
        self.line + 1
    }

    /// Convert to 1-based for display
    pub fn display_col(&self) -> u32 {
        self.col + 1
    }
}

/// Index for fast line/column lookups
pub struct LineIndex {
    /// Byte offset of each line start
    line_starts: Vec<u32>,
}

impl LineIndex {
    /// Build a line index from source text
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (offset, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push((offset + 1) as u32);
            }
        }
        Self { line_starts }
    }

    /// Get the line/column for a byte offset
    pub fn line_col(&self, offset: u32) -> LineCol {
        let line = self
            .line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        LineCol {
            line: line as u32,
            col: offset.saturating_sub(line_start),
        }
    }

    /// Get the byte offset for a line/column
    pub fn offset(&self, line_col: LineCol) -> u32 {
        self.line_starts
            .get(line_col.line as usize)
            .copied()
            .unwrap_or(0)
            .saturating_add(line_col.col)
    }

    /// Get the byte offset of the start of a line
    pub fn line_start(&self, line: u32) -> u32 {
        self.line_starts.get(line as usize).copied().unwrap_or(0)
    }

    /// Get the number of lines
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_basics() {
        let span = SourceSpan::new(10, 20);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
        assert!(span.contains(10));
        assert!(span.contains(15));
        assert!(!span.contains(20)); // exclusive end
        assert!(!span.contains(5));
    }

    #[test]
    fn test_span_union() {
        let a = SourceSpan::new(5, 10);
        let b = SourceSpan::new(15, 20);
        let union = a.union(b);
        assert_eq!(union, SourceSpan::new(5, 20));
    }

    #[test]
    fn test_span_subspan() {
        let span = SourceSpan::new(100, 200);
        let sub = span.subspan(10, 30);
        assert_eq!(sub, SourceSpan::new(110, 130));
    }

    #[test]
    fn test_span_contains_span() {
        let outer = SourceSpan::new(0, 100);
        let inner = SourceSpan::new(20, 50);
        assert!(outer.contains_span(&inner));
        assert!(!inner.contains_span(&outer));
    }

    #[test]
    fn test_line_index() {
        let source = "line one\nline two\nline three";
        let index = LineIndex::new(source);

        assert_eq!(index.line_count(), 3);

        // Start of file
        assert_eq!(index.line_col(0), LineCol::new(0, 0));

        // Middle of first line
        assert_eq!(index.line_col(5), LineCol::new(0, 5));

        // Start of second line (after first \n)
        assert_eq!(index.line_col(9), LineCol::new(1, 0));

        // Middle of third line
        assert_eq!(index.line_col(20), LineCol::new(2, 2));
    }

    #[test]
    fn test_line_col_to_offset() {
        let source = "abc\ndefg\nhi";
        let index = LineIndex::new(source);

        assert_eq!(index.offset(LineCol::new(0, 0)), 0);
        assert_eq!(index.offset(LineCol::new(1, 0)), 4);
        assert_eq!(index.offset(LineCol::new(1, 2)), 6);
        assert_eq!(index.offset(LineCol::new(2, 1)), 10);
    }
}
