//! Position mapping tests for the Spacetime LSP.
//!
//! Tests conversion between byte offsets and LSP positions,
//! including proper UTF-16 handling for multi-byte characters.

use spacetime::diagnostics::SourceSpan;
use spacetime::lsp::PositionMapper;
use tower_lsp::lsp_types::Position;

// =============================================================================
// Basic Position Mapping Tests
// =============================================================================

#[test]
fn test_single_line_mapping() {
    let content = "hello world";
    let mapper = PositionMapper::new(content);

    // Start of line
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 0
        }),
        0
    );

    // Middle of content
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 6
        }),
        6
    );

    // End of content
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 11
        }),
        11
    );
}

#[test]
fn test_multi_line_mapping() {
    let content = "line 1\nline 2\nline 3";
    let mapper = PositionMapper::new(content);

    // Line starts are at: 0, 7, 14
    assert_eq!(mapper.line_starts(), &[0, 7, 14]);

    // First line
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 0
        }),
        0
    );
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 5
        }),
        5
    );

    // Second line
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 1,
            character: 0
        }),
        7
    );
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 1,
            character: 4
        }),
        11
    );

    // Third line
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 2,
            character: 0
        }),
        14
    );
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 2,
            character: 6
        }),
        20
    );
}

#[test]
fn test_position_from_offset_roundtrip() {
    let content = "first line\nsecond line\nthird line";
    let mapper = PositionMapper::new(content);

    // Test every valid offset
    for offset in 0..content.len() {
        let pos = mapper.position_from_offset(offset);
        let recovered = mapper.offset_from_position(pos);
        assert_eq!(
            offset, recovered,
            "Roundtrip failed for offset {}: got position {:?}, recovered {}",
            offset, pos, recovered
        );
    }
}

// =============================================================================
// UTF-16 Handling Tests
// =============================================================================

#[test]
fn test_utf16_emoji_handling() {
    // Emoji (U+1F600 GRINNING FACE) takes 4 UTF-8 bytes but 2 UTF-16 code units
    let content = "a\u{1F600}b"; // a + emoji + b
    let mapper = PositionMapper::new(content);

    // 'a' is at byte 0, UTF-16 position 0
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
    // e-acute (U+00E9): 2 UTF-8 bytes, 1 UTF-16 code unit
    let content = "caf\u{00E9}"; // "cafe" with accent
    let mapper = PositionMapper::new(content);

    // 'c' at position 0
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 0
        }),
        0
    );

    // 'a' at position 1 (byte 1)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 1
        }),
        1
    );

    // 'f' at position 2 (byte 2)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 2
        }),
        2
    );

    // 'e-acute' at position 3 (byte 3, but 2 bytes long)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 3
        }),
        3
    );

    // End of string at byte 5, position 4
    assert_eq!(
        mapper.position_from_offset(5),
        Position {
            line: 0,
            character: 4
        }
    );
}

#[test]
fn test_utf16_chinese_characters() {
    // Chinese characters: 3 UTF-8 bytes each, 1 UTF-16 code unit each
    let content = "\u{4E2D}\u{6587}\u{6D4B}\u{8BD5}"; // Chinese text
    let mapper = PositionMapper::new(content);

    // First character at byte 0
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 0
        }),
        0
    );

    // Second character at byte 3
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 1
        }),
        3
    );

    // Third character at byte 6
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 2
        }),
        6
    );

    // Fourth character at byte 9
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 3
        }),
        9
    );

    // End at byte 12, position 4
    assert_eq!(
        mapper.position_from_offset(12),
        Position {
            line: 0,
            character: 4
        }
    );
}

#[test]
fn test_utf16_mixed_content() {
    // Mix of ASCII, 2-byte, 3-byte, and 4-byte UTF-8 characters
    let content = "a\u{00E9}\u{4E2D}\u{1F600}z"; // a + e-acute + chinese + emoji + z
    let mapper = PositionMapper::new(content);

    // 'a' at byte 0, pos 0
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 0
        }),
        0
    );

    // 'e-acute' at byte 1, pos 1 (2 bytes, 1 UTF-16)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 1
        }),
        1
    );

    // Chinese at byte 3, pos 2 (3 bytes, 1 UTF-16)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 2
        }),
        3
    );

    // Emoji at byte 6, pos 3 (4 bytes, 2 UTF-16)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 3
        }),
        6
    );

    // 'z' at byte 10, pos 5 (after 2 UTF-16 units for emoji)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 5
        }),
        10
    );

    // Verify reverse mapping
    assert_eq!(
        mapper.position_from_offset(10),
        Position {
            line: 0,
            character: 5
        }
    );
}

#[test]
fn test_utf16_on_multiple_lines() {
    let content = "line 1\n\u{1F600} emoji line\nline 3";
    let mapper = PositionMapper::new(content);

    // Line 0: "line 1" (7 bytes including newline)
    // Line 1: emoji + " emoji line" (starts at byte 7)
    // Line 2: "line 3" (starts at byte 7 + 4 + 11 + 1 = 23)

    // Start of line 1
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 1,
            character: 0
        }),
        7
    );

    // After emoji (4 bytes = 2 UTF-16 code units)
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 1,
            character: 2
        }),
        11
    );

    // Space after emoji
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 1,
            character: 3
        }),
        12
    );
}

