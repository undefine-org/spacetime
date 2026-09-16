// GH-20: standard CSS @font-face at top level.
// Spacetime's real macro is `@font("Family") { ... }`, NOT `@font-face { ... }`.
// Issue: the standard spelling silently vanishes, zero diagnostics.
@font-face {
  font-family: "Test Font";
  font-style: normal;
  font-weight: 400;
  font-display: swap;
  src: url("/assets/fonts/test.woff2") format("woff2");
}

body { font-family: "Test Font", sans-serif; }

<main class="page">
  <h1>Hello</h1>
</main>
