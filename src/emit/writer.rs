//! Source Map Writer
//!
//! A writer abstraction that tracks output positions during string generation,
//! enabling source map generation. Follows the pattern used by swc's emitter.
//!
//! When source maps are disabled, this has zero overhead - just string concatenation.
//! When enabled, it tracks line/column positions and records mappings.

use super::sourcemap::{SourceMapBuilder, SourceSpan};

/// Writer that tracks output position for source map generation.
///
/// This is the core abstraction for source-map-aware code emission.
/// It wraps string building with position tracking and mapping recording.
#[derive(Debug)]
pub struct SourceMapWriter {
    /// The output string being built
    output: String,
    /// Current line (0-based)
    line: u32,
    /// Current column (0-based)
    column: u32,
    /// Source map builder (None if source maps disabled)
    source_map: Option<SourceMapBuilder>,
}

impl SourceMapWriter {
    /// Create a new source map writer.
    ///
    /// # Arguments
    /// * `output_name` - Name of the output file (e.g., "main.js")
    /// * `enabled` - Whether to generate source maps
    pub fn new(output_name: &str, enabled: bool) -> Self {
        Self {
            output: String::new(),
            line: 0,
            column: 0,
            source_map: if enabled {
                Some(SourceMapBuilder::new(output_name))
            } else {
                None
            },
        }
    }

    /// Create a writer with source maps disabled.
    pub fn without_source_map() -> Self {
        Self {
            output: String::new(),
            line: 0,
            column: 0,
            source_map: None,
        }
    }

    /// Add a source file and return its index.
    ///
    /// # Arguments
    /// * `path` - Path to the source file (e.g., "src/main.st")
    /// * `content` - Optional source content to embed in the source map
    ///
    /// Returns the source index (0 if source maps disabled).
    pub fn add_source(&mut self, path: &str, content: Option<&str>) -> usize {
        self.source_map
            .as_mut()
            .map(|sm| sm.add_source(path, content))
            .unwrap_or(0)
    }

    /// Set the source root prefix for all source paths.
    pub fn set_source_root(&mut self, root: &str) {
        if let Some(ref mut sm) = self.source_map {
            sm.set_source_root(root);
        }
    }

    /// Write a string to the output, tracking position.
    ///
    /// This is the core method - it updates line/column as it writes.
    pub fn write(&mut self, s: &str) {
        for c in s.chars() {
            self.output.push(c);
            if c == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }

    /// Write a string with a source mapping.
    ///
    /// Records a mapping from the current output position to the source span,
    /// then writes the string.
    pub fn write_mapped(&mut self, s: &str, span: &SourceSpan) {
        self.mark(span);
        self.write(s);
    }

    /// Write a string with an optional source mapping.
    ///
    /// Convenience method that handles Option<&SourceSpan>.
    pub fn write_with_span(&mut self, s: &str, span: Option<&SourceSpan>) {
        if let Some(sp) = span {
            self.mark(sp);
        }
        self.write(s);
    }

    /// Mark the current output position as mapping to a source span.
    ///
    /// Call this immediately before writing the corresponding output.
    pub fn mark(&mut self, span: &SourceSpan) {
        if let Some(ref mut sm) = self.source_map {
            sm.add_mapping_from_span(self.line, self.column, span);
        }
    }

    /// Mark a specific output position as mapping to a source span.
    pub fn mark_at(&mut self, gen_line: u32, gen_col: u32, span: &SourceSpan) {
        if let Some(ref mut sm) = self.source_map {
            sm.add_mapping_from_span(gen_line, gen_col, span);
        }
    }

    /// Add a raw mapping (generated position to source position).
    pub fn add_mapping(
        &mut self,
        gen_line: u32,
        gen_col: u32,
        src_idx: usize,
        src_line: u32,
        src_col: u32,
    ) {
        if let Some(ref mut sm) = self.source_map {
            sm.add_mapping(gen_line, gen_col, src_idx, src_line, src_col);
        }
    }

    /// Get the current line (0-based).
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Get the current column (0-based).
    pub fn column(&self) -> u32 {
        self.column
    }

    /// Get the current output length in bytes.
    pub fn len(&self) -> usize {
        self.output.len()
    }

    /// Check if output is empty.
    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }

    /// Get a reference to the output string.
    pub fn as_str(&self) -> &str {
        &self.output
    }

    /// Check if source maps are enabled.
    pub fn has_source_map(&self) -> bool {
        self.source_map.is_some()
    }

    /// Finish writing and return the output and optional source map.
    ///
    /// Returns (output_string, source_map_json).
    pub fn finish(self) -> (String, Option<String>) {
        let source_map = self.source_map.map(|sm| sm.build());
        (self.output, source_map)
    }

    /// Finish writing and return the output with pretty-printed source map.
    pub fn finish_pretty(self) -> (String, Option<String>) {
        let source_map = self.source_map.map(|sm| sm.build_pretty());
        (self.output, source_map)
    }

