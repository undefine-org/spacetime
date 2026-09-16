//! Position mapping between byte offsets and LSP positions.
//!
//! LSP uses UTF-16 code units for character positions, while Rust strings
//! use byte offsets. This module provides conversion utilities.

use tower_lsp::lsp_types::{Position, Range};

use crate::diagnostics::SourceSpan;

/// Mapper for converting between byte offsets and LSP positions.
///
/// LSP positions are line/character pairs where character is a UTF-16 code unit offset.
/// This mapper pre-computes line start offsets for efficient conversions.
#[derive(Debug, Clone)]
pub struct PositionMapper {
    /// Byte offsets of the start of each line (0-indexed line number -> byte offset)
    line_starts: Vec<usize>,
    /// The source content (needed for UTF-16 conversion)
    content: String,
}

impl PositionMapper {
    /// Create a new PositionMapper from source content.
    ///
    /// Pre-computes line start offsets for efficient position lookups.
    pub fn new(content: &str) -> Self {
        let mut line_starts = vec![0]; // Line 0 starts at byte 0

        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                // Next line starts after the newline
                line_starts.push(i + 1);
            }
        }

        Self {
            line_starts,
            content: content.to_string(),
        }
    }

    /// Get the line start offsets (for testing).
    pub fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }

    /// Convert an LSP Position to a byte offset.
    ///
    /// The position uses UTF-16 code units for the character offset.
    /// Returns the byte offset in the source string.
    pub fn offset_from_position(&self, pos: Position) -> usize {
        let line = pos.line as usize;

        // Get the byte offset of the start of this line
        let line_start = if line < self.line_starts.len() {
            self.line_starts[line]
        } else {
            // Position is past end of file
            return self.content.len();
        };

        // Get the content of this line
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1].saturating_sub(1) // Exclude the newline
        } else {
            self.content.len()
        };

        let line_content = &self.content[line_start..line_end.min(self.content.len())];

        // Convert UTF-16 code unit offset to byte offset
        let char_offset = pos.character as usize;
        let byte_offset = utf16_offset_to_byte_offset(line_content, char_offset);

        line_start + byte_offset
    }

    /// Convert a byte offset to an LSP Position.
    ///
    /// Returns a position with line (0-indexed) and character (UTF-16 code units).
    pub fn position_from_offset(&self, offset: usize) -> Position {
        // Binary search for the line containing this offset
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact, // Offset is exactly at line start
            Err(insert_pos) => insert_pos.saturating_sub(1), // Offset is within this line
        };

        let line_start = self.line_starts[line];

        // Get the portion of the line up to the offset
        let byte_offset_in_line = offset.saturating_sub(line_start);
        let line_content = &self.content[line_start..];
        let prefix = if byte_offset_in_line <= line_content.len() {
            &line_content[..byte_offset_in_line]
        } else {
            line_content
        };

        // Convert byte offset to UTF-16 code units
        let character = byte_offset_to_utf16_offset(prefix);

        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    /// Convert a SourceSpan to an LSP Range.
    pub fn span_to_range(&self, span: &SourceSpan) -> Range {
        Range {
            start: self.position_from_offset(span.start),
            end: self.position_from_offset(span.end),
        }
    }

    /// Convert an LSP Range to a SourceSpan.
    pub fn range_to_span(&self, range: &Range) -> SourceSpan {
        SourceSpan {
            start: self.offset_from_position(range.start),
            end: self.offset_from_position(range.end),
        }
    }
}

/// Convert a UTF-16 code unit offset to a byte offset within a string.
///
/// UTF-16 uses 2 bytes for most characters but 4 bytes (2 code units) for
/// characters outside the BMP (like emojis).
fn utf16_offset_to_byte_offset(s: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    let mut byte_offset = 0;

    for ch in s.chars() {
        if utf16_count >= utf16_offset {
            break;
        }

        // Count UTF-16 code units for this character
        let utf16_len = ch.len_utf16();
        utf16_count += utf16_len;
        byte_offset += ch.len_utf8();
    }

    byte_offset
}