// =============================================================================
// Span to Range Conversion Tests
// =============================================================================

#[test]
fn test_span_to_range_single_line() {
    let content = "hello world";
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
}

#[test]
fn test_span_to_range_multi_line() {
    let content = "hello\nworld";
    let mapper = PositionMapper::new(content);

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
fn test_span_to_range_with_utf16() {
    let content = "a\u{1F600}b\nc"; // Line 0: a + emoji + b, Line 1: c
    let mapper = PositionMapper::new(content);

    // Span covering just 'b' (byte 5)
    let span = SourceSpan { start: 5, end: 6 };
    let range = mapper.span_to_range(&span);

    // 'b' is at UTF-16 position 3 (a=1, emoji=2, so b=3)
    assert_eq!(
        range.start,
        Position {
            line: 0,
            character: 3
        }
    );
    assert_eq!(
        range.end,
        Position {
            line: 0,
            character: 4
        }
    );
}

#[test]
fn test_range_to_span_roundtrip() {
    let content = "hello\nworld\ntest";
    let mapper = PositionMapper::new(content);

    let original_span = SourceSpan { start: 6, end: 11 }; // "world"
    let range = mapper.span_to_range(&original_span);
    let recovered_span = mapper.range_to_span(&range);

    assert_eq!(original_span.start, recovered_span.start);
    assert_eq!(original_span.end, recovered_span.end);
}

// =============================================================================
// Edge Cases
// =============================================================================

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
    assert_eq!(
        mapper.offset_from_position(Position {
            line: 0,
            character: 0
        }),
        0
    );
}

#[test]
fn test_only_newlines() {
    let content = "\n\n\n";
    let mapper = PositionMapper::new(content);

    assert_eq!(mapper.line_starts(), &[0, 1, 2, 3]);

    // Each line is empty
    assert_eq!(
        mapper.position_from_offset(0),
        Position {
            line: 0,
            character: 0
        }
    );
    assert_eq!(
        mapper.position_from_offset(1),
        Position {
            line: 1,
            character: 0
        }
    );
    assert_eq!(
        mapper.position_from_offset(2),
        Position {
            line: 2,
            character: 0
        }
    );
    assert_eq!(
        mapper.position_from_offset(3),
        Position {
            line: 3,
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

#[test]
fn test_windows_line_endings_are_content() {
    // Note: We don't specially handle \r\n, the \r is treated as content
    let content = "hello\r\nworld";
    let mapper = PositionMapper::new(content);

    // Line breaks only on \n, so \r is part of line content
    // Line 0: "hello\r" (0-6)
    // Line 1: "world" (7-12)
    assert_eq!(mapper.line_starts(), &[0, 7]);
}

#[test]
fn test_position_past_end_of_file() {
    let content = "hello";
    let mapper = PositionMapper::new(content);

    // Position past end of file should clamp to end
    let offset = mapper.offset_from_position(Position {
        line: 10,
        character: 10,
    });
    assert_eq!(offset, content.len());
}

#[test]
fn test_position_past_end_of_line() {
    let content = "hi\nworld";
    let mapper = PositionMapper::new(content);

    // Requesting position past end of first line (which is 2 chars)
    let offset = mapper.offset_from_position(Position {
        line: 0,
        character: 10,
    });
    // Should be at byte 2 (end of "hi")
    assert_eq!(offset, 2);
}

// =============================================================================
// Spacetime-Specific Content Tests
// =============================================================================

#[test]
fn test_spacetime_directive_positions() {
    let content = r#"@scope(".container") {
    @scroll {
        opacity: 0 -> 1;
    }
}"#;
    let mapper = PositionMapper::new(content);

    // Find position of "@scroll"
    let scroll_start = content.find("@scroll").unwrap();
    let scroll_pos = mapper.position_from_offset(scroll_start);

    // Should be on line 1 (0-indexed), some character into the line
    assert_eq!(scroll_pos.line, 1);
    assert!(scroll_pos.character > 0); // After indentation

    // Roundtrip
    let recovered = mapper.offset_from_position(scroll_pos);
    assert_eq!(recovered, scroll_start);
}

#[test]
fn test_spacetime_property_positions() {
    let content = r#"@scope(".hero") {
    @on &.hover {
        background-color: #ff0000;
        transform: scale(1.1);
    }
}"#;
    let mapper = PositionMapper::new(content);

    // Find position of "transform"
    let transform_start = content.find("transform").unwrap();
    let transform_pos = mapper.position_from_offset(transform_start);

    // Should be on line 3 (0-indexed)
    assert_eq!(transform_pos.line, 3);

    // Verify the range of the property name
    let transform_end = transform_start + "transform".len();
    let range = mapper.span_to_range(&SourceSpan {
        start: transform_start,
        end: transform_end,
    });

    assert_eq!(range.start.line, 3);
    assert_eq!(range.end.line, 3);
    assert_eq!(range.end.character - range.start.character, 9); // "transform" is 9 chars
}
