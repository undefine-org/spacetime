/// Doc Macro — render a Markdown file into an element.
///
/// `@doc(src: "path/to/file.md")` renders the named markdown file's content as
/// HTML into the host element. The path is resolved at BUILD time relative to
/// the site directory; the compiler reads the file's bytes and inlines them, so
/// the rendered output is generated from the very markdown the docs are written
/// in (self-documenting: edit the `.md`, rebuild, the page updates).
///
/// The markdown→HTML conversion runs in the browser via the vendored snarkdown
/// engine, demand-injected only on pages that use `@doc`.
///
/// @example render a guide
/// .doc-body {
///   @doc(src: "docs/testing/GUIDE.md")
/// }
///
/// @example inline markdown (no file)
/// .note {
///   @doc(content: "# Heading\n\nSome **bold** prose.")
/// }

%macro doc {

  %form {
    @doc(
      src: $src:string? = "",
      content: $content:string? = ""
    )
  }

  %binds {
    render-markdown(&self, content: $content)
  }
}
