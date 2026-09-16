/// Markdown Module Tests: @doc (snarkdown-backed rendering)
///
/// NOTE: these .test.st assertions do not yet GATE — blocked on BUG-051 (nested
/// directives inside a test/@fixture body do not execute, so the @doc in the
/// fixture's selector block never runs and snarkdown is never injected). This is
/// the SAME limitation documented in stdlib/text/tests/measure.test.st. Until
/// BUG-051 lands, the md module is verified at the emit level by Rust unit tests
/// in src/compiler.rs:
///   - doc_src_inlines_markdown_file_and_renders_via_snarkdown
///   - doc_src_missing_file_emits_visible_error_not_blank
/// and end-to-end by the demos/spacetime-docs reader (6 docs rendered live).
/// These files gate for real once BUG-051 is fixed.
///
/// Verifies that the `md` module renders Markdown to HTML in a live DOM:
///   - importing the module + using @doc ships the vendored snarkdown engine
///   - @doc(content: …) turns markdown into real HTML elements (h1, strong, code)

@import "stdlib/testing/test"
@import "stdlib/md"

// =============================================================================
// Vendored engine availability (demand-driven emission)
// =============================================================================

@test "snarkdown global is injected when @doc is used" {
  @fixture {
    <div class="md-avail" data-doc-avail></div>
    [data-doc-avail] { @doc(content: "# Hi") }
  }
  @assert (typeof snarkdown === 'function')
    "vendored snarkdown engine should be present on the page"
}

// =============================================================================
// Markdown renders to real HTML elements
// =============================================================================

@test "@doc renders a heading from markdown" {
  @fixture {
    <div class="md-h1" data-doc-h1></div>
    [data-doc-h1] { @doc(content: "# Inlined Heading") }
  }
  @then [data-doc-h1] h1 should exist
  @then [data-doc-h1] should have_text "Inlined Heading"
}

@test "@doc renders inline marks (strong) from markdown" {
  @fixture {
    <div class="md-inline" data-doc-inline></div>
    [data-doc-inline] { @doc(content: "Some **bold** text here.") }
  }
  @then [data-doc-inline] strong should exist
}
