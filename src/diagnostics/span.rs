//! Source location tracking for diagnostics.

/// A span representing a range of bytes in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceSpan {
    /// Byte offset start (inclusive)
    pub start: usize,
    /// Byte offset end (exclusive)
    pub end: usize,
}

impl SourceSpan {
    /// Create a new span from start and end byte offsets.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Merge two spans into one covering both.
    pub fn merge(self, other: SourceSpan) -> SourceSpan {
        SourceSpan {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Resolve this span to line and column numbers (1-indexed).
    /// Returns (line, column, line_text).
    pub fn resolve<'a>(&self, source: &'a str) -> (usize, usize, &'a str) {
        let mut line = 1;
        let mut line_start = 0;

        for (i, ch) in source.char_indices() {
            if i >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                line_start = i + 1;
            }
        }

        let column = self.start - line_start + 1;

        // Extract the line text
        let line_end = source[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(source.len());
        let line_text = &source[line_start..line_end];

        (line, column, line_text)
    }

    /// Get the length of this span in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if this span is empty.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl From<crate::parser::SourceSpan> for SourceSpan {
    fn from(span: crate::parser::SourceSpan) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}
