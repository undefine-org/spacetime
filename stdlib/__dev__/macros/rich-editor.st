/// Dev Rich Editor Macro
///
/// Composes the rich text toolbar and media picker primitives into a
/// single dev-mode macro. Provides inline formatting controls (bold,
/// italic, link, code) and image replacement via upload.
///
/// Dependency: Requires dev-editable to be active (from dev-editor macro)
///
/// @example
/// @rich-editor

%macro rich-editor {
  %scope file
  %order 51

  %form {
    @rich-editor
  }

  // FEAT-100: the execCommand-based floating toolbar (dev-rich-toolbar) is GONE.
  // Structured rich text is now the @editable surface (AST model, no execCommand,
  // no contenteditable innerHTML reads) — used by the admin richtext field and
  // available on any page via `@import "stdlib/editable"`. The dev rich-editor
  // keeps only the media picker; inline formatting moved to the structured editor.
  %binds {
    dev-hooks()
    dev-media-picker()
  }
}
