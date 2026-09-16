/// CMS Display-Hint Macro  (Spacetime CMS — admin module)
///
/// Declares how the local content admin (/__spacetime/admin/) should DISPLAY a
/// @type's entries: which field is the row title, subtitle, thumbnail, and which
/// string[] supplies category chips. These are *display* hints only — they
/// produce NO runtime JS/CSS (no %binds), exactly like @type. The dev server's
/// /__spacetime/dev/types.json handler reads these FormMatches and merges them
/// with convention-based inference (W1) into a per-type `display` object.
///
/// INTERIM FORM (V1): file-scope, takes the target type as a paren argument:
///
///   @type Talent {
///     id: string; slug: string; name: string;
///     headline: string; image: string; categories: string[];
///   }
///   @cms(Talent) {
///     title: name;
///     subtitle: headline;
///     thumbnail: image;
///     chips: categories;
///   }
///
/// WHY file-scope-with-arg instead of nested inside @type:
///   The originally-specified nested form
///       @type T { @cms { ... } id: string; ... }
///   does NOT parse today: @type captures its whole body as `$fields:properties`,
///   so a nested @cms directive corrupts the field capture (Talent ends up with
///   zero fields). The nested `@type.cms`-style namespaced form requires two
///   pieces of language work tracked as follow-ups:
///     - FUP-macro-body-scope  (a macro valid only inside another macro's body,
///                              with rich placement errors)
///     - FUP-macro-namespaces  (the symbol/grammar for nesting a macro under a
///                              parent, e.g. @type.cms)
///   Until those land, @cms(TypeName) is the interim, fully-parsing surface.
///   It is a real metasystem macro (no Rust special-casing of placement).
///
/// CONVENTION FALLBACK: @cms is OPTIONAL. With no @cms at all, the admin infers
/// display roles from field names/types (name|title→title, headline|subtitle→
/// subtitle, image|photo|url→thumbnail, categories|tags→chips). @cms only
/// OVERRIDES specific roles; omitted roles still fall back to convention.

%macro cms {
  %scope file
  %order 51

  %form {
    @cms($type:ident) {
      $hints:properties
    }
  }

  %registers cms($type) {
    $hints
  }
}
