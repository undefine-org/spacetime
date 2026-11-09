/// Editable Mark + Block Macros (PLAN-031 / FEAT-097)
///
/// An `@editable-mark` defines an INLINE formatting span for an `@editable`
/// rich-text surface; an `@editable-block` defines a BLOCK-level container.
///
/// The core insight: **a mark IS a template** whose first parameter is the
/// selection (`&sel`) and whose body is the HTML *projection* of that mark. So
/// these macros ride the existing template machinery (`register-template` builds
/// the factory; backtick holes + `&sel` element-substitution + typed params all
/// work unchanged) and add two things on top:
///   1. a `%registers editable-mark/editable-block` record, making the construct
///      compiler-visible so the build can emit `editable-schema.json` (the editor
///      reads which marks exist, what HTML they project, which typed attrs they
///      carry). The allowlist FALLS OUT of registered constructs — never a
///      hand-maintained Rust enum (AGENTS self-describing rule).
///   2. runtime registration into `ST.editable.marks` / `ST.editable.blocks`.
///
/// @example inline marks
/// @editable-mark &bold(&sel) { <strong>`&sel`</strong> }
/// @editable-mark &italic(&sel) { <em>`&sel`</em> }
/// @editable-mark &link(&sel, $href url) {
///   <a href="`$href`">`&sel`</a>
/// }
///
/// @example block containers
/// @editable-block &h2(&sel) { <h2>`&sel`</h2> }
/// @editable-block &figure(&sel, $src url, $alt string = "") {
///   <figure>
///     <img src="`$src`" alt="`$alt`">
///     <figcaption>`&sel`</figcaption>
///   </figure>
/// }
///
/// ## Parameters
/// - `&sel`      — the selection (an element param; its value is the selected
///                 content, inserted whole; a multi-node selection flattens).
/// - `$name type`— a typed FREE param (FEAT-096 space form). Persisted into the
///                 AST node's attrs, type-validated (`url` ⇒ `javascript:` is a
///                 type error), and surfaced as an inline-edit widget.
///
/// ## Invertibility
/// A mark/block whose projection is a single root, with each attr a literal or a
/// single typed-param hole, and exactly one `&sel`, is INVERTIBLE — usable for
/// paste-lift (the backward direction of the projection lens, FEAT-101). The
/// compiler reports non-invertible projections; they still render forward.

// FEAT-103: an OPTIONAL `( shortcut: $s:string ; )?` head-line rides the body-group
// PEG — a parenthesized group at the head of the body region matches via the SAME
// machinery as %capture_type, ahead of the greedy `$body:component_body`. The two no
// longer fight over the head bytes: the group consumes the metadata line, the body
// capture scans the projection from the yielded boundary. `$shortcut` surfaces as a
// top-level capture (merged from the group) into %registers/%binds. Absent → None.

%macro editable-mark {
  %scope file

  %form {
    @editable-mark &$name:ident($params:param_list) {
      ( "shortcut" ":" $shortcut:string ";"? )?
      $body:component_body
    }
  }

  // Compiler-visible record: drives editable-schema.json emission (FEAT-099)
  // and the per-mark invertibility check. The schema is a projection of THIS
  // registered construct — add a mark, its schema appears automatically.
  %registers editable-mark($name) {
    params: $params
    body: $body
    shortcut: $shortcut
  }

  // A mark IS a template: build the factory via the existing primitive so
  // backtick holes, `&sel` flattening, and typed params all work unchanged.
  %binds {
    register-template(
      name: $name,
      body: $body,
      params: $params
    ) -> { $factory }
  }
}

%macro editable-block {
  %scope file

  %form {
    @editable-block &$name:ident($params:param_list) {
      ( "shortcut" ":" $shortcut:string ";"? )?
      $body:component_body
    }
  }

  %registers editable-block($name) {
    params: $params
    body: $body
    shortcut: $shortcut
  }

  %binds {
    register-template(
      name: $name,
      body: $body,
      params: $params
    ) -> { $factory }
  }
}

/// Editable Surface Macro (PLAN-031 / FEAT-098)
///
/// `@editable` mounts a structured rich-text surface inside a selector scope. It
/// binds a persisted value path (an AST object) and lists the marks + blocks the
/// surface accepts. The surface holds the document AST, projects it to the host,
/// intercepts beforeinput, and yields editor-state signals
/// (`$doc $sel $activeMarks $blockType $empty`) so toolbars are ordinary bindings.
///
/// @example
/// @import "stdlib/editable"
/// .post-body {
///   @editable(bind: $post.body, marks: ["bold","italic","link"], blocks: ["p","h2"]);
/// }
///
/// marks/blocks are arrays of registered mark/block NAMES (see @editable-mark /
/// @editable-block). The schema descriptor is resolved at runtime from
/// ST.editable.* (populated from editable-schema.json, FEAT-099).
%macro editable {
  %scope selector
  %order 200

  %form {
    @editable(bind: $bind:expr, marks: $marks:expr? = [], blocks: $blocks:expr? = []);
  }

  %binds {
    editable-surface(
      &self,
      bind: $bind,
      marks: $marks,
      blocks: $blocks
    ) -> { $doc, $sel, $activeMarks, $blockType, $empty }
  }
}

/// `@richtext` — the PUBLISHED rich-text surface (editable-everything default flow).
///
/// Same structured surface as `@editable`, but EDIT-MODE-GATED: the node renders
/// as clean read-only HTML (the projection IS the final markup, so a blog reads as
/// a blog) and upgrades to an in-place editor only while `html[data-st-edit-mode]`
/// is set (the dev edit-mode toggle, `@edit-toggle`). The document AST stays truth
/// in both states; toggling only flips `contenteditable`. Persistence rides the
/// host's bound `@on input` exactly like `@editable` (EditJson with the doc AST).
///
/// Use `@editable` for always-on form fields (admin drawer); use `@richtext` for
/// content on a page you want to read normally and edit on demand.
///
/// @example
/// article {
///   @richtext(bind: $post.body, marks: ["bold","italic","link"], blocks: ["p","h2","blockquote"]);
/// }
%macro richtext {
  %scope selector
  %order 200

  %form {
    @richtext(bind: $bind:expr, marks: $marks:expr? = [], blocks: $blocks:expr? = []);
  }

  %binds {
    editable-surface(
      &self,
      bind: $bind,
      marks: $marks,
      blocks: $blocks,
      gate: "edit-mode"
    ) -> { $doc, $sel, $activeMarks, $blockType, $empty }
  }
}
