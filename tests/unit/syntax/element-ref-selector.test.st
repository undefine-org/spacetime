/// A selector-form element ref (`&name sel;`) must register its named element
/// at runtime so the name resolves through `ST.ref` → `window.__stRefs`.
///
/// Regression gate: the JS emit block of `element-ref-impl` was being silently
/// dropped because `--st-%name-width` (CSS-var tails inside the JS emit body)
/// tokenized as a single unknown param `%name-width`, so the whole block
/// resolved to zero statements and `window.__stRefs[name]` was never written.
/// `&box .num;` compiled clean but registered nothing.

@test "a selector-form &name ref registers and resolves at runtime" {
  @mount {
    <div class="host">
      <div class="num">01</div>
    </div>

    &box .num;
  }

  // Reads through ST.ref("box") -> window.__stRefs['box']: null (unregistered)
  // when the writer was dropped, so this assertion fails RED for the real cause.
  @then &box should exist
}

@test "a selector-form &name ref scopes a descendant assertion" {
  @mount {
    <div class="host">
      <div class="num">01</div>
    </div>

    &panel .host;
  }

  // &panel resolves via ST.ref, then .num is queried within the named element.
  @then &panel .num should have_text "01"
}

// Over-widening guard: a plain CSS selector subject must keep working unchanged.
@test "a plain selector subject still resolves" {
  @mount {
    <div class="host">
      <div class="plain">ok</div>
    </div>
  }

  @then .plain should have_text "ok"
}
