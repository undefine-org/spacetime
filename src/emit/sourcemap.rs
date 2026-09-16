//! Source Map Generation
//!
//! Generates V3 source maps to enable debugging compiled output back to `.st` source files.
//! Source maps encode position mappings using Base64 VLQ encoding.
//!
//! ## Source Map V3 Format
//!
//! ```json
//! {
//!   "version": 3,
//!   "file": "output.js",
//!   "sourceRoot": "",
//!   "sources": ["input.st"],
//!   "sourcesContent": ["..."],
//!   "mappings": "AAAA,SAAS..."
//! }
//! ```
//!
//! ## VLQ Encoding
//!
//! Each segment in the mappings encodes (using relative values):
//! - Generated column
//! - Source file index
//! - Source line
//! - Source column

use serde::Serialize;

/// Source span representing a range in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceSpan {
    /// Source file index (into sources array)
    pub source_index: usize,
    /// 0-based start line in source
    pub start_line: u32,
    /// 0-based start column in source
    pub start_column: u32,
    /// 0-based end line in source
    pub end_line: u32,
    /// 0-based end column in source
    pub end_column: u32,
}

impl SourceSpan {
    /// Create a new source span
    pub fn new(
        source_index: usize,
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    ) -> Self {
        Self {
            source_index,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Create a span from byte offsets in source text
    pub fn from_byte_offsets(source_index: usize, source: &str, start: usize, end: usize) -> Self {
        let (start_line, start_column) = byte_offset_to_line_col(source, start);
        let (end_line, end_column) = byte_offset_to_line_col(source, end);
        Self {
            source_index,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}

/// Convert byte offset to 0-based (line, column)
fn byte_offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current_offset = 0usize;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

/// A single mapping from generated to source location
#[derive(Debug, Clone, Copy)]
pub struct Mapping {
    /// 0-based line in generated output
    pub generated_line: u32,
    /// 0-based column in generated output
    pub generated_column: u32,
    /// 0-based line in source
    pub source_line: u32,
    /// 0-based column in source
    pub source_column: u32,
    /// Index into sources array
    pub source_index: usize,
}

impl Mapping {
    /// Create a new mapping
    pub fn new(
        generated_line: u32,
        generated_column: u32,
        source_index: usize,
        source_line: u32,
        source_column: u32,
    ) -> Self {
        Self {
            generated_line,
            generated_column,
            source_line,
            source_column,
            source_index,
        }
    }
}

/// Source map builder that generates V3 source maps
#[derive(Debug, Clone)]
pub struct SourceMapBuilder {
    /// Output file name
    file: String,
    /// Optional source root prefix
    source_root: Option<String>,
    /// List of source file paths
    sources: Vec<String>,
    /// Optional source content for each source
    sources_content: Vec<Option<String>>,
    /// All mappings
    mappings: Vec<Mapping>,
}

/// Serializable V3 source map
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceMapV3 {
    version: u8,
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_root: Option<String>,
    sources: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sources_content: Vec<Option<String>>,
    mappings: String,
}

impl SourceMapBuilder {
    /// Create a new source map builder
    pub fn new(file: &str) -> Self {
        Self {
            file: file.to_string(),
            source_root: None,
            sources: Vec::new(),
            sources_content: Vec::new(),
            mappings: Vec::new(),
        }
    }

    /// Set the source root (prefix for all source paths)
    pub fn set_source_root(&mut self, root: &str) {
        self.source_root = Some(root.to_string());
    }

    /// Add a source file and return its index
    pub fn add_source(&mut self, path: &str, content: Option<&str>) -> usize {
        let index = self.sources.len();
        self.sources.push(path.to_string());
        self.sources_content.push(content.map(|s| s.to_string()));
        index
    }

    /// Add a mapping from generated position to source position
    pub fn add_mapping(
        &mut self,
        gen_line: u32,
        gen_col: u32,
        src_idx: usize,
        src_line: u32,
        src_col: u32,
    ) {
        self.mappings
            .push(Mapping::new(gen_line, gen_col, src_idx, src_line, src_col));
    }

    /// Add a mapping from a source span
    pub fn add_mapping_from_span(&mut self, gen_line: u32, gen_col: u32, span: &SourceSpan) {
        self.add_mapping(
            gen_line,
            gen_col,
            span.source_index,
            span.start_line,
            span.start_column,
        );
    }

    /// Build the source map as a JSON string
    pub fn build(&self) -> String {
        let mappings = self.encode_mappings();

        let source_map = SourceMapV3 {
            version: 3,
            file: self.file.clone(),
            source_root: self.source_root.clone(),
            sources: self.sources.clone(),
            sources_content: self.sources_content.clone(),
            mappings,
        };

        serde_json::to_string(&source_map).unwrap_or_else(|_| "{}".to_string())
    }

    /// Build the source map as a pretty-printed JSON string
    pub fn build_pretty(&self) -> String {
        let mappings = self.encode_mappings();

        let source_map = SourceMapV3 {
            version: 3,
            file: self.file.clone(),
            source_root: self.source_root.clone(),
            sources: self.sources.clone(),
            sources_content: self.sources_content.clone(),
            mappings,
        };

        serde_json::to_string_pretty(&source_map).unwrap_or_else(|_| "{}".to_string())
    }

    /// Encode all mappings to VLQ string
    fn encode_mappings(&self) -> String {
        if self.mappings.is_empty() {
            return String::new();
        }

        // Sort mappings by generated line, then column
        let mut sorted_mappings = self.mappings.clone();
        sorted_mappings.sort_by(|a, b| {
            a.generated_line
                .cmp(&b.generated_line)
                .then(a.generated_column.cmp(&b.generated_column))
        });

        let mut result = String::new();
        let mut prev_gen_line = 0u32;
        let mut prev_gen_col = 0i32;
        let mut prev_source_idx = 0i32;
        let mut prev_source_line = 0i32;
        let mut prev_source_col = 0i32;

        for mapping in &sorted_mappings {
            // Add semicolons for line breaks
            while prev_gen_line < mapping.generated_line {
                result.push(';');
                prev_gen_line += 1;
                prev_gen_col = 0; // Reset column at new line
            }

            // Add comma separator within same line (except for first segment on line)
            if !result.is_empty() && !result.ends_with(';') {
                result.push(',');
            }

            // Encode the segment: generated column, source index, source line, source column
            let gen_col_delta = mapping.generated_column as i32 - prev_gen_col;
            let source_idx_delta = mapping.source_index as i32 - prev_source_idx;
            let source_line_delta = mapping.source_line as i32 - prev_source_line;
            let source_col_delta = mapping.source_column as i32 - prev_source_col;

            result.push_str(&encode_vlq(gen_col_delta));
            result.push_str(&encode_vlq(source_idx_delta));
            result.push_str(&encode_vlq(source_line_delta));
            result.push_str(&encode_vlq(source_col_delta));

            // Update previous values
            prev_gen_col = mapping.generated_column as i32;
            prev_source_idx = mapping.source_index as i32;
            prev_source_line = mapping.source_line as i32;
            prev_source_col = mapping.source_column as i32;
        }

        result
    }
}

/// Base64 alphabet for VLQ encoding
const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode an integer value to Base64 VLQ
///
/// VLQ uses a continuation bit (bit 5) to indicate if more digits follow.
/// The sign is stored in bit 0 of the first digit.
pub fn encode_vlq(value: i32) -> String {
    let mut result = String::new();

    // Convert to unsigned with sign in LSB
    let mut vlq: u32 = if value < 0 {
        (((-value) << 1) | 1) as u32
    } else {
        (value << 1) as u32
    };

    loop {
        // Take the lowest 5 bits
        let mut digit = (vlq & 0x1F) as u8;
        vlq >>= 5;

        // Set continuation bit if there's more data
        if vlq > 0 {
            digit |= 0x20;
        }

        result.push(BASE64_CHARS[digit as usize] as char);

        if vlq == 0 {
            break;
        }
    }

    result
}

/// Decode a Base64 VLQ value (for testing/verification)
pub fn decode_vlq(input: &str) -> Result<(i32, usize), String> {
    let mut result = 0u32;
    let mut shift = 0;
    let mut chars_consumed = 0;

    for ch in input.chars() {
        let digit = BASE64_CHARS
            .iter()
            .position(|&c| c as char == ch)
            .ok_or_else(|| format!("Invalid Base64 character: {}", ch))?;

        let digit = digit as u32;
        result |= (digit & 0x1F) << shift;
        shift += 5;
        chars_consumed += 1;

        // Check continuation bit
        if (digit & 0x20) == 0 {
            break;
        }
    }

    // Extract sign from LSB
    let is_negative = (result & 1) == 1;
    let value = (result >> 1) as i32;
    let value = if is_negative { -value } else { value };

    Ok((value, chars_consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_vlq_zero() {
        assert_eq!(encode_vlq(0), "A");
    }

    #[test]
    fn test_encode_vlq_positive() {
        assert_eq!(encode_vlq(1), "C");
        assert_eq!(encode_vlq(2), "E");
        assert_eq!(encode_vlq(3), "G");
        assert_eq!(encode_vlq(15), "e");
        assert_eq!(encode_vlq(16), "gB");
        assert_eq!(encode_vlq(100), "oG");
    }

    #[test]
    fn test_encode_vlq_negative() {
        assert_eq!(encode_vlq(-1), "D");
        assert_eq!(encode_vlq(-2), "F");
        assert_eq!(encode_vlq(-3), "H");
        assert_eq!(encode_vlq(-15), "f");
        assert_eq!(encode_vlq(-16), "hB");
    }

    #[test]
    fn test_decode_vlq() {
        // Test roundtrip
        for value in [-1000, -100, -16, -15, -1, 0, 1, 15, 16, 100, 1000] {
            let encoded = encode_vlq(value);
            let (decoded, _) = decode_vlq(&encoded).unwrap();
            assert_eq!(
                decoded, value,
                "Failed for value {}: encoded as '{}'",
                value, encoded
            );
        }
    }

    #[test]
    fn test_byte_offset_to_line_col() {
        let source = "line1\nline2\nline3";

        // First character
        assert_eq!(byte_offset_to_line_col(source, 0), (0, 0));

        // End of first line
        assert_eq!(byte_offset_to_line_col(source, 5), (0, 5));

        // Start of second line
        assert_eq!(byte_offset_to_line_col(source, 6), (1, 0));

        // Middle of second line
        assert_eq!(byte_offset_to_line_col(source, 8), (1, 2));

        // Start of third line
        assert_eq!(byte_offset_to_line_col(source, 12), (2, 0));
    }

    #[test]
    fn test_source_span_from_byte_offsets() {
        let source = "line1\nline2\nline3";
        let span = SourceSpan::from_byte_offsets(0, source, 6, 11);

        assert_eq!(span.source_index, 0);
        assert_eq!(span.start_line, 1);
        assert_eq!(span.start_column, 0);
        assert_eq!(span.end_line, 1);
        assert_eq!(span.end_column, 5);
    }

    #[test]
    fn test_source_map_builder_basic() {
        let mut builder = SourceMapBuilder::new("output.js");
        let src_idx = builder.add_source("input.st", Some(".element { color: red; }"));

        // Map generated line 0, col 0 to source line 0, col 0
        builder.add_mapping(0, 0, src_idx, 0, 0);

        let json = builder.build();
        assert!(json.contains("\"version\":3"));
        assert!(json.contains("\"file\":\"output.js\""));
        assert!(json.contains("\"sources\":[\"input.st\"]"));
        assert!(json.contains("\"mappings\":"));
    }

    #[test]
    fn test_source_map_builder_multiple_mappings() {
        let mut builder = SourceMapBuilder::new("output.css");
        let src_idx = builder.add_source("styles.st", None);

        // Multiple mappings on same line
        builder.add_mapping(0, 0, src_idx, 0, 0);
        builder.add_mapping(0, 10, src_idx, 0, 5);

        // Mapping on next line
        builder.add_mapping(1, 0, src_idx, 1, 0);

        let json = builder.build();
        let mappings_start = json.find("\"mappings\":\"").unwrap() + 12;
        let mappings_end = json[mappings_start..].find('"').unwrap() + mappings_start;
        let mappings = &json[mappings_start..mappings_end];

        // Should have semicolon between lines and comma between segments on same line
        assert!(
            mappings.contains(','),
            "Expected comma separator, got: {}",
            mappings
        );
        assert!(
            mappings.contains(';'),
            "Expected semicolon separator, got: {}",
            mappings
        );
    }

    #[test]
    fn test_source_map_builder_with_source_root() {
        let mut builder = SourceMapBuilder::new("out.js");
        builder.set_source_root("/src/");
        builder.add_source("module.st", None);

        let json = builder.build();
        assert!(json.contains("\"sourceRoot\":\"/src/\""));
    }

    #[test]
    fn test_source_map_builder_multiple_sources() {
        let mut builder = SourceMapBuilder::new("bundle.js");
        let idx0 = builder.add_source("a.st", Some("// file a"));
        let idx1 = builder.add_source("b.st", Some("// file b"));

        builder.add_mapping(0, 0, idx0, 0, 0);
        builder.add_mapping(1, 0, idx1, 0, 0);

        let json = builder.build();
        assert!(json.contains("\"sources\":[\"a.st\",\"b.st\"]"));
        assert!(json.contains("\"sourcesContent\":[\"// file a\",\"// file b\"]"));
    }

    #[test]
    fn test_source_map_empty_mappings() {
        let builder = SourceMapBuilder::new("empty.js");
        let json = builder.build();
        assert!(json.contains("\"mappings\":\"\""));
    }

    #[test]
    fn test_mapping_struct() {
        let mapping = Mapping::new(5, 10, 0, 3, 7);
        assert_eq!(mapping.generated_line, 5);
        assert_eq!(mapping.generated_column, 10);
        assert_eq!(mapping.source_index, 0);
        assert_eq!(mapping.source_line, 3);
        assert_eq!(mapping.source_column, 7);
    }

    #[test]
    fn test_add_mapping_from_span() {
        let mut builder = SourceMapBuilder::new("test.js");
        builder.add_source("test.st", None);

        let span = SourceSpan::new(0, 5, 10, 5, 20);
        builder.add_mapping_from_span(0, 0, &span);

        let json = builder.build();
        assert!(json.contains("\"mappings\":"));
    }

    #[test]
    fn test_vlq_large_values() {
        // Test larger values that require multiple characters
        let large_value = 10000;
        let encoded = encode_vlq(large_value);
        let (decoded, _) = decode_vlq(&encoded).unwrap();
        assert_eq!(decoded, large_value);

        let large_negative = -10000;
        let encoded_neg = encode_vlq(large_negative);
        let (decoded_neg, _) = decode_vlq(&encoded_neg).unwrap();
        assert_eq!(decoded_neg, large_negative);
    }
}
