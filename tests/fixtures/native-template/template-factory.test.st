// BUG-105 regression: a @template factory defined + invoked inside @mount must
// compile (no false E0402) AND run (the `$label` hole must not break the
// fixture-html template literal — see .fixture_html accessor). Native proof that
// the Spacetime .test.st harness can test template factories.
@test "template factory mounts in @mount" {
  @mount {
    @template &card($label) { <div class="card">`$label`</div> }
    // The host element has to EXIST for the invocation to render into it —
    // `.host { &card(…); }` is a rule targeting an element, not a way to
    // create one. Without this the fixture asserted against a selector that
    // matched nothing.
    <div class="host"></div>
    .host { &card("Hello"); }
  }
  // `exists` is not a claim verb — `@assert exists .host .card` compiled to a
  // JS reference to an undefined identifier ("exists is not defined"), so the
  // test failed on its own harness rather than on the template. The existence
  // claim is `should exist`.
  @then .host .card should exist
}
