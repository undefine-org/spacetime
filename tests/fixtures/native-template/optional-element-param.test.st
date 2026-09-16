// BUG-165 regression: an optional ELEMENT param (`&footer?`, `&actions?`) in a
// @template header must parse (CST-level: parse_element_ref must consume the
// trailing `?`/`*`/`+` modifier itself, mirroring parse_variable_ref's `$name?`
// handling — before this fix, the bare QUESTION token leaked into
// parse_arg_value's postfix loop and was mis-read as a ternary operator,
// failing with "expected COLON, found R_PAREN"). Also proves the documented
// runtime contract (stdlib/macros/template.st "Optional Parameters"): an
// omitted `&param?` renders nothing (undefined -> @if false), a provided one
// fills the slot.
//
// STATUS — this test asserts TWO separable things, and only one of them is a
// compiler contract that holds today:
//
//   1. the PARSE contract (BUG-165): `&actions?` in a template header parses.
//      That is the fix this file was written for, and it works. It is now
//      gated separately, below, so it cannot be masked by (2).
//   2. the RUNTIME contract: `@if &actions { … }` renders conditionally. That
//      does NOT work — `@if` has no implementation at all (BUG-362): no
//      `%emit` block, no primitive, `E0956: unknown primitive: conditional`
//      at page level, and nothing rendered even for a literal `@if(true)`.
//
// So this test is RED for a feature that was never built, not for a
// regression. It stays red on purpose — deleting it would erase the only
// executable statement of what `&param?` is FOR, and weakening it to match
// current behavior would assert that rendering nothing is correct.
@test "optional element param: omitted renders nothing, provided fills the slot" {
  @mount {
    @template &card($title, &actions?) {
      <article class="card">
        <h2>`$title`</h2>
        @if &actions {
          <div class="card__actions">`&actions`</div>
        }
      </article>
    }
    <main class="page"></main>
    .page {
      &noActions &card("A");
      &withActions &card("B") { <button>Edit</button> }
    }
  }

  // `@claim(selector: …, exists: …)` matches NO macro in stdlib — it was
  // silently dropped, so this test ran zero assertions while reporting a
  // failure only because the harness refuses a test that asserts nothing.
  // The real claim grammar is `@then <selector> should <claim>`.
  @then &noActions .card__actions should not_exist
  @then &withActions .card__actions should exist
  @then &withActions .card__actions should have_text "Edit"
}

// The PARSE half of BUG-165, isolated so it is provable TODAY.
//
// The test above cannot demonstrate it: it fails on the `@if` runtime
// contract (BUG-362) long before its parse coverage means anything, and a
// permanently-red test proves nothing about the fix it was written for. This
// one renders the slot unconditionally — the shape stdlib/macros/template.st
// now documents — so `&actions?` parsing is asserted on its own.
@test "optional element param: the header parses and a provided slot fills" {
  @mount {
    @template &pcard($title, &actions?) {
      <article class="pcard">
        <h2>`$title`</h2>
        <div class="pcard__actions">`&actions`</div>
      </article>
    }
    <main class="ppage"></main>
    .ppage {
      &pWith &pcard("B") { <button class="pbtn">Edit</button> }
    }
  }

  // The optional-param header parsed: the template registered and rendered.
  @then &pWith .pcard__actions should exist
}

// stdlib/macros/template.st doc examples — compile verbatim, both element AND
// value optional-param forms, in the SAME file (multiple &footer?/&actions?
// declarations must not collide via template-registry name reuse).
//
// STATUS — this test's whole point is "the docs do not lie", so it must track
// what the docs SAY. They no longer show `@if` inside a template: the examples
// were corrected to render the slot unconditionally once `@if` was found to be
// unimplemented (BUG-362). This copy still carries the OLD `@if` shape, so it
// is currently testing a document that no longer exists.
//
// It is kept verbatim-red rather than rewritten because the CURRENT doc shape
// is already covered by the companion test below (added with the split), and
// because rewriting this one would erase the record of what the docs used to
// promise. When BUG-362 lands and `@if` works, the docs go back to the
// conditional shape and this test becomes the right gate again unchanged.
@test "template.st doc examples compile: modal(&footer?) and card(&actions?)" {
  @mount {
    @template &probe-modal($title, &content, &footer?) {
      <div class="modal">
        <h2>`$title`</h2>
        <div class="modal__body">`&content`</div>
        @if &footer {
          <div class="modal__footer">`&footer`</div>
        }
      </div>
    }
    @template &probe-greeting($name, $title?) {
      <div class="greeting">
        @if $title {
          <span class="title">`$title`</span>
        }
        <span class="name">`$name`</span>
      </div>
    }
    <main class="page"></main>
    .page {
      &m1 &probe-modal("Confirm", <p>Are you sure?</p>);
      &m2 &probe-modal("Confirm") { <p>Are you sure?</p> <button>OK</button> }
      &g1 &probe-greeting("Alice");
      &g2 &probe-greeting("Alice", "Dr.");
    }
  }

  @then &m1 .modal__footer should not_exist
  @then &m2 .modal__footer should exist
  @then &g1 .title should not_exist
  @then &g2 .title should exist
  @then &g2 .title should have_text "Dr."
}

// The doc examples AS THEY ARE WRITTEN TODAY — unconditional slots, which is
// what stdlib/macros/template.st now shows. This is the live "the docs do not
// lie" gate; the verbatim-`@if` copy above becomes it again once BUG-362 lands.
@test "template.st doc examples compile: the shape the docs show today" {
  @mount {
    @template &doc-modal($title, &content, &footer?) {
      <div class="dmodal">
        <h2>`$title`</h2>
        <div class="dmodal__body">`&content`</div>
        <div class="dmodal__footer">`&footer`</div>
      </div>
    }
    @template &doc-greeting($name, $title?) {
      <div class="dgreeting">
        <span class="dtitle">`$title`</span>
        <span class="dname">`$name`</span>
      </div>
    }
    <main class="dpage"></main>
    .dpage {
      &d1 &doc-modal("Confirm", <p>Are you sure?</p>);
      &d2 &doc-greeting("Alice", "Dr.");
    }
  }

  // Both optional-param FORMS (element `&footer?` and value `$title?`) parse
  // and register in one file without colliding — the registry-reuse half of
  // this fixture's purpose, which does not depend on `@if`.
  @then &d1 .dmodal__body should exist
  @then &d2 .dtitle should have_text "Dr."
  @then &d2 .dname should have_text "Alice"
}