/// Convert a byte offset to UTF-16 code unit count.
fn byte_offset_to_utf16_offset(s: &str) -> usize {
    s.chars().map(|ch| ch.len_utf16()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_single_line() {
        let mapper = PositionMapper::new("hello world");
        assert_eq!(mapper.line_starts(), &[0]);
    }

    #[test]
    fn test_new_multi_line() {
        let mapper = PositionMapper::new("line 1\nline 2\nline 3");
        assert_eq!(mapper.line_starts(), &[0, 7, 14]);
    }

    #[test]
    fn test_offset_from_position_simple() {
        let mapper = PositionMapper::new("hello\nworld");

        // Start of file
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 0
            }),
            0
        );

        // Middle of first line
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 3
            }),
            3
        );

        // Start of second line
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 1,
                character: 0
            }),
            6
        );

        // Middle of second line
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 1,
                character: 3
            }),
            9
        );
    }

    #[test]
    fn test_position_from_offset_simple() {
        let mapper = PositionMapper::new("hello\nworld");

        // Start of file
        assert_eq!(
            mapper.position_from_offset(0),
            Position {
                line: 0,
                character: 0
            }
        );

        // Middle of first line
        assert_eq!(
            mapper.position_from_offset(3),
            Position {
                line: 0,
                character: 3
            }
        );

        // Start of second line (after newline)
        assert_eq!(
            mapper.position_from_offset(6),
            Position {
                line: 1,
                character: 0
            }
        );

        // Middle of second line
        assert_eq!(
            mapper.position_from_offset(9),
            Position {
                line: 1,
                character: 3
            }
        );
    }

    #[test]
    fn test_roundtrip() {
        let content = "first line\nsecond line\nthird line";
        let mapper = PositionMapper::new(content);

        for offset in 0..content.len() {
            let pos = mapper.position_from_offset(offset);
            let recovered = mapper.offset_from_position(pos);
            assert_eq!(offset, recovered, "Roundtrip failed for offset {}", offset);
        }
    }

    #[test]
    fn test_utf16_emoji() {
        // Emoji (outside BMP) takes 2 UTF-16 code units but 4 UTF-8 bytes
        let content = "a\u{1F600}b"; // a + grinning face emoji + b
        let mapper = PositionMapper::new(content);

        // 'a' is at position 0 (1 UTF-16 code unit, 1 byte)
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 0
            }),
            0
        );

        // Emoji starts at byte 1, UTF-16 position 1
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 1
            }),
            1
        );

        // 'b' is at byte 5 (1 + 4), UTF-16 position 3 (1 + 2)
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 3
            }),
            5
        );

        // Reverse: byte offset 5 should give UTF-16 position 3
        assert_eq!(
            mapper.position_from_offset(5),
            Position {
                line: 0,
                character: 3
            }
        );
    }

    #[test]
    fn test_utf16_multi_byte_chars() {
        // Various multi-byte UTF-8 characters
        let content = "a\u{00E9}b"; // a + e-acute (2 bytes, 1 UTF-16) + b
        let mapper = PositionMapper::new(content);

        // 'a' at byte 0, UTF-16 position 0
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 0
            }),
            0
        );

        // e-acute at byte 1, UTF-16 position 1
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 1
            }),
            1
        );

        // 'b' at byte 3 (1 + 2), UTF-16 position 2 (1 + 1)
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 2
            }),
            3
        );
    }

    #[test]
    fn test_utf16_chinese() {
        // Chinese characters: 3 bytes UTF-8, 1 UTF-16 code unit each
        let content = "\u{4E2D}\u{6587}"; // Chinese: "zhong wen" (2 chars)
        let mapper = PositionMapper::new(content);

        // First character at byte 0, UTF-16 position 0
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 0
            }),
            0
        );

        // Second character at byte 3, UTF-16 position 1
        assert_eq!(
            mapper.offset_from_position(Position {
                line: 0,
                character: 1
            }),
            3
        );

        // End of string at byte 6, UTF-16 position 2
        assert_eq!(
            mapper.position_from_offset(6),
            Position {
                line: 0,
                character: 2
            }
        );
    }

    #[test]
    fn test_span_to_range() {
        let content = "hello\nworld";
        let mapper = PositionMapper::new(content);

        let span = SourceSpan { start: 0, end: 5 }; // "hello"
        let range = mapper.span_to_range(&span);

        assert_eq!(
            range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            range.end,
            Position {
                line: 0,
                character: 5
            }
        );

        // Span across lines
        let span = SourceSpan { start: 0, end: 11 }; // "hello\nworld"
        let range = mapper.span_to_range(&span);

        assert_eq!(
            range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            range.end,
            Position {
                line: 1,
                character: 5
            }
        );
    }

    #[test]
    fn test_range_to_span() {
        let content = "hello\nworld";
        let mapper = PositionMapper::new(content);

        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 5,
            },
        };
        let span = mapper.range_to_span(&range);

        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
    }

    #[test]
    fn test_empty_content() {
        let mapper = PositionMapper::new("");
        assert_eq!(mapper.line_starts(), &[0]);
        assert_eq!(
            mapper.position_from_offset(0),
            Position {
                line: 0,
                character: 0
            }
        );
    }

    #[test]
    fn test_trailing_newline() {
        let content = "hello\n";
        let mapper = PositionMapper::new(content);

        assert_eq!(mapper.line_starts(), &[0, 6]);

        // Position on empty last line
        assert_eq!(
            mapper.position_from_offset(6),
            Position {
                line: 1,
                character: 0
            }
        );
    }
}