    /// Take the output string, leaving an empty string in its place.
    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output)
    }
}

/// Output with optional source map.
#[derive(Debug, Clone, Default)]
pub struct EmitOutput {
    /// The emitted code
    pub code: String,
    /// The source map JSON (if generated)
    pub source_map: Option<String>,
}

impl EmitOutput {
    /// Create a new emit output without source map.
    pub fn new(code: String) -> Self {
        Self {
            code,
            source_map: None,
        }
    }

    /// Create a new emit output with source map.
    pub fn with_source_map(code: String, source_map: String) -> Self {
        Self {
            code,
            source_map: Some(source_map),
        }
    }

    /// Create from writer finish result.
    pub fn from_writer(result: (String, Option<String>)) -> Self {
        Self {
            code: result.0,
            source_map: result.1,
        }
    }
}

// ============================================================================
// Inline Source Map Utilities
// ============================================================================

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Generate an inline source map data URL for JavaScript.
///
/// Returns the `//# sourceMappingURL=data:...` comment to append to JS output.
pub fn inline_js_source_map(source_map_json: &str) -> String {
    let encoded = BASE64.encode(source_map_json.as_bytes());
    format!(
        "\n//# sourceMappingURL=data:application/json;base64,{}",
        encoded
    )
}

/// Generate an inline source map data URL for CSS.
///
/// Returns the `/*# sourceMappingURL=data:... */` comment to append to CSS output.
pub fn inline_css_source_map(source_map_json: &str) -> String {
    let encoded = BASE64.encode(source_map_json.as_bytes());
    format!(
        "\n/*# sourceMappingURL=data:application/json;base64,{} */",
        encoded
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_tracks_position() {
        let mut writer = SourceMapWriter::without_source_map();

        writer.write("hello");
        assert_eq!(writer.line(), 0);
        assert_eq!(writer.column(), 5);

        writer.write("\n");
        assert_eq!(writer.line(), 1);
        assert_eq!(writer.column(), 0);

        writer.write("world");
        assert_eq!(writer.line(), 1);
        assert_eq!(writer.column(), 5);
    }

    #[test]
    fn test_write_multiline() {
        let mut writer = SourceMapWriter::without_source_map();

        writer.write("line1\nline2\nline3");
        assert_eq!(writer.line(), 2);
        assert_eq!(writer.column(), 5);
        assert_eq!(writer.as_str(), "line1\nline2\nline3");
    }

    #[test]
    fn test_source_map_disabled() {
        let mut writer = SourceMapWriter::new("test.js", false);
        writer.write("const x = 1;");

        let (code, source_map) = writer.finish();
        assert_eq!(code, "const x = 1;");
        assert!(source_map.is_none());
    }

    #[test]
    fn test_source_map_enabled() {
        let mut writer = SourceMapWriter::new("test.js", true);
        writer.add_source("test.st", Some("$x: 1"));

        let span = SourceSpan::new(0, 0, 0, 0, 5);
        writer.write_mapped("const x = 1;", &span);

        let (code, source_map) = writer.finish();
        assert_eq!(code, "const x = 1;");
        assert!(source_map.is_some());

        let map = source_map.unwrap();
        assert!(map.contains("\"version\":3"));
        assert!(map.contains("\"file\":\"test.js\""));
        assert!(map.contains("\"sources\":[\"test.st\"]"));
    }

    #[test]
    fn test_add_source() {
        let mut writer = SourceMapWriter::new("out.js", true);

        let idx0 = writer.add_source("a.st", None);
        let idx1 = writer.add_source("b.st", None);

        assert_eq!(idx0, 0);
        assert_eq!(idx1, 1);
    }

    #[test]
    fn test_write_with_optional_span() {
        let mut writer = SourceMapWriter::new("test.js", true);
        writer.add_source("test.st", None);

        // Write without span
        writer.write_with_span("// comment\n", None);

        // Write with span
        let span = SourceSpan::new(0, 0, 0, 0, 5);
        writer.write_with_span("const x = 1;", Some(&span));

        let (code, _) = writer.finish();
        assert_eq!(code, "// comment\nconst x = 1;");
    }

    #[test]
    fn test_emit_output_from_writer() {
        let mut writer = SourceMapWriter::new("test.js", true);
        writer.add_source("test.st", None);
        writer.write("test");

        let output = EmitOutput::from_writer(writer.finish());
        assert_eq!(output.code, "test");
        assert!(output.source_map.is_some());
    }

    #[test]
    fn test_is_empty() {
        let writer = SourceMapWriter::without_source_map();
        assert!(writer.is_empty());

        let mut writer2 = SourceMapWriter::without_source_map();
        writer2.write("x");
        assert!(!writer2.is_empty());
    }

    #[test]
    fn test_take_output() {
        let mut writer = SourceMapWriter::without_source_map();
        writer.write("hello");

        let output = writer.take_output();
        assert_eq!(output, "hello");
        assert!(writer.is_empty());
    }
}
